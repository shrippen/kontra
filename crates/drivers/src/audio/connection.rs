//! How a device is connected: guessed from its name, and a USB microphone told from a
//! USB interface (LH-EIN-02): under PipeWire by the device's form factor (headset, webcam,
//! microphone), else by having no output. cpal gives no WASAPI form factor: Windows uses only the
//! output rule, so a USB headset stays an interface there.

use super::{Connection, DeviceInfo};

pub(super) fn guess_connection(name: &str) -> Connection {
    let lower = name.to_lowercase();
    if lower.contains("bluez") || lower.contains("bluetooth") {
        return Connection::Bluetooth;
    }
    if lower.contains("usb") || lower.contains("scarlett") || lower.contains("focusrite") {
        return Connection::Usb;
    }
    if lower.contains("hdmi") || lower.contains("displayport") {
        return Connection::Hdmi;
    }
    Connection::Other
}

/// Prefixes of PipeWire's ALSA node names; the rest names the card and profile.
const NODE_PREFIXES: [&str; 2] = ["alsa_input.", "alsa_output."];

/// What an input and an output of one USB device have in common: PipeWire's node name without
/// direction and profile ("usb-Focusrite_Scarlett_Solo_USB-00" for `.analog-stereo` and
/// `.pro-output-0`), else the driver in the last parentheses as WASAPI names it ("Analogue 1 + 2
/// (Focusrite USB Audio)" and "Lautsprecher (Focusrite USB Audio)" share "Focusrite USB Audio").
fn device_key(device: &DeviceInfo) -> &str {
    let node = device.id.strip_prefix("pipewire:").unwrap_or(&device.id);
    if let Some(card) = NODE_PREFIXES.iter().find_map(|p| node.strip_prefix(p)) {
        return card.rsplit_once('.').map_or(card, |(card, _profile)| card);
    }
    let name = device.name.trim_end();
    match (name.rfind('('), name.ends_with(')')) {
        (Some(open), true) => &name[open + 1..name.len() - 1],
        _ => name,
    }
}

/// A USB input whose device offers no output is a microphone, not the bass interface: a webcam
/// must not be taken by "Automatic" nor held open in standby (review #231). ALSA fallback inputs
/// (`past_pipewire`) have no PipeWire output to match and stay interfaces (review #73).
pub(super) fn mark(inputs: &mut [DeviceInfo], outputs: &[DeviceInfo]) {
    for input in inputs
        .iter_mut()
        .filter(|d| d.connection == Connection::Usb && !d.past_pipewire)
    {
        let key = device_key(input);
        let plays = outputs
            .iter()
            .any(|o| o.connection == Connection::Usb && device_key(o) == key);
        if !plays {
            input.connection = Connection::UsbMicrophone;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio::Direction;

    fn device(id: &str, direction: Direction, connection: Connection) -> DeviceInfo {
        DeviceInfo {
            id: id.into(),
            name: id.into(),
            direction,
            is_default: false,
            connection,
            sample_rate: 48_000,
            channels: 2,
            past_pipewire: false,
        }
    }

    fn named(id: &str, name: &str, direction: Direction) -> DeviceInfo {
        DeviceInfo {
            name: name.into(),
            ..device(id, direction, Connection::Usb)
        }
    }

    // Windows: the Scarlett plays back as "Lautsprecher (Focusrite USB Audio)"; a webcam only
    // listens. Linux: the Scarlett has a source and a sink node of one card.
    #[test]
    fn guesses_connections() {
        assert_eq!(
            guess_connection("bluez_output.AA_BB"),
            Connection::Bluetooth
        );
        assert_eq!(guess_connection("Scarlett Solo USB"), Connection::Usb);
        assert_eq!(guess_connection("HDA Intel HDMI 0"), Connection::Hdmi);
    }

    // Review #254: the ALSA fallback of a card PipeWire lost has no PipeWire output to match; it
    // stays an interface (review #73).
    #[test]
    fn a_direct_input_stays_an_interface() {
        let mut direct = named(
            "alsa:plughw:CARD=2,DEV=0",
            "Scarlett Solo USB, USB Audio",
            Direction::Input,
        );
        direct.past_pipewire = true;
        let mut inputs = vec![direct];
        let outputs = vec![named(
            "pipewire:alsa_output.usb-Focusrite_Scarlett_Solo_USB-00.pro-output-0",
            "Scarlett Solo USB Pro",
            Direction::Output,
        )];
        mark(&mut inputs, &outputs);
        assert_eq!(inputs[0].connection, Connection::Usb);
    }

    // Review #254: a USB headset also plays back; PipeWire's form factor says it is a voice device.
    #[cfg(target_os = "linux")]
    #[test]
    fn a_headset_is_a_microphone() {
        use crate::pipewire::NodeClass;
        let headset = "alsa_input.usb-Logitech_H390-00.analog-stereo";
        let scarlett = "alsa_input.usb-Focusrite_Scarlett_Solo_USB-00.analog-stereo";
        let graph = crate::pipewire::Graph::fake(
            &[(headset, NodeClass::Source), (scarlett, NodeClass::Source)],
            None,
            None,
        )
        .with_voice(headset);
        let listed = vec![
            device(
                &format!("pipewire:{headset}"),
                Direction::Input,
                Connection::Usb,
            ),
            device(
                &format!("pipewire:{scarlett}"),
                Direction::Input,
                Connection::Usb,
            ),
        ];
        let kinds: Vec<_> = crate::audio::pipewire_devices(listed, &graph)
            .iter()
            .map(|d| d.connection)
            .collect();
        assert_eq!(kinds, [Connection::UsbMicrophone, Connection::Usb]);
    }

    #[test]
    fn an_input_only_usb_device_is_a_microphone() {
        let mut inputs = vec![
            named(
                "w1",
                "Analogue 1 + 2 (Focusrite USB Audio)",
                Direction::Input,
            ),
            named("w2", "Mikrofon (HD Webcam C270)", Direction::Input),
            named(
                "pipewire:alsa_input.usb-Focusrite_Scarlett_Solo_USB-00.analog-stereo",
                "Scarlett Solo USB Analog Stereo",
                Direction::Input,
            ),
            named(
                "pipewire:alsa_input.usb-046d_HD_Webcam-02.mono-fallback",
                "HD Webcam Mono",
                Direction::Input,
            ),
            device("hda", Direction::Input, Connection::BuiltIn),
        ];
        let outputs = vec![
            named(
                "o1",
                "Lautsprecher (Focusrite USB Audio)",
                Direction::Output,
            ),
            named(
                "pipewire:alsa_output.usb-Focusrite_Scarlett_Solo_USB-00.pro-output-0",
                "Scarlett Solo USB Pro",
                Direction::Output,
            ),
        ];
        mark(&mut inputs, &outputs);
        let kinds: Vec<_> = inputs.iter().map(|d| d.connection).collect();
        assert_eq!(
            kinds,
            [
                Connection::Usb,
                Connection::UsbMicrophone,
                Connection::Usb,
                Connection::UsbMicrophone,
                Connection::BuiltIn
            ]
        );
    }
}
