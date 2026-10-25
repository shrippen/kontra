//! Bass removal by an external program (LH-BIB-09, round 9): Demucs, when it is installed.
//! Kontra ships nothing of it and only starts it as a separate process.
//!
//! ```text
//! song.mp3 ─► nice -n 10 demucs --two-stems bass -n htdemucs -o <out> song.mp3
//!               │ stderr " 37%|███▋ | 12.3/33.0 [00:10<00:17, …]" ─► progress 0.37, 17 s left
//!               └─► <out>/htdemucs/song/no_bass.wav
//! ```
//!
//! It runs at a lower priority (`nice`), so playing along and the UI keep the processor first
//! on a slow machine; it only takes longer then.
//!
//! Where it is looked for: `KONTRA_DEMUCS` (a path, for an install outside `PATH`, e.g. a conda
//! environment), then `demucs` in `PATH` and in `~/.local/bin` (where `pipx` puts it; a desktop
//! session often lacks it in `PATH`). Inside a Flatpak through `flatpak-spawn --host`, if the
//! sandbox allows it.

use std::io::Read;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use thiserror::Error;

const PROGRAM: &str = "demucs";
/// Starts a program at a lower priority (coreutils, on every Linux).
const NICE: &str = "nice";
/// Niceness of the separator: below the UI and the audio, above idle jobs.
const NICENESS: &str = "10";
const OVERRIDE_ENV: &str = "KONTRA_DEMUCS";
/// The model Demucs downloads on its first run (about 80 MB) and its output folder name.
const MODEL: &str = "htdemucs";
const RESULT_FILE: &str = "no_bass.wav";
/// Progress is kept in permille.
const PERMILLE: f64 = 1000.0;
/// After the program exited, its output is read this long at most (a child it left behind may
/// keep the pipe open).
const READER_GRACE: Duration = Duration::from_secs(2);
/// Cancelling waits this long for the program to end on SIGTERM before SIGKILL.
const STOP_GRACE: Duration = Duration::from_millis(300);
const STOP_POLL: Duration = Duration::from_millis(10);
/// Last bytes of stderr kept for an error message.
const TAIL_BYTES: usize = 600;
/// Seconds left are kept +1, so 0 means "not known yet".
const UNKNOWN: u32 = 0;
const SECONDS_PER_MINUTE: u32 = 60;

#[derive(Debug, Error)]
pub enum SeparatorError {
    #[error("cannot start {program}: {source}")]
    Start {
        program: String,
        source: std::io::Error,
    },
    #[error("separation failed: {0}")]
    Failed(String),
    #[error("separation cancelled")]
    Cancelled,
}

/// An installed separator: the command that runs Demucs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Separator {
    command: Vec<String>,
}

/// Finds Demucs; `None` when it is not installed (the UI explains how to install it).
pub fn find_separator() -> Option<Separator> {
    if let Some(path) = std::env::var_os(OVERRIDE_ENV) {
        let path = PathBuf::from(path);
        return is_executable(&path).then(|| Separator::with_program(&path).niced());
    }
    if in_sandbox() {
        return host_separator();
    }
    let home_bin = std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/bin"));
    let path_dirs = std::env::var_os("PATH")
        .map(|p| std::env::split_paths(&p).collect::<Vec<_>>())
        .unwrap_or_default();
    path_dirs
        .into_iter()
        .chain(home_bin)
        .map(|dir| dir.join(PROGRAM))
        .find(|p| is_executable(p))
        .map(|p| Separator::with_program(&p).niced())
}

/// Demucs on the host, seen from a Flatpak; needs the permission to talk to the Flatpak
/// portal (`--talk-name=org.freedesktop.Flatpak`), else `None`.
fn host_separator() -> Option<Separator> {
    let found = Command::new("flatpak-spawn")
        .args(["--host", "sh", "-c", "command -v demucs"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|s| s.success());
    found.then(|| Separator {
        // --watch-bus: the host program ends when Kontra (and so flatpak-spawn) is gone.
        command: vec![
            "flatpak-spawn".into(),
            "--host".into(),
            "--watch-bus".into(),
            NICE.into(),
            "-n".into(),
            NICENESS.into(),
            PROGRAM.into(),
        ],
    })
}

/// True inside a Flatpak: programs of the system are reached only through `flatpak-spawn`.
pub fn in_sandbox() -> bool {
    Path::new("/.flatpak-info").exists()
}

fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    std::fs::metadata(path).is_ok_and(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
}

impl Separator {
    /// A separator at `program` (a Demucs-compatible command line).
    pub fn with_program(program: &Path) -> Separator {
        Separator {
            command: vec![program.to_string_lossy().into_owned()],
        }
    }

    /// The same program at a lower priority, if `nice` is there.
    fn niced(self) -> Separator {
        let Some(nice) = ["/usr/bin", "/bin"]
            .iter()
            .map(|dir| Path::new(dir).join(NICE))
            .find(|p| is_executable(p))
        else {
            return self;
        };
        let mut command = vec![
            nice.to_string_lossy().into_owned(),
            "-n".into(),
            NICENESS.into(),
        ];
        command.extend(self.command);
        Separator { command }
    }

    /// Starts removing the bass from `input`; the result lands below `out_dir`.
    pub fn start(&self, input: &Path, out_dir: &Path) -> Result<SeparationJob, SeparatorError> {
        let (program, prefix) = match self.command.split_first() {
            Some((program, prefix)) => (program, prefix),
            None => {
                return Err(SeparatorError::Failed("no program".into()));
            }
        };
        // Its own process group: cancelling also stops what a wrapper (conda, a script) started.
        let mut command = Command::new(program);
        orphan::end_with_parent(&mut command);
        let mut child = command
            .process_group(0)
            .args(prefix)
            .args(["--two-stems", "bass", "-n", MODEL, "-o"])
            .arg(out_dir)
            .arg(input)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|source| SeparatorError::Start {
                program: program.clone(),
                source,
            })?;
        let progress = Arc::new(AtomicU32::new(0));
        let left = Arc::new(AtomicU32::new(UNKNOWN));
        let tail = Arc::new(Mutex::new(String::new()));
        let reader = child.stderr.take().map(|stderr| {
            let bar = Bar {
                progress: Arc::clone(&progress),
                left: Arc::clone(&left),
            };
            let tail = Arc::clone(&tail);
            std::thread::spawn(move || read_progress(stderr, &bar, &tail))
        });
        let stem = input
            .file_stem()
            .map_or_else(String::new, |s| s.to_string_lossy().into_owned());
        Ok(SeparationJob {
            child,
            progress,
            left,
            tail,
            reader,
            exited_at: None,
            result: out_dir.join(MODEL).join(stem).join(RESULT_FILE),
            cancelled: false,
        })
    }
}

/// A running separation; dropping it stops the program.
#[derive(Debug)]
pub struct SeparationJob {
    child: Child,
    progress: Arc<AtomicU32>,
    /// Seconds left + 1, `UNKNOWN` before the bar says.
    left: Arc<AtomicU32>,
    tail: Arc<Mutex<String>>,
    /// Reads the program's output until it ends; joined once the program exited, so the
    /// last progress and error lines are in.
    reader: Option<std::thread::JoinHandle<()>>,
    exited_at: Option<Instant>,
    result: PathBuf,
    cancelled: bool,
}

impl SeparationJob {
    /// 0..=1, from the program's progress bar (Demucs runs several passes; this is the current
    /// one, so it can go back).
    pub fn progress(&self) -> f64 {
        f64::from(self.progress.load(Ordering::Relaxed)) / PERMILLE
    }

    /// Time left of the current pass, as the program estimates it; `None` until it does.
    pub fn remaining(&self) -> Option<Duration> {
        let left = self.left.load(Ordering::Relaxed);
        (left != UNKNOWN).then(|| Duration::from_secs(u64::from(left - 1)))
    }

    /// `None` while it runs; the file without bass when done.
    pub fn poll(&mut self) -> Option<Result<PathBuf, SeparatorError>> {
        let status = match self.child.try_wait() {
            Ok(None) => return None,
            Ok(Some(status)) => status,
            Err(e) => return Some(Err(SeparatorError::Failed(e.to_string()))),
        };
        // The pipe closes with the program and the reader ends after its last line; until then
        // the job counts as running (never block the caller, e.g. the UI's tick).
        let exited_at = *self.exited_at.get_or_insert_with(Instant::now);
        let reading = self.reader.as_ref().is_some_and(|r| !r.is_finished());
        if reading && exited_at.elapsed() < READER_GRACE {
            return None;
        }
        // Something the program started still holds its output: it goes with the group.
        if reading {
            self.stop_group();
        }
        if let Some(reader) = self.reader.take().filter(|r| r.is_finished()) {
            let _ = reader.join();
        }
        if self.cancelled {
            return Some(Err(SeparatorError::Cancelled));
        }
        if status.success() && self.result.is_file() {
            return Some(Ok(self.result.clone()));
        }
        let tail = self.tail.lock().map(|t| t.clone()).unwrap_or_default();
        Some(Err(SeparatorError::Failed(last_line(&tail))))
    }

    pub fn cancel(&mut self) {
        self.cancelled = true;
        self.stop();
    }

    /// Asks the program's whole process group to end (a wrapper like flatpak-spawn needs a
    /// moment to pass the signal on), then kills what is left of it, so nothing writes into
    /// its folder afterwards.
    fn stop(&mut self) {
        self.stop_group();
        let deadline = Instant::now() + STOP_GRACE;
        while Instant::now() < deadline && matches!(self.child.try_wait(), Ok(None)) {
            std::thread::sleep(STOP_POLL);
        }
        self.signal_group("-KILL");
        let _ = self.child.kill();
        let _ = self.child.wait();
    }

    fn stop_group(&self) {
        self.signal_group("-TERM");
    }

    fn signal_group(&self, signal: &str) {
        let group = format!("-{}", self.child.id());
        let _ = Command::new("kill")
            .args([signal, "--", &group])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }
}

impl Drop for SeparationJob {
    fn drop(&mut self) {
        if matches!(self.child.try_wait(), Ok(None)) {
            self.stop();
            let _ = self.child.wait();
        } else if self.reader.as_ref().is_some_and(|r| !r.is_finished()) {
            // Gone, but something it started still runs (it holds the output).
            self.stop_group();
        }
    }
}

/// Demucs must not outlive Kontra: in its own process group it misses the terminal's hang-up,
/// and after a crash it would keep every core busy for minutes.
#[allow(unsafe_code)]
mod orphan {
    use std::os::unix::process::CommandExt;
    use std::process::Command;

    /// The child gets SIGKILL when the thread that started it ends (in Kontra the UI thread,
    /// so: when Kontra ends, also by a crash).
    pub(super) fn end_with_parent(command: &mut Command) {
        let parent = std::process::id();
        // SAFETY: runs in the forked child before exec; prctl and getppid are async-signal-safe
        // and the closure neither allocates nor takes locks.
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

/// What the reader thread passes on from the progress bar.
struct Bar {
    progress: Arc<AtomicU32>,
    left: Arc<AtomicU32>,
}

/// Reads the progress bar ("…\r 37%|███▋ |…") and keeps the end of the output for errors.
fn read_progress(mut stderr: impl Read, bar: &Bar, tail: &Mutex<String>) {
    let mut buffer = [0u8; 1024];
    let mut line = String::new();
    loop {
        let n = match stderr.read(&mut buffer) {
            Ok(0) | Err(_) => return,
            Ok(n) => n,
        };
        for c in String::from_utf8_lossy(&buffer[..n]).chars() {
            if c != '\r' && c != '\n' {
                line.push(c);
                continue;
            }
            if let Some(p) = percent(&line) {
                bar.progress
                    .store((p * PERMILLE).round() as u32, Ordering::Relaxed);
                let left = seconds_left(&line).map_or(UNKNOWN, |s| s + 1);
                bar.left.store(left, Ordering::Relaxed);
            }
            if let Ok(mut t) = tail.lock() {
                t.push_str(&line);
                t.push('\n');
                let cut = t.len().saturating_sub(TAIL_BYTES);
                let cut = (cut..t.len()).find(|i| t.is_char_boundary(*i)).unwrap_or(0);
                t.drain(..cut);
            }
            line.clear();
        }
    }
}

/// The share from a progress bar line, " 37%|███▋ |" → 0.37.
fn percent(line: &str) -> Option<f64> {
    let (before, _) = line.split_once("%|")?;
    let digits = before
        .rsplit(|c: char| !c.is_ascii_digit() && c != '.')
        .next()?;
    let value: f64 = digits.parse().ok()?;
    (0.0..=100.0).contains(&value).then_some(value / 100.0)
}

/// Seconds left from a progress bar line, "[00:10<01:17, …]" → 77; "<?" → `None`.
fn seconds_left(line: &str) -> Option<u32> {
    let (_, after) = line.rsplit_once('<')?;
    let field = after.split([',', ']']).next()?.trim();
    field.split(':').try_fold(0u32, |total, part| {
        let value: u32 = part.parse().ok()?;
        total.checked_mul(SECONDS_PER_MINUTE)?.checked_add(value)
    })
}

/// The last line with text, for an error message; without Python's exception name and the
/// closing full stop ("demucs.api.LoadAudioError: Could not read." → "Could not read").
fn last_line(text: &str) -> String {
    let line = text
        .lines()
        .rev()
        .map(str::trim)
        .find(|l| !l.is_empty())
        .unwrap_or("");
    let message = match line.split_once(": ") {
        Some((name, rest)) if is_exception(name) => rest,
        _ => line,
    };
    message.trim_end_matches('.').to_string()
}

/// "ValueError", "demucs.api.LoadAudioError": a dotted name ending in Error or Exception.
fn is_exception(name: &str) -> bool {
    let plain = name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '_');
    plain && (name.ends_with("Error") || name.ends_with("Exception"))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// One test at a time writes and starts a script: a child forked by the other test can hold
    /// the file still open for writing, and starting it fails with "Text file busy" (ETXTBSY).
    static SCRIPTS: std::sync::Mutex<()> = std::sync::Mutex::new(());

    fn scripts() -> std::sync::MutexGuard<'static, ()> {
        SCRIPTS
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    #[test]
    fn reads_the_progress_bar() {
        assert_eq!(
            percent(" 37%|███▋      | 12.3/33.0 [00:10<00:17]"),
            Some(0.37)
        );
        assert_eq!(percent("100%|██████████| 33.0/33.0"), Some(1.0));
        assert_eq!(percent("Separated tracks will be stored in …"), None);
    }

    #[test]
    fn keeps_demucs_own_words() {
        let tail = "Traceback …\ndemucs.api.LoadAudioError: When trying to load using ffmpeg, got \
                    the following error: FFmpeg could not read the file.\n";
        assert_eq!(
            last_line(tail),
            "When trying to load using ffmpeg, got the following error: FFmpeg could not read the file"
        );
        assert_eq!(last_line("Note: a hint\n\n"), "Note: a hint");
        assert_eq!(last_line(""), "");
    }

    #[test]
    fn reads_the_time_left() {
        let line = " 25%|██▌ | 5.85/23.4 [00:30<01:32,  5.24s/seconds]";
        assert_eq!(seconds_left(line), Some(92));
        assert_eq!(
            seconds_left(" 5%| | 1/20 [00:01<1:02:03, 2s/it]"),
            Some(3723)
        );
        assert_eq!(seconds_left("  0%| | 0.0/23.4 [00:00<?, ?seconds/s]"), None);
        assert_eq!(seconds_left("no bar"), None);
    }

    /// Demucs takes every processor it gets; playing along must not stutter meanwhile.
    #[test]
    fn runs_below_the_ui() {
        let _scripts = scripts();
        let dir =
            std::env::temp_dir().join(format!("kontra-separator-nice-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let script = dir.join("demucs");
        std::fs::write(&script, "#!/bin/sh\necho \"$(nice)\" >&2\nexit 1\n").unwrap();
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
        let song = dir.join("Song.wav");
        std::fs::write(&song, b"RIFF").unwrap();

        let mut job = Separator::with_program(&script)
            .niced()
            .start(&song, &dir.join("out"))
            .unwrap();
        let niceness = match wait(&mut job) {
            Err(SeparatorError::Failed(line)) => line.parse::<i32>().unwrap(),
            other => panic!("{other:?}"),
        };
        // Ten below the caller (a test may itself run under `nice`), at most 19.
        let own = Command::new("nice").output().unwrap();
        let own: i32 = String::from_utf8_lossy(&own.stdout).trim().parse().unwrap();
        assert_eq!(niceness, (own + 10).min(19));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn niced_starts_through_nice() {
        let command = Separator::with_program(Path::new("/opt/demucs"))
            .niced()
            .command;
        assert!(command[0].ends_with("/nice"), "{command:?}");
        assert_eq!(command[1..], ["-n", NICENESS, "/opt/demucs"]);
    }

    /// Second reader #64: a grandchild that ignores SIGTERM outlived the cancel and could write
    /// into the work folder after it was cleared.
    #[test]
    fn cancel_kills_the_whole_group() {
        let _scripts = scripts();
        let dir =
            std::env::temp_dir().join(format!("kontra-separator-group-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let alive = dir.join("alive");
        let script = dir.join("demucs");
        let body = format!(
            "#!/bin/sh\nsh -c 'trap \"\" TERM; while true; do touch {}; sleep 0.02; done' &\nwait\n",
            alive.display()
        );
        std::fs::write(&script, body).unwrap();
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
        let song = dir.join("Song.wav");
        std::fs::write(&song, b"RIFF").unwrap();

        let mut job = Separator::with_program(&script)
            .start(&song, &dir.join("out"))
            .unwrap();
        std::thread::sleep(Duration::from_millis(200));
        job.cancel();
        drop(job);
        let _ = std::fs::remove_file(&alive);
        std::thread::sleep(Duration::from_millis(200));
        assert!(!alive.exists(), "the grandchild still runs");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Second reader #64: Demucs outlived a crashed Kontra and kept every core busy. It ends with
    /// the thread that started it (in Kontra the UI thread).
    #[test]
    fn ends_with_its_starter() {
        let _scripts = scripts();
        let dir =
            std::env::temp_dir().join(format!("kontra-separator-orphan-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let pid_file = dir.join("pid");
        let script = dir.join("demucs");
        let body = format!(
            "#!/bin/sh\necho $$ > {}\nexec sleep 30\n",
            pid_file.display()
        );
        std::fs::write(&script, body).unwrap();
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
        let song = dir.join("Song.wav");
        std::fs::write(&song, b"RIFF").unwrap();

        let (script2, song2, out) = (script.clone(), song.clone(), dir.join("out"));
        let pid_file2 = pid_file.clone();
        std::thread::spawn(move || {
            let job = Separator::with_program(&script2)
                .start(&song2, &out)
                .unwrap();
            // As if Kontra died: nothing stops it. The starter ends once the program runs; a
            // fixed 200 ms was too short on a busy machine (the shell had not written its pid).
            std::mem::forget(job);
            let deadline = Instant::now() + Duration::from_secs(5);
            // `echo $$ > pid` creates the file before it writes: wait for the number itself.
            let written =
                || std::fs::read_to_string(&pid_file2).is_ok_and(|s| !s.trim().is_empty());
            while !written() && Instant::now() < deadline {
                std::thread::sleep(STOP_POLL);
            }
        })
        .join()
        .unwrap();
        let pid = std::fs::read_to_string(&pid_file).unwrap();
        assert!(!pid.trim().is_empty(), "the program never wrote its pid");
        let running = || {
            let stat = std::fs::read_to_string(format!("/proc/{}/stat", pid.trim()));
            // A killed child stays a zombie (state Z) until reaped.
            stat.is_ok_and(|s| !s.contains(") Z "))
        };
        let deadline = Instant::now() + Duration::from_secs(2);
        while running() && Instant::now() < deadline {
            std::thread::sleep(STOP_POLL);
        }
        assert!(!running(), "Demucs outlived its starter");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A stand-in for Demucs: prints a progress bar and writes the result file.
    fn fake(dir: &Path, exit: u8) -> PathBuf {
        let script = dir.join("demucs");
        let body = format!(
            "#!/bin/sh\n\
             out=\"$6\"; name=$(basename \"$7\"); name=\"${{name%.*}}\"\n\
             printf ' 50%%|█████     | 1/2\\r' >&2\n\
             mkdir -p \"$out/htdemucs/$name\" && cp \"$7\" \"$out/htdemucs/$name/no_bass.wav\"\n\
             printf '100%%|██████████| 2/2\\nbroken model\\n' >&2\n\
             exit {exit}\n"
        );
        std::fs::write(&script, body).unwrap();
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
        script
    }

    fn wait(job: &mut SeparationJob) -> Result<PathBuf, SeparatorError> {
        loop {
            if let Some(r) = job.poll() {
                return r;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
    }

    #[test]
    fn runs_the_program_and_finds_its_result() {
        let _scripts = scripts();
        let dir = std::env::temp_dir().join(format!("kontra-separator-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let song = dir.join("Song.wav");
        std::fs::write(&song, b"RIFF").unwrap();

        let mut job = Separator::with_program(&fake(&dir, 0))
            .start(&song, &dir.join("out"))
            .unwrap();
        let result = wait(&mut job).unwrap();
        assert_eq!(result, dir.join("out/htdemucs/Song/no_bass.wav"));
        assert!((job.progress() - 1.0).abs() < 1e-9);

        let mut failing = Separator::with_program(&fake(&dir, 1))
            .start(&song, &dir.join("out2"))
            .unwrap();
        match wait(&mut failing) {
            Err(SeparatorError::Failed(m)) => assert_eq!(m, "broken model"),
            other => panic!("{other:?}"),
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn cancel_stops_what_a_wrapper_started() {
        let _scripts = scripts();
        let dir =
            std::env::temp_dir().join(format!("kontra-separator-cancel-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        // A wrapper (like `conda run`) whose child keeps the output pipe open.
        let script = dir.join("demucs");
        std::fs::write(&script, "#!/bin/sh\nsleep 30 &\nwait\n").unwrap();
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
        let song = dir.join("Song.wav");
        std::fs::write(&song, b"RIFF").unwrap();

        let mut job = Separator::with_program(&script)
            .start(&song, &dir.join("out"))
            .unwrap();
        std::thread::sleep(Duration::from_millis(100));
        job.cancel();
        let started = Instant::now();
        let result = wait(&mut job);
        assert!(
            matches!(result, Err(SeparatorError::Cancelled)),
            "{result:?}"
        );
        assert!(
            started.elapsed() < READER_GRACE,
            "the child went with the wrapper"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
