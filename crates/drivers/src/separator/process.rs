//! How the separator's program is found, started and stopped on each system (LH-NF-17).
//!
//! ```text
//!            find          start                              stop
//! Unix       mode bits     nice, own process group,           SIGTERM, then SIGKILL to
//!                          SIGKILL when the starter ends      the group
//! Windows    .exe/.cmd     below-normal priority, own group   taskkill /T /F (the tree)
//! ```
//!
//! Both end the program with Kontra, also after a crash: Unix by a signal the kernel sends when
//! the starter dies, Windows by a job object that ends its processes when its last handle
//! closes (`Leash`).

use std::path::{Path, PathBuf};
use std::process::Command;

#[cfg(all(test, unix))]
pub(super) use imp::NICENESS;
pub(super) use imp::{
    ask_to_end, detach, is_executable, kill_tree, leash, lower_priority, user_bin, Leash, PROGRAM,
};

#[cfg(unix)]
mod imp {
    use super::{Command, Path, PathBuf};
    use std::os::unix::fs::PermissionsExt;
    use std::os::unix::process::CommandExt;
    use std::process::Stdio;

    pub(in super::super) const PROGRAM: &str = "demucs";
    /// Starts a program at a lower priority (coreutils, on every Linux).
    const NICE: &str = "nice";
    /// Niceness of the separator: below the UI and the audio, above idle jobs.
    pub(in super::super) const NICENESS: &str = "10";
    const EXECUTE_BITS: u32 = 0o111;

    pub(in super::super) fn is_executable(path: &Path) -> bool {
        std::fs::metadata(path)
            .is_ok_and(|m| m.is_file() && m.permissions().mode() & EXECUTE_BITS != 0)
    }

    /// Where `pipx` puts programs; a desktop session often lacks it in `PATH`.
    pub(in super::super) fn user_bin() -> Option<PathBuf> {
        std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/bin"))
    }

    /// `command` through `nice`, if `nice` is there.
    pub(in super::super) fn lower_priority(command: Vec<String>) -> Vec<String> {
        let Some(nice) = ["/usr/bin", "/bin"]
            .iter()
            .map(|dir| Path::new(dir).join(NICE))
            .find(|p| is_executable(p))
        else {
            return command;
        };
        let mut niced = vec![
            nice.to_string_lossy().into_owned(),
            "-n".into(),
            NICENESS.into(),
        ];
        niced.extend(command);
        niced
    }

    /// Its own process group: cancelling also stops what a wrapper (conda, a script) started.
    /// And it ends with its starter.
    pub(in super::super) fn detach(command: &mut Command) {
        orphan::end_with_parent(command);
        command.process_group(0);
    }

    /// Nothing to hold on Unix: `detach` already ties the program to its starter.
    #[derive(Debug)]
    pub(in super::super) struct Leash;

    pub(in super::super) fn leash(_child: &std::process::Child) -> Option<Leash> {
        Some(Leash)
    }

    /// Asks the whole process group of `pid` to end.
    pub(in super::super) fn ask_to_end(pid: u32) {
        signal_group(pid, "-TERM");
    }

    /// Kills what is left of the process group of `pid`.
    pub(in super::super) fn kill_tree(pid: u32) {
        signal_group(pid, "-KILL");
    }

    fn signal_group(pid: u32, signal: &str) {
        let group = format!("-{pid}");
        let _ = Command::new("kill")
            .args([signal, "--", &group])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }

    /// Demucs must not outlive Kontra: in its own process group it misses the terminal's
    /// hang-up, and after a crash it would keep every core busy for minutes.
    #[allow(unsafe_code)]
    mod orphan {
        use std::os::unix::process::CommandExt;
        use std::process::Command;

        /// The child gets SIGKILL when the thread that started it ends (in Kontra the UI
        /// thread, so: when Kontra ends, also by a crash).
        pub(super) fn end_with_parent(command: &mut Command) {
            let parent = std::process::id();
            // SAFETY: runs in the forked child before exec; prctl and getppid are
            // async-signal-safe and the closure neither allocates nor takes locks.
            unsafe {
                command.pre_exec(move || {
                    if libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGKILL) != 0 {
                        return Err(std::io::Error::last_os_error());
                    }
                    // Kontra died between fork and prctl: the signal will not come.
                    if u32::try_from(libc::getppid()) != Ok(parent) {
                        libc::_exit(1);
                    }
                    Ok(())
                });
            }
        }
    }
}

#[cfg(windows)]
mod imp {
    use super::{Command, Path, PathBuf};
    use std::os::windows::process::CommandExt;
    use std::process::Stdio;

    pub(in super::super) const PROGRAM: &str = "demucs.exe";
    /// Process creation flags (Win32 `CreateProcess`): a group of its own, below the UI and
    /// the audio, and no console window popping up.
    const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
    const BELOW_NORMAL_PRIORITY_CLASS: u32 = 0x0000_4000;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    /// What Windows starts by name.
    const EXECUTABLE: [&str; 4] = ["exe", "cmd", "bat", "com"];

    /// Windows has no execute bit: a file with a program's extension.
    pub(in super::super) fn is_executable(path: &Path) -> bool {
        let known = path
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| EXECUTABLE.iter().any(|x| e.eq_ignore_ascii_case(x)));
        known && path.is_file()
    }

    /// Where `pipx` puts programs.
    pub(in super::super) fn user_bin() -> Option<PathBuf> {
        std::env::var_os("USERPROFILE").map(|h| PathBuf::from(h).join(".local").join("bin"))
    }

    /// Nothing to wrap: the priority is set when the program starts (`detach`).
    pub(in super::super) fn lower_priority(command: Vec<String>) -> Vec<String> {
        command
    }

    pub(in super::super) fn detach(command: &mut Command) {
        command.creation_flags(
            CREATE_NEW_PROCESS_GROUP | BELOW_NORMAL_PRIORITY_CLASS | CREATE_NO_WINDOW,
        );
    }

    /// Ends the program and what it starts from now on when dropped, or when Kontra is gone
    /// (Windows then closes the handle): a job object with "kill on close".
    #[derive(Debug)]
    pub(in super::super) struct Leash {
        /// Held, not read: dropping it closes the job.
        _job: job::Job,
    }

    /// `None` if Windows refuses the job: the program then runs without it, as before.
    pub(in super::super) fn leash(child: &std::process::Child) -> Option<Leash> {
        job::Job::holding(child).map(|job| Leash { _job: job })
    }

    /// The Win32 calls behind `Leash`.
    #[allow(unsafe_code)]
    mod job {
        use std::os::windows::io::AsRawHandle;
        use std::process::Child;
        use windows_sys::Win32::Foundation::{CloseHandle, HANDLE};
        use windows_sys::Win32::System::JobObjects::{
            AssignProcessToJobObject, CreateJobObjectW, JobObjectExtendedLimitInformation,
            SetInformationJobObject, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
            JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
        };

        #[derive(Debug)]
        pub(super) struct Job(HANDLE);

        // SAFETY: a job handle is a kernel object that any thread may close.
        unsafe impl Send for Job {}

        impl Job {
            /// A job that ends its processes when closed, with `child` in it.
            pub(super) fn holding(child: &Child) -> Option<Job> {
                // SAFETY: no security attributes and no name are valid arguments; the handle
                // is checked before use and owned by `Job` from here on.
                let handle = unsafe { CreateJobObjectW(std::ptr::null(), std::ptr::null()) };
                if handle.is_null() {
                    return None;
                }
                let job = Job(handle);

                let mut limits = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
                limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
                let size = std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32;
                // SAFETY: `limits` lives through the call and `size` is its size; the job and
                // the child's process handle are open (the child is not reaped yet).
                let held = unsafe {
                    SetInformationJobObject(
                        job.0,
                        JobObjectExtendedLimitInformation,
                        std::ptr::from_ref(&limits).cast(),
                        size,
                    ) != 0
                        && AssignProcessToJobObject(job.0, child.as_raw_handle().cast()) != 0
                };
                held.then_some(job)
            }
        }

        impl Drop for Job {
            fn drop(&mut self) {
                // SAFETY: the handle is open and closed only here.
                unsafe {
                    CloseHandle(self.0);
                }
            }
        }
    }

    /// A program without a window cannot be asked: it ends at once, with what it started.
    pub(in super::super) fn ask_to_end(pid: u32) {
        kill_tree(pid);
    }

    /// Ends `pid` and everything it started.
    pub(in super::super) fn kill_tree(pid: u32) {
        let _ = Command::new("taskkill")
            .args(["/T", "/F", "/PID", &pid.to_string()])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .creation_flags(CREATE_NO_WINDOW)
            .status();
    }
}
