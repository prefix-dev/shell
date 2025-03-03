use std::time::Instant;

use deno_task_shell::{ExecuteResult, ShellCommand, ShellCommandContext};
use futures::future::LocalBoxFuture;

#[cfg(unix)]
use libc::{rusage, timeval, RUSAGE_CHILDREN};

#[cfg(windows)]
use windows_sys::Win32::System::Threading::GetProcessTimes;
#[cfg(windows)]
use windows_sys::Win32::Foundation::{FILETIME, HANDLE};

pub struct TimeCommand;

impl ShellCommand for TimeCommand {
    fn execute(&self, mut context: ShellCommandContext) -> LocalBoxFuture<'static, ExecuteResult> {
        Box::pin(async move {
            match execute_time(&mut context).await {
                Ok(_) => ExecuteResult::from_exit_code(0),
                Err(exit_code) => ExecuteResult::from_exit_code(exit_code),
            }
        })
    }
}

#[cfg(unix)]
fn timeval_to_seconds(tv: timeval) -> f64 {
    tv.tv_sec as f64 + (tv.tv_usec as f64 / 1_000_000.0)
}

#[cfg(unix)]
fn get_resource_usage() -> rusage {
    let mut usage = unsafe { std::mem::zeroed::<rusage>() };
    unsafe {
        libc::getrusage(RUSAGE_CHILDREN, &mut usage);
    }
    usage
}

#[cfg(windows)]
fn filetime_to_seconds(ft: FILETIME) -> f64 {
    // Convert FILETIME to 100-nanosecond intervals, then to seconds
    let time_value = ((ft.dwHighDateTime as u64) << 32) | (ft.dwLowDateTime as u64);
    time_value as f64 / 10_000_000.0
}

#[cfg(windows)]
fn get_process_times(handle: HANDLE) -> (f64, f64) {
    // Initialize FILETIME structures
    let mut creation_time = FILETIME {
        dwLowDateTime: 0,
        dwHighDateTime: 0,
    };
    let mut exit_time = FILETIME {
        dwLowDateTime: 0,
        dwHighDateTime: 0,
    };
    let mut kernel_time = FILETIME {
        dwLowDateTime: 0,
        dwHighDateTime: 0,
    };
    let mut user_time = FILETIME {
        dwLowDateTime: 0,
        dwHighDateTime: 0,
    };

    unsafe {
        GetProcessTimes(
            handle,
            &mut creation_time,
            &mut exit_time,
            &mut kernel_time,
            &mut user_time,
        );
    }

    // Convert to seconds
    let kernel_seconds = filetime_to_seconds(kernel_time);
    let user_seconds = filetime_to_seconds(user_time);

    (user_seconds, kernel_seconds)
}

#[cfg(windows)]
fn get_current_process_handle() -> HANDLE {
    use windows_sys::Win32::System::Threading::GetCurrentProcess;
    unsafe { GetCurrentProcess() }
}

async fn execute_time(context: &mut ShellCommandContext) -> Result<(), i32> {
    if context.args.is_empty() {
        context
            .stderr
            .write_line("Usage: time COMMAND [ARGS...]")
            .ok();
        return Err(1);
    }

    let command_line = context.args.join(" ");

    #[cfg(unix)]
    let before_usage = get_resource_usage();

    #[cfg(windows)]
    let process_handle = get_current_process_handle();
    #[cfg(windows)]
    let (before_user, before_kernel) = get_process_times(process_handle);

    let start = Instant::now();

    let result = crate::execute::execute(&command_line, None, &mut context.state).await;

    let duration = start.elapsed();

    #[cfg(unix)]
    let after_usage = get_resource_usage();

    #[cfg(windows)]
    let (after_user, after_kernel) = get_process_times(process_handle);

    #[cfg(unix)]
    let user_time =
        timeval_to_seconds(after_usage.ru_utime) - timeval_to_seconds(before_usage.ru_utime);
    #[cfg(unix)]
    let sys_time =
        timeval_to_seconds(after_usage.ru_stime) - timeval_to_seconds(before_usage.ru_stime);

    #[cfg(windows)]
    let user_time = after_user - before_user;
    #[cfg(windows)]
    let sys_time = after_kernel - before_kernel;

    #[cfg(not(any(unix, windows)))]
    let user_time = 0.0;
    #[cfg(not(any(unix, windows)))]
    let sys_time = 0.0;

    let real_time = duration.as_secs_f64();
    let cpu_time = user_time + sys_time;
    let cpu_usage = if real_time > 0.0 {
        (cpu_time / real_time) * 100.0
    } else {
        0.0
    };

    context
        .stderr
        .write_line(&format!("\nreal\t{:.3}s", real_time))
        .ok();
    context
        .stderr
        .write_line(&format!("user\t{:.3}s", user_time))
        .ok();
    context
        .stderr
        .write_line(&format!("sys\t{:.3}s", sys_time))
        .ok();
    context
        .stderr
        .write_line(&format!("cpu\t{:.1}%", cpu_usage))
        .ok();

    match result {
        Ok(execute_result) => match execute_result.exit_code() {
            0 => Ok(()),
            code => Err(code),
        },
        Err(err) => {
            context.stderr.write_line(&format!("Error: {}", err)).ok();
            Err(1)
        }
    }
}
