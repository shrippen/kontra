//! The PipeWire graph as the device lists need it: which nodes are sources and sinks, and which
//! are the system defaults. cpal opens the streams, but it lists every node (sink monitors as
//! inputs, other programs' streams) and does not say which node is the default.
//!
//! ```text
//! registry ──► nodes: name → class (Audio/Source, Audio/Sink, Audio/Duplex)
//!          └─► metadata "default" ──► default.audio.source / default.audio.sink
//! ```

use crate::audio::Direction;
use pipewire::{
    context::ContextRc, core::PW_ID_CORE, main_loop::MainLoopRc, metadata::Metadata,
    spa::utils::result::AsyncSeq, types::ObjectType,
};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::time::Duration;

/// A server that accepts the connection but never answers must not hang the settings page.
const READ_TIMEOUT: Duration = Duration::from_secs(2);
const MEDIA_CLASS: &str = "media.class";
const NODE_NAME: &str = "node.name";
const METADATA_NAME: &str = "metadata.name";
/// The metadata object holding the effective defaults (WirePlumber).
const DEFAULT_METADATA: &str = "default";
const DEFAULT_SOURCE: &str = "default.audio.source";
const DEFAULT_SINK: &str = "default.audio.sink";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NodeClass {
    Source,
    Sink,
    Duplex,
}

impl NodeClass {
    /// "Audio/Source", "Audio/Source/Virtual", "Audio/Sink", "Audio/Duplex"; streams of other
    /// programs ("Stream/Output/Audio") and video are not devices.
    fn parse(media_class: &str) -> Option<NodeClass> {
        if media_class.starts_with("Audio/Source") {
            return Some(NodeClass::Source);
        }
        if media_class.starts_with("Audio/Sink") {
            return Some(NodeClass::Sink);
        }
        if media_class.starts_with("Audio/Duplex") {
            return Some(NodeClass::Duplex);
        }
        None
    }

    fn serves(self, direction: Direction) -> bool {
        matches!(
            (self, direction),
            (NodeClass::Duplex, _)
                | (NodeClass::Source, Direction::Input)
                | (NodeClass::Sink, Direction::Output)
        )
    }
}

#[derive(Debug, Default, Clone, PartialEq)]
pub(crate) struct Graph {
    classes: HashMap<String, NodeClass>,
    default_source: Option<String>,
    default_sink: Option<String>,
}

impl Graph {
    /// Whether node `name` is a device for `direction` (a sink is no input: its monitor would
    /// record the music, not the bass).
    pub(crate) fn serves(&self, name: &str, direction: Direction) -> bool {
        self.classes
            .get(name)
            .is_some_and(|class| class.serves(direction))
    }

    pub(crate) fn is_default(&self, name: &str, direction: Direction) -> bool {
        let default = match direction {
            Direction::Input => &self.default_source,
            Direction::Output => &self.default_sink,
        };
        default.as_deref() == Some(name)
    }

    /// Reads the graph once. Call only while a cpal PipeWire host lives: cpal initialises the
    /// library and deinitialises it when its last host or stream is dropped.
    pub(crate) fn read() -> Option<Graph> {
        let mainloop = MainLoopRc::new(None).ok()?;
        let context = ContextRc::new(&mainloop, None).ok()?;
        let core = context.connect_rc(None).ok()?;
        let registry = core.get_registry_rc().ok()?;
        let graph = Rc::new(RefCell::new(Graph::default()));
        let pending: Rc<RefCell<Vec<AsyncSeq>>> = Rc::new(RefCell::new(vec![core.sync(0).ok()?]));
        let metadata: Rc<RefCell<Vec<Box<dyn std::any::Any>>>> = Rc::default();

        // Done when every sync (the first, one per bound metadata) came back.
        let quit = mainloop.clone();
        let pending_done = Rc::clone(&pending);
        let complete = Rc::new(std::cell::Cell::new(Reading::TimedOut));
        let complete_done = Rc::clone(&complete);
        let _core_listener = core
            .add_listener_local()
            .done(move |id, seq| {
                if id != PW_ID_CORE {
                    return;
                }
                let mut list = pending_done.borrow_mut();
                list.retain(|s| *s != seq);
                if list.is_empty() {
                    complete_done.set(Reading::Complete);
                    quit.quit();
                }
            })
            .register();

        let graph_global = Rc::clone(&graph);
        let registry_bind = registry.clone();
        let core_sync = core.clone();
        let _registry_listener = registry
            .add_listener_local()
            .global(move |global| {
                let Some(props) = global.props else {
                    return;
                };
                if global.type_ == ObjectType::Node {
                    let class = props.get(MEDIA_CLASS).and_then(NodeClass::parse);
                    if let (Some(class), Some(name)) = (class, props.get(NODE_NAME)) {
                        graph_global
                            .borrow_mut()
                            .classes
                            .insert(name.to_string(), class);
                    }
                    return;
                }
                if global.type_ != ObjectType::Metadata
                    || props.get(METADATA_NAME) != Some(DEFAULT_METADATA)
                {
                    return;
                }
                let Ok(bound) = registry_bind.bind::<Metadata, _>(global) else {
                    return;
                };
                let graph_meta = Rc::clone(&graph_global);
                let listener = bound
                    .add_listener_local()
                    .property(move |_, key, _, value| {
                        let name = value.and_then(default_name);
                        let mut graph = graph_meta.borrow_mut();
                        match key {
                            Some(DEFAULT_SOURCE) => graph.default_source = name,
                            Some(DEFAULT_SINK) => graph.default_sink = name,
                            _ => {}
                        }
                        0
                    })
                    .register();
                if let Ok(seq) = core_sync.sync(0) {
                    pending.borrow_mut().push(seq);
                }
                let mut kept = metadata.borrow_mut();
                kept.push(Box::new(bound));
                kept.push(Box::new(listener));
            })
            .register();

        let quit = mainloop.clone();
        let timer = mainloop.loop_().add_timer(move |_| quit.quit());
        let _ = timer.update_timer(Some(READ_TIMEOUT), None);
        mainloop.run();

        let graph = graph.borrow().clone();
        finished(graph, complete.get())
    }

    #[cfg(test)]
    pub(crate) fn fake(
        nodes: &[(&str, NodeClass)],
        default_source: Option<&str>,
        default_sink: Option<&str>,
    ) -> Graph {
        Graph {
            classes: nodes
                .iter()
                .map(|(name, class)| (name.to_string(), *class))
                .collect(),
            default_source: default_source.map(str::to_string),
            default_sink: default_sink.map(str::to_string),
        }
    }
}

/// Whether every answer came before the timeout.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Reading {
    Complete,
    TimedOut,
}

/// A partial graph would drop devices it has not seen yet; then the caller keeps cpal's list.
fn finished(graph: Graph, reading: Reading) -> Option<Graph> {
    match reading {
        Reading::Complete => Some(graph),
        Reading::TimedOut => None,
    }
}

/// The node name in a default entry: `{ "name": "alsa_output.usb-…" }`.
fn default_name(value: &str) -> Option<String> {
    let (_, rest) = value.split_once("\"name\"")?;
    let (_, rest) = rest.split_once('"')?;
    let (name, _) = rest.split_once('"')?;
    Some(name.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classes_from_media_class() {
        assert_eq!(NodeClass::parse("Audio/Source"), Some(NodeClass::Source));
        assert_eq!(
            NodeClass::parse("Audio/Source/Virtual"),
            Some(NodeClass::Source)
        );
        assert_eq!(NodeClass::parse("Audio/Sink"), Some(NodeClass::Sink));
        assert_eq!(NodeClass::parse("Audio/Duplex"), Some(NodeClass::Duplex));
        assert_eq!(NodeClass::parse("Stream/Output/Audio"), None);
        assert_eq!(NodeClass::parse("Midi/Bridge"), None);
    }

    // A sink is no input (its monitor records the music); a duplex node serves both.
    #[test]
    fn sinks_are_no_inputs() {
        let graph = Graph::fake(
            &[
                ("alsa_input.usb-Scarlett", NodeClass::Source),
                ("alsa_output.usb-Scarlett", NodeClass::Sink),
                ("loop", NodeClass::Duplex),
            ],
            Some("alsa_input.usb-Scarlett"),
            None,
        );
        assert!(graph.serves("alsa_input.usb-Scarlett", Direction::Input));
        assert!(!graph.serves("alsa_output.usb-Scarlett", Direction::Input));
        assert!(graph.serves("alsa_output.usb-Scarlett", Direction::Output));
        assert!(graph.serves("loop", Direction::Input) && graph.serves("loop", Direction::Output));
        assert!(!graph.serves("mpv", Direction::Output));
        assert!(graph.is_default("alsa_input.usb-Scarlett", Direction::Input));
        assert!(!graph.is_default("alsa_input.usb-Scarlett", Direction::Output));
    }

    // Review #73: after the timeout the graph is partial; filtering by it dropped every device.
    #[test]
    fn unfinished_reading_gives_no_graph() {
        let graph = Graph::fake(&[("a", NodeClass::Source)], None, None);
        assert_eq!(finished(graph.clone(), Reading::TimedOut), None);
        assert_eq!(finished(graph.clone(), Reading::Complete), Some(graph));
    }

    #[test]
    fn reads_default_names() {
        assert_eq!(
            default_name(r#"{ "name": "alsa_output.pci-0000_12_00.4.iec958-stereo" }"#).as_deref(),
            Some("alsa_output.pci-0000_12_00.4.iec958-stereo")
        );
        assert_eq!(default_name("{}"), None);
    }
}
