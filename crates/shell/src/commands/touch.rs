use std::{
    ffi::OsString,
    fs::{self, OpenOptions},
    path::{Path, PathBuf},
};

use chrono::{DateTime, Duration, Local, NaiveDateTime, TimeZone, Timelike};
use deno_task_shell::{ExecuteResult, ShellCommand, ShellCommandContext};
use filetime::{set_file_times, set_symlink_file_times, FileTime};
use futures::future::LocalBoxFuture;
use miette::{miette, IntoDiagnostic, Result};
use uu_touch::{options, uu_app as uu_touch};

static ARG_FILES: &str = "files";

pub struct TouchCommand;

impl ShellCommand for TouchCommand {
    fn execute(&self, mut context: ShellCommandContext) -> LocalBoxFuture<'static, ExecuteResult> {
        Box::pin(futures::future::ready(match execute_touch(&mut context) {
            Ok(_) => ExecuteResult::from_exit_code(0),
            Err(e) => {
                let _ = context.stderr.write_all(format!("{:?}", e).as_bytes());
                ExecuteResult::from_exit_code(1)
            }
        }))
    }
}

fn execute_touch(context: &mut ShellCommandContext) -> Result<()> {
    let matches = uu_touch()
        .override_usage("touch [OPTION]...")
        .no_binary_name(true)
        .try_get_matches_from(&context.args)
        .into_diagnostic()?;

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
            return Err(miette!(
                "missing file operand\nTry 'touch --help' for more information."
            ))
        }
    };

    let (mut atime, mut mtime) = match (
        matches.get_one::<OsString>(options::sources::REFERENCE),
        matches.get_one::<String>(options::sources::DATE),
    ) {
        (Some(reference), Some(date)) => {
            let reference_path = PathBuf::from(reference);
            let reference_path = if reference_path.is_absolute() {
                reference_path
            } else {
                context.state.cwd().join(reference_path)
            };
            let (atime, mtime) = stat(&reference_path, !matches.get_flag(options::NO_DEREF))?;
            let atime = filetime_to_datetime(&atime)
                .ok_or_else(|| miette!("Could not process the reference access time"))?;
            let mtime = filetime_to_datetime(&mtime)
                .ok_or_else(|| miette!("Could not process the reference modification time"))?;
            Ok((parse_date(atime, date)?, parse_date(mtime, date)?))
        }
        (Some(reference), None) => {
            let reference_path = PathBuf::from(reference);
            let reference_path = if reference_path.is_absolute() {
                reference_path
            } else {
                context.state.cwd().join(reference_path)
            };
            stat(&reference_path, !matches.get_flag(options::NO_DEREF))
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
    }
    .map_err(|e| miette!("{}", e))?;

    for filename in files {
        let pathbuf = if filename.to_str() == Some("-") {
            pathbuf_from_stdout()?
        } else {
            filename
        };

        let path = pathbuf.as_path();

        let metadata_result = if matches.get_flag(options::NO_DEREF) {
            path.symlink_metadata()
        } else {
            path.metadata()
        };

        if let Err(e) = metadata_result {
            if e.kind() != std::io::ErrorKind::NotFound {
                return Err(miette!("setting times of {}: {}", path.display(), e));
            }

            if matches.get_flag(options::NO_CREATE) {
                continue;
            }

            if matches.get_flag(options::NO_DEREF) {
                let _ = context.stderr.write_all(
                    format!(
                        "setting times of {:?}: No such file or directory",
                        path.display()
                    )
                    .as_bytes(),
                );
                continue;
            }

            OpenOptions::new().create(true).truncate(false).write(true).open(path)
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
        if path.to_string_lossy() == "-" {
            set_file_times(path, atime, mtime)
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
    let now = Local::now();
    let parsed = if s.len() == 15 && s.contains('.') {
        // Handle the specific format "202401010000.00"
        NaiveDateTime::parse_from_str(s, "%Y%m%d%H%M.%S")
            .map_err(|_| miette!("invalid date format '{}'", s))?
    } else {
        dtparse::parse(s)
            .map(|(dt, _)| dt)
            .map_err(|_| miette!("invalid date format '{}'", s))?
    };

    let local = now
        .timezone()
        .from_local_datetime(&parsed)
        .single()
        .ok_or_else(|| miette!("invalid date '{}'", s))?;

    // Handle leap seconds
    let local = if parsed.second() == 59 && s.ends_with(".60") {
        local + Duration::seconds(1)
    } else {
        local
    };

    // Check for daylight saving time issues
    if (local + Duration::hours(1) - Duration::hours(1)).hour() != local.hour() {
        return Err(miette!("invalid date '{}'", s));
    }

    Ok(datetime_to_filetime(&local))
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
                return Err(miette!("GetFinalPathNameByHandleW failed with code {ret}"))
            }
            0 => {
                return Err(miette!(
                    "GetFinalPathNameByHandleW failed with code {}",
                    // SAFETY: GetLastError is thread-safe and has no documented memory unsafety.
                    unsafe { GetLastError() }
                ));
            }
            e => e as usize,
        };

        // Don't include the null terminator
        Ok(String::from_utf16(&file_path_buffer[0..buffer_size])
            .map_err(|e| miette!("Generated path is not valid UTF-16: {e}"))?
            .into())
    }
}

fn parse_date(ref_time: DateTime<Local>, s: &str) -> Result<FileTime> {
    // Using the dtparse crate for more robust date parsing

    match dtparse::parse(s) {
        Ok((naive_dt, offset)) => {
            let dt = offset.map_or_else(
                || Local.from_local_datetime(&naive_dt).unwrap(),
                |off| DateTime::<Local>::from_naive_utc_and_offset(naive_dt, off),
            );
            Ok(datetime_to_filetime(&dt))
        }
        Err(_) => {
            // Fallback to parsing Unix timestamp if dtparse fails
            if let Some(stripped) = s.strip_prefix('@') {
                stripped
                    .parse::<i64>()
                    .map(|ts| FileTime::from_unix_time(ts, 0))
                    .map_err(|_| miette!("Unable to parse date: {s}"))
            } else {
                // Use ref_time as a base for relative date parsing
                parse_datetime::parse_datetime_at_date(ref_time, s)
                    .map(|dt| datetime_to_filetime(&dt))
                    .map_err(|_| miette!("Unable to parse date: {s}"))
            }
        }
    }
}

fn datetime_to_filetime<T: TimeZone>(dt: &DateTime<T>) -> FileTime {
    FileTime::from_unix_time(dt.timestamp(), dt.timestamp_subsec_nanos())
}
