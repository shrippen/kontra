//! What the output shares with the input, and the devices beside the system's own ones
//! (Windows: ASIO drivers).

#[cfg(all(windows, feature = "asio"))]
use super::{describe, DeviceInfo};
#[cfg(windows)]
use super::{devices_turn, find_device, Direction};
#[cfg(windows)]
use cpal::traits::DeviceTrait;
#[cfg(all(windows, feature = "asio"))]
use cpal::traits::HostTrait;

/// The rate to open the output at so the input can be played through it (monitoring needs
/// both at one rate); `None`: the output's own rate is right, or nothing better is known.
///
/// Windows (shared mode): each device runs at the rate set in the sound settings, and an
/// input opens at no other. Example: a Scarlett Solo, input 48 kHz, output 44.1 kHz; the
/// output is opened at 48 kHz (Windows converts) and the bass is heard.
#[cfg(windows)]
pub fn monitor_rate(input: Option<&str>, output: Option<&str>) -> Option<u32> {
    let _turn = devices_turn();
    let input = find_device(Direction::Input, input).ok()?;
    let output = find_device(Direction::Output, output).ok()?;
    let input_rate = input.default_input_config().ok()?.sample_rate();
    let output_rate = output.default_output_config().ok()?.sample_rate();
    let ranges: Vec<(u32, u32)> = output
        .supported_output_configs()
        .map(|configs| {
            configs
                .map(|c| (c.min_sample_rate(), c.max_sample_rate()))
                .collect()
        })
        .unwrap_or_default();
    rate_to_share(input_rate, output_rate, &ranges)
}

/// Elsewhere both ends already run at one rate (PipeWire: the graph's), or the device decides
/// (ALSA): the output keeps its own.
#[cfg(not(windows))]
pub fn monitor_rate(_input: Option<&str>, _output: Option<&str>) -> Option<u32> {
    None
}

/// The input's rate when it differs from the output's and the output takes it, given as
/// `(lowest, highest)` rates per supported config.
#[cfg_attr(not(windows), allow(dead_code))]
fn rate_to_share(input: u32, output: u32, output_takes: &[(u32, u32)]) -> Option<u32> {
    if input == output {
        return None;
    }
    let taken = output_takes
        .iter()
        .any(|(low, high)| (*low..=*high).contains(&input));
    taken.then_some(input)
}

/// ASIO drivers as further devices (ids "asio:<driver>"), the way to low latency under
/// Windows; cpal's WASAPI streams run in shared mode only. Named with the system, so a
/// driver and the same interface over WASAPI read apart: "Focusrite USB ASIO (ASIO)".
#[cfg(all(windows, feature = "asio"))]
pub(super) fn asio_devices(direction: Direction) -> Vec<DeviceInfo> {
    const MARK: &str = " (ASIO)";
    let Ok(host) = cpal::host_from_id(cpal::HostId::Asio) else {
        return Vec::new();
    };
    let devices = match direction {
        Direction::Input => host.input_devices().map(|d| d.collect::<Vec<_>>()),
        Direction::Output => host.output_devices().map(|d| d.collect::<Vec<_>>()),
    };
    devices
        .unwrap_or_default()
        .iter()
        .filter_map(|d| describe(d, direction, None))
        .map(|mut device| {
            device.name.push_str(MARK);
            device
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Win2 (10.11.2026, Scarlett Solo under Windows): input at 48 kHz, output at 44.1 kHz,
    /// and the bass could not be played through. The output follows the input where it can.
    #[test]
    fn the_output_shares_the_inputs_rate() {
        let takes = [(8_000, 384_000)];
        assert_eq!(rate_to_share(48_000, 44_100, &takes), Some(48_000));
        assert_eq!(
            rate_to_share(48_000, 48_000, &takes),
            None,
            "nothing to change"
        );
        // An output fixed at its own rate keeps it; monitoring then says why it is silent.
        assert_eq!(rate_to_share(48_000, 44_100, &[(44_100, 44_100)]), None);
        assert_eq!(rate_to_share(48_000, 44_100, &[]), None);
    }
}
