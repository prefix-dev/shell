use std::{ffi::OsString, fs::{self, File}, path::{Path, PathBuf}};

use deno_task_shell::{ExecuteResult, ShellCommand, ShellCommandContext};
use futures::future::LocalBoxFuture;
use uu_touch::{uu_app as uu_touch, options};
use miette::{miette, Result, IntoDiagnostic};
use filetime::{set_file_times, set_symlink_file_times, FileTime};

static ARG_FILES: &str = "files";

pub struct TouchCommand;

impl ShellCommand for TouchCommand {
    fn execute(&self, mut context: ShellCommandContext) -> LocalBoxFuture<'static, ExecuteResult> {
        Box::pin(futures::future::ready(match execute_touch(&mut context) {
            Ok(_) => ExecuteResult::from_exit_code(0),
            Err(e) => {
                let _ = context.stderr.write_all(format!("{:#}", e).as_bytes());
                ExecuteResult::from_exit_code(1)
            },
        }))
    }
}

fn execute_touch(context: &mut ShellCommandContext) -> Result<()> {
    let matches = uu_touch().try_get_matches_from(context.args.clone()).into_diagnostic()?;

    let files = match matches.get_many::<OsString>(ARG_FILES) {
        Some(files) => files.map(|file| {
            let path = PathBuf::from(file);
            if path.is_absolute() {
                path
            } else {
                context.state.cwd().join(path)
            }
        }),
        None => {
            return Err(miette!("missing file operand\nTry 'touch --help' for more information."));
        }
    };

    let (atime, mtime) = match (
        matches.get_one::<OsString>(options::sources::REFERENCE),
        matches.get_one::<String>(options::sources::DATE),
    ) {
        (Some(reference), Some(date)) => {
            let (atime, mtime) = stat(Path::new(&reference), !matches.get_flag(options::NO_DEREF))?;
            let atime = filetime_to_datetime(&atime).ok_or_else(|| {
                miette!("Could not process the reference access time")
            })?;
            let mtime = filetime_to_datetime(&mtime).ok_or_else(|| {
                miette!("Could not process the reference modification time")
            })?;
            Ok((parse_date(atime, date)?, parse_date(mtime, date)?))
        }
        (Some(reference), None) => {
            stat(Path::new(&reference), !matches.get_flag(options::NO_DEREF))
        }
        (None, Some(date)) => {
            let timestamp = parse_date(Local::now(), date)?;
            Ok((timestamp, timestamp))
        }
        (None, None) => {
            let timestamp = if let Some(ts) = matches.get_one::<String>(options::sources::TIMESTAMP)
            {
                parse_timestamp(ts)?
            } else {
                datetime_to_filetime(&Local::now())
            };
            Ok((timestamp, timestamp))
        }
    }.map_err(|e| miette!("{}", e))?;

    for filename in files {
        // FIXME: find a way to avoid having to clone the path
        let pathbuf = if filename.to_string_lossy() == "-" {
            pathbuf_from_stdout()
                .into_diagnostic()?
        } else {
            PathBuf::from(filename)
        };

        let path = pathbuf.as_path();

        let metadata_result = if matches.get_flag(options::NO_DEREF) {
            path.symlink_metadata()
        } else {
            path.metadata()
        };

        if let Err(e) = metadata_result {
            if e.kind() != std::io::ErrorKind::NotFound {
                return Err(miette!("setting times of {}: {}", filename.to_string_lossy(), e));
            }

            if matches.get_flag(options::NO_CREATE) {
                continue;
            }

            if matches.get_flag(options::NO_DEREF) {
                context.stderr.write_all(format!("setting times of {}: No such file or directory", filename.to_string_lossy()).as_bytes())
                    .map_err(|e| miette!("Failed to write to stderr: {}", e))?;
                continue;
            }

            File::create(path)
                .into_diagnostic()
                .map_err(|e| miette!("cannot touch {}: {}", path.display(), e))?;

            // Minor optimization: if no reference time was specified, we're done.
            if !matches.contains_id(options::SOURCES) {
                continue;
            }
        }

        if matches.get_flag(options::ACCESS)
            || matches.get_flag(options::MODIFICATION)
            || matches.contains_id(options::TIME)
        {
            let st = stat(path, !matches.get_flag(options::NO_DEREF))?;
            let time = matches
                .get_one::<String>(options::TIME)
                .map(|s| s.as_str())
                .unwrap_or("");

            if !(matches.get_flag(options::ACCESS)
                || time.contains(&"access".to_owned())
                || time.contains(&"atime".to_owned())
                || time.contains(&"use".to_owned()))
            {
                atime = st.0;
            }

            if !(matches.get_flag(options::MODIFICATION)
                || time.contains(&"modify".to_owned())
                || time.contains(&"mtime".to_owned()))
            {
                mtime = st.1;
            }
        }

        // sets the file access and modification times for a file or a symbolic link.
        // The filename, access time (atime), and modification time (mtime) are provided as inputs.

        // If the filename is not "-", indicating a special case for touch -h -,
        // the code checks if the NO_DEREF flag is set, which means the user wants to
        // set the times for a symbolic link itself, rather than the file it points to.
        if filename.to_string_lossy() == "-" {
            filetime::set_file_times(path, atime, mtime)
        } else if matches.get_flag(options::NO_DEREF) {
            set_symlink_file_times(path, atime, mtime)
        } else {
            set_file_times(path, atime, mtime)
        }
        .map_err(|e| miette!("setting times of {}: {}", path.display(), e))?;
        }

    Ok(())
}

fn stat(path: &Path, follow: bool) -> Result<(FileTime, FileTime)> {
    let metadata = if follow {
        fs::metadata(path).or_else(|_| fs::symlink_metadata(path))
    } else {
        fs::symlink_metadata(path)
    }
    .map_err(|e| miette!("failed to get attributes of {}: {}", path.display(), e))?;

    Ok((
        FileTime::from_last_access_time(&metadata),
        FileTime::from_last_modification_time(&metadata),
    ))
}

fn filetime_to_datetime(ft: &FileTime) -> Option<DateTime<Local>> {
    Some(DateTime::from_timestamp(ft.unix_seconds(), ft.nanoseconds())?.into())
}

fn parse_timestamp(s: &str) -> Result<FileTime> {
    let current_year = || Local::now().year();

    let (format, ts) = match s.chars().count() {
        15 => (YYYYMMDDHHMM_DOT_SS, s.to_owned()),
        12 => (YYYYMMDDHHMM, s.to_owned()),
        // If we don't add "20", we have insufficient information to parse
        13 => (YYYYMMDDHHMM_DOT_SS, format!("20{}", s)),
        10 => (YYYYMMDDHHMM, format!("20{}", s)),
        11 => (YYYYMMDDHHMM_DOT_SS, format!("{}{}", current_year(), s)),
        8 => (YYYYMMDDHHMM, format!("{}{}", current_year(), s)),
        _ => {
            return Err(miette!("invalid date format '{}'", s));
        }
    };

    let local = NaiveDateTime::parse_from_str(&ts, format)
        .map_err(|_| miette!("invalid date ts format '{}'", s))?;
    let mut local = match chrono::Local.from_local_datetime(&local) {
        LocalResult::Single(dt) => dt,
        _ => {
            return Err(miette!(
                "invalid date ts format '{}'", s,
            ))
        }
    };

    // Chrono caps seconds at 59, but 60 is valid. It might be a leap second
    // or wrap to the next minute. But that doesn't really matter, because we
    // only care about the timestamp anyway.
    // Tested in gnu/tests/touch/60-seconds
    if local.second() == 59 && ts.ends_with(".60") {
        local += Duration::try_seconds(1).unwrap();
    }

    // Due to daylight saving time switch, local time can jump from 1:59 AM to
    // 3:00 AM, in which case any time between 2:00 AM and 2:59 AM is not
    // valid. If we are within this jump, chrono takes the offset from before
    // the jump. If we then jump forward an hour, we get the new corrected
    // offset. Jumping back will then now correctly take the jump into account.
    let local2 = local + Duration::try_hours(1).unwrap() - Duration::try_hours(1).unwrap();
    if local.hour() != local2.hour() {
        return Err(miette!(
            "invalid date format '{}'", s,
        ));
    }

    let local = FileTime::from_unix_time(local.timestamp(), local.timestamp_subsec_nanos());
    Ok(local)
}

// TODO: this may be a good candidate to put in fsext.rs
/// Returns a PathBuf to stdout.
///
/// On Windows, uses GetFinalPathNameByHandleW to attempt to get the path
/// from the stdout handle.
fn pathbuf_from_stdout() -> Result<PathBuf> {
    #[cfg(all(unix, not(target_os = "android")))]
    {
        Ok(PathBuf::from("/dev/stdout"))
    }
    #[cfg(target_os = "android")]
    {
        Ok(PathBuf::from("/proc/self/fd/1"))
    }
    #[cfg(windows)]
    {
        use std::os::windows::prelude::AsRawHandle;
        use windows_sys::Win32::Foundation::{
            GetLastError, ERROR_INVALID_PARAMETER, ERROR_NOT_ENOUGH_MEMORY, ERROR_PATH_NOT_FOUND,
            HANDLE, MAX_PATH,
        };
        use windows_sys::Win32::Storage::FileSystem::{
            GetFinalPathNameByHandleW, FILE_NAME_OPENED,
        };

        let handle = std::io::stdout().lock().as_raw_handle() as HANDLE;
        let mut file_path_buffer: [u16; MAX_PATH as usize] = [0; MAX_PATH as usize];

        // https://docs.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-getfinalpathnamebyhandlea#examples
        // SAFETY: We transmute the handle to be able to cast *mut c_void into a
        // HANDLE (i32) so rustc will let us call GetFinalPathNameByHandleW. The
        // reference example code for GetFinalPathNameByHandleW implies that
        // it is safe for us to leave lpszfilepath uninitialized, so long as
        // the buffer size is correct. We know the buffer size (MAX_PATH) at
        // compile time. MAX_PATH is a small number (260) so we can cast it
        // to a u32.
        let ret = unsafe {
            GetFinalPathNameByHandleW(
                handle,
                file_path_buffer.as_mut_ptr(),
                file_path_buffer.len() as u32,
                FILE_NAME_OPENED,
            )
        };

        let buffer_size = match ret {
            ERROR_PATH_NOT_FOUND | ERROR_NOT_ENOUGH_MEMORY | ERROR_INVALID_PARAMETER => {
                return Err(USimpleError::new(
                    1,
                    format!("GetFinalPathNameByHandleW failed with code {ret}"),
                ))
            }
            0 => {
                return Err(USimpleError::new(
                    1,
                    format!(
                        "GetFinalPathNameByHandleW failed with code {}",
                        // SAFETY: GetLastError is thread-safe and has no documented memory unsafety.
                        unsafe { GetLastError() }
                    ),
                ));
            }
            e => e as usize,
        };

        // Don't include the null terminator
        Ok(String::from_utf16(&file_path_buffer[0..buffer_size])
            .map_err(|e| USimpleError::new(1, e.to_string()))?
            .into())
    }
}
