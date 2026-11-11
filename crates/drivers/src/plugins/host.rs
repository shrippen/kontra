//! The plugin thread: one thread per process that owns every `LoadedPlugin`.
//!
//! CLAP and VST3 want creation, state and (later) windows and timers on one main thread, and a
//! plugin instance is not `Send`. So callers never touch an instance: they send commands and
//! get a `PluginHandle` back.
//!
//! ```text
//!  any thread                         kontra-plugins (owns the LoadedPlugins)
//!  PluginHost::load ── Load ────────► open ─ state ─ activate ─ params ─► reply
//!  PendingLoad::poll ◄───── reply ────────────────────────────────────────┘
//!  PluginHandle::set_param ── SetParam ─►  instance
//!  PluginHandle::take_changes ◄─ inbox ◄─  instance.take_changes (every tick)
//!  PluginHandle::open_editor ── OpenEditor ─► editor in a frame; every tick: idle (timers,
//!                                             window events); EditorShown says if it is open
//!  drop PluginHandle ── Close ──────► instance dropped
//!
//!  the PluginProcessor of a load goes on to the audio thread, as it did before
//! ```
//!
//! A processor must be dropped before its handle: the handle's drop closes the instance.

use super::editor::{EditorShown, FrameMaker};
use super::{coalesce, LoadedPlugin, ParamInfo, PluginError, PluginFormat, PluginProcessor};
use std::collections::HashMap;
use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex, OnceLock};
use std::thread;
use std::time::Duration;

/// How long the loop waits for a command before it looks after the instances: their own
/// parameter changes, timers, descriptors and editor windows.
const TICK: Duration = Duration::from_millis(10);

/// Separates the file and the plugin id in a crash marker.
const MARKER_SEPARATOR: char = '\n';

/// What a load needs: the plugin, its saved state and parameters, and how it will run.
#[derive(Debug, Clone)]
pub struct PluginSetup {
    pub format: PluginFormat,
    pub path: PathBuf,
    pub id: String,
    /// The plugin's own saved state; empty: none.
    pub state: Vec<u8>,
    /// Parameter values set after the activation, (id, value).
    pub params: Vec<(u32, f64)>,
    pub sample_rate: f64,
    pub max_block: u32,
    /// Written before the plugin's code runs and removed after: a marker left behind means the
    /// program died in there (`crashed_plugin`).
    pub crash_marker: Option<PathBuf>,
}

/// An activated plugin: the audio side to hand to the audio thread and the handle that keeps
/// the instance open. Fields drop in this order: processor first, then the instance closes.
#[derive(Debug)]
pub struct PluginLoaded {
    pub processor: PluginProcessor,
    pub handle: PluginHandle,
    /// Delay the plugin adds, in samples.
    pub latency_samples: u32,
    /// The parameters of the setup the plugin accepted, as set.
    pub applied: Vec<(u32, f64)>,
}

/// What `describe` found out about a plugin.
#[derive(Debug)]
pub struct PluginDescription {
    pub params: Vec<ParamInfo>,
    pub state: Vec<u8>,
}

/// A load in progress.
#[derive(Debug)]
pub struct PendingLoad {
    reply: Receiver<Result<PluginLoaded, PluginError>>,
}

impl PendingLoad {
    /// The result once the plugin thread has it, `None` before. Not blocking. Dropping a load
    /// that is not collected closes the instance on the plugin thread.
    pub fn poll(&self) -> Option<Result<PluginLoaded, PluginError>> {
        match self.reply.try_recv() {
            Ok(result) => Some(result),
            Err(mpsc::TryRecvError::Empty) => None,
            Err(mpsc::TryRecvError::Disconnected) => Some(Err(gone())),
        }
    }
}

/// An instance on the plugin thread. Dropping it closes the instance (after the processor is
/// gone, see `PluginLoaded`).
#[derive(Debug)]
pub struct PluginHandle {
    id: u64,
    commands: Sender<Command>,
    changes: Arc<Mutex<Vec<(u32, f64)>>>,
    editor: EditorShown,
}

impl PluginHandle {
    /// Tells the plugin a value, in its units; clamped there. No answer: a parameter the
    /// plugin does not have is ignored.
    pub fn set_param(&self, param: u32, value: f64) {
        let _ = self.commands.send(Command::SetParam {
            plugin: self.id,
            param,
            value,
        });
    }

    /// Opens the plugin's own editor in a frame `make_frame` makes (`system_frame` for a real
    /// window); an open one stays. Waits for the plugin thread.
    pub fn open_editor(&self, title: &str, make_frame: FrameMaker) -> Result<(), PluginError> {
        let (reply, receiver) = mpsc::channel();
        let command = Command::OpenEditor {
            plugin: self.id,
            title: title.to_string(),
            make_frame,
            reply,
        };
        self.commands.send(command).map_err(|_| gone())?;
        receiver.recv().map_err(|_| gone())?
    }

    /// Closes the editor if it is open. Not blocking.
    pub fn close_editor(&self) {
        let _ = self.commands.send(Command::CloseEditor { plugin: self.id });
    }

    /// Whether the editor is open now (the player may close it any time).
    pub fn editor_open(&self) -> bool {
        self.editor.get()
    }

    /// Parameter changes the plugin made itself since the last call: one entry per parameter,
    /// the last value. Not blocking.
    pub fn take_changes(&self) -> Vec<(u32, f64)> {
        match self.changes.lock() {
            Ok(mut inbox) => std::mem::take(&mut *inbox),
            Err(_) => Vec::new(),
        }
    }
}

impl Drop for PluginHandle {
    fn drop(&mut self) {
        let _ = self.commands.send(Command::Close { plugin: self.id });
    }
}

enum Command {
    Load {
        id: u64,
        setup: PluginSetup,
        changes: Arc<Mutex<Vec<(u32, f64)>>>,
        handle_commands: Sender<Command>,
        reply: Sender<Result<PluginLoaded, PluginError>>,
    },
    Describe {
        setup: PluginSetup,
        reply: Sender<Result<PluginDescription, PluginError>>,
    },
    Count {
        path: PathBuf,
        reply: Sender<usize>,
    },
    SetParam {
        plugin: u64,
        param: u32,
        value: f64,
    },
    Close {
        plugin: u64,
    },
    OpenEditor {
        plugin: u64,
        title: String,
        make_frame: FrameMaker,
        reply: Sender<Result<(), PluginError>>,
    },
    CloseEditor {
        plugin: u64,
    },
    /// Answers once everything sent before, and what that sent in turn, is done.
    Settle {
        reply: Sender<()>,
    },
}

/// The plugin thread of this process, once something used it.
static HOST: OnceLock<PluginHost> = OnceLock::new();

/// The one plugin thread of the process.
pub struct PluginHost {
    commands: Sender<Command>,
    next_id: AtomicU64,
}

impl fmt::Debug for PluginHost {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PluginHost").finish_non_exhaustive()
    }
}

impl PluginHost {
    /// The host of this process; the thread starts with the first call.
    pub fn global() -> &'static PluginHost {
        HOST.get_or_init(PluginHost::start)
    }

    fn start() -> PluginHost {
        let (commands, inbox) = mpsc::channel();
        // A thread that cannot start leaves a host whose loads all fail ("gone").
        let _ = thread::Builder::new()
            .name("kontra-plugins".into())
            .spawn(move || run(inbox));
        PluginHost {
            commands,
            next_id: AtomicU64::new(1),
        }
    }

    /// Starts loading a plugin on the plugin thread: open, saved state, activation, parameters.
    pub fn load(&self, setup: PluginSetup) -> PendingLoad {
        let (reply, receiver) = mpsc::channel();
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let sent = self.commands.send(Command::Load {
            id,
            setup,
            changes: Arc::default(),
            handle_commands: self.commands.clone(),
            reply,
        });
        // Without a thread the reply sender is gone and the poll says so.
        drop(sent);
        PendingLoad { reply: receiver }
    }

    /// Loads a plugin once to learn its parameters and state. Waits for the plugin thread (it
    /// may be busy with other loads), so call it only for a user's action, never per frame.
    pub fn describe(&self, setup: PluginSetup) -> Result<PluginDescription, PluginError> {
        let (reply, receiver) = mpsc::channel();
        self.commands
            .send(Command::Describe { setup, reply })
            .map_err(|_| gone())?;
        receiver.recv().map_err(|_| gone())?
    }
}

impl PluginHost {
    /// How many instances of the plugin file `path` are open. Waits for the plugin thread; for
    /// tests and diagnostics (an instance that leaks shows here).
    pub fn open_of(&self, path: &Path) -> usize {
        let (reply, receiver) = mpsc::channel();
        let path = path.to_path_buf();
        if self.commands.send(Command::Count { path, reply }).is_err() {
            return 0;
        }
        receiver.recv().unwrap_or(0)
    }
}

/// Waits up to `patience` until the plugin thread has finished every load and closed every
/// instance it was asked to; true when it did. Call it when Kontra ends, after the players are
/// gone: a load cut short would leave its crash marker, and a plugin would end with the program.
pub fn settle_plugins(patience: Duration) -> bool {
    let Some(host) = HOST.get() else {
        return true;
    };
    let (reply, receiver) = mpsc::channel();
    if host.commands.send(Command::Settle { reply }).is_err() {
        return false;
    }
    receiver.recv_timeout(patience).is_ok()
}

fn gone() -> PluginError {
    PluginError::Plugin("the plugin thread is not running".into())
}

/// The plugin file and id a crash marker names, if the program died while loading it.
pub fn crashed_plugin(marker: &Path) -> Option<(PathBuf, String)> {
    let text = std::fs::read_to_string(marker).ok()?;
    let (path, id) = text.split_once(MARKER_SEPARATOR)?;
    Some((PathBuf::from(path), id.to_string()))
}

/// Removes a crash marker once the owner has taken note of it.
pub fn clear_crash_marker(marker: &Path) {
    let _ = std::fs::remove_file(marker);
}

/// Runs `work` with the marker in place.
fn guarded<T>(setup: &PluginSetup, work: impl FnOnce() -> T) -> T {
    let Some(marker) = &setup.crash_marker else {
        return work();
    };
    let text = format!("{}{MARKER_SEPARATOR}{}", setup.path.display(), setup.id);
    let _ = std::fs::write(marker, text);
    let result = work();
    let _ = std::fs::remove_file(marker);
    result
}

/// What the thread keeps of an instance.
struct Slot {
    plugin: LoadedPlugin,
    changes: Arc<Mutex<Vec<(u32, f64)>>>,
    editor: EditorShown,
}

/// The loop of the plugin thread.
fn run(inbox: Receiver<Command>) {
    let mut slots: HashMap<u64, Slot> = HashMap::new();
    loop {
        match inbox.recv_timeout(TICK) {
            Ok(Command::Settle { reply }) => {
                // Closes that the work before sent (a load nobody waited for) come after this.
                while let Ok(command) = inbox.try_recv() {
                    handle(command, &mut slots);
                }
                let _ = reply.send(());
            }
            Ok(command) => handle(command, &mut slots),
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => return,
        }
        for slot in slots.values_mut() {
            slot.plugin.idle();
            slot.editor.set(slot.plugin.editor_open());
        }
        collect_changes(&mut slots);
    }
}

fn handle(command: Command, slots: &mut HashMap<u64, Slot>) {
    match command {
        Command::Load {
            id,
            setup,
            changes,
            handle_commands,
            reply,
        } => {
            let loaded = guarded(&setup, || open(&setup));
            let result = loaded.map(|opened| {
                let editor = EditorShown::default();
                slots.insert(
                    id,
                    Slot {
                        plugin: opened.plugin,
                        changes: Arc::clone(&changes),
                        editor: editor.clone(),
                    },
                );
                PluginLoaded {
                    processor: opened.processor,
                    handle: PluginHandle {
                        id,
                        commands: handle_commands,
                        changes,
                        editor,
                    },
                    latency_samples: opened.latency_samples,
                    applied: opened.applied,
                }
            });
            // A reply nobody waits for drops here: the processor, then the handle, which closes.
            let _ = reply.send(result);
        }
        Command::Describe { setup, reply } => {
            let _ = reply.send(guarded(&setup, || describe(&setup)));
        }
        // A second settle in the same drain: everything before it is done too.
        Command::Settle { reply } => {
            let _ = reply.send(());
        }
        Command::Count { path, reply } => {
            let open = slots.values().filter(|s| s.plugin.info().path == path);
            let _ = reply.send(open.count());
        }
        Command::SetParam {
            plugin,
            param,
            value,
        } => {
            if let Some(slot) = slots.get_mut(&plugin) {
                let _ = slot.plugin.set_param(param, value);
            }
        }
        Command::Close { plugin } => {
            // The editor goes before the instance: the plugin lets go of its window first.
            if let Some(mut slot) = slots.remove(&plugin) {
                slot.plugin.close_editor();
            }
        }
        Command::OpenEditor {
            plugin,
            title,
            make_frame,
            reply,
        } => {
            let Some(slot) = slots.get_mut(&plugin) else {
                let _ = reply.send(Err(gone()));
                return;
            };
            let opened = slot.plugin.open_editor(&title, make_frame);
            slot.editor.set(slot.plugin.editor_open());
            let _ = reply.send(opened);
        }
        Command::CloseEditor { plugin } => {
            if let Some(slot) = slots.get_mut(&plugin) {
                slot.plugin.close_editor();
                slot.editor.set(false);
            }
        }
    }
}

/// A plugin opened on this thread.
struct Opened {
    plugin: LoadedPlugin,
    processor: PluginProcessor,
    latency_samples: u32,
    applied: Vec<(u32, f64)>,
}

/// Opens, restores and activates; the instance stays here, the processor goes to the caller.
fn open(setup: &PluginSetup) -> Result<Opened, PluginError> {
    let mut plugin = LoadedPlugin::open(setup.format, &setup.path, Some(&setup.id))?;
    if !setup.state.is_empty() {
        // A state that does not fit (another version) leaves the plugin at its defaults.
        let _ = plugin.load_state(&setup.state);
    }
    let processor = plugin.activate(setup.sample_rate, setup.max_block)?;
    let latency = plugin.latency_samples();

    let mut applied = Vec::new();
    for &(id, value) in &setup.params {
        if plugin.set_param(id, value).is_ok() {
            applied.push((id, value));
        }
    }
    Ok(Opened {
        plugin,
        processor,
        latency_samples: latency,
        applied,
    })
}

fn describe(setup: &PluginSetup) -> Result<PluginDescription, PluginError> {
    let mut plugin = LoadedPlugin::open(setup.format, &setup.path, Some(&setup.id))?;
    drop(plugin.activate(setup.sample_rate, setup.max_block)?);
    plugin.deactivate()?;
    Ok(PluginDescription {
        params: plugin.params(),
        state: plugin.save_state().unwrap_or_default(),
    })
}

/// Moves what plugins changed by themselves to the inboxes of their handles.
fn collect_changes(slots: &mut HashMap<u64, Slot>) {
    for slot in slots.values_mut() {
        let fresh = slot.plugin.take_changes();
        if fresh.is_empty() {
            continue;
        }
        let Ok(mut inbox) = slot.changes.lock() else {
            continue;
        };
        let merged = coalesce(inbox.drain(..).chain(fresh));
        *inbox = merged;
    }
}
