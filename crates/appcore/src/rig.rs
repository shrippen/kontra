//! One set of audio devices across the modes (Runde 20: "Das Wechseln von Player zu Player
//! darf nicht spürbar sein!"). A mode that ends parks its devices; the next one takes them over
//! if they fit, instead of closing and opening them (31–42 ms of silence on the Scarlett).
//!
//! ```text
//!  standby ─park─► rig ─take─► tuner ─park─► rig ─take─► run ─park─► rig ─take─► standby
//!                   └─ does not fit (other device, failed, microphone) ─► closes, mode opens anew
//! ```
//!
//! Only the app keeps devices (`keep_devices_open`), and only at a cable input: a microphone
//! and MIDI keep the gap (owner, 11.11.2026). The loopback opens its own devices.

use crate::mixer::{ear_gains, gains};
use crate::Core;
use kontra_model::{Arrangement, InputSource, Instrument};
use kontra_session::{
    Live, LiveEar, LiveError, LiveSetup, PracticeOptions, Rig, RigOutput, ScoreSettings,
};

impl Core {
    /// Keeps `rig` for the next mode while the app holds devices open at a cable input; else it
    /// closes.
    pub(crate) fn park(&mut self, rig: Rig) {
        let cable = matches!(self.source(), InputSource::Line | InputSource::LaptopJack);
        if self.standby.keeps_devices() && cable {
            self.standby.rig = Some(rig);
        }
    }

    /// The parked devices if they are the ones `setup` asks for; others close first (a direct
    /// device opens only once).
    pub(crate) fn take_rig(&mut self, setup: &LiveSetup) -> Option<Rig> {
        let rig = self.standby.rig.take()?;
        let output = RigOutput::Device(setup.output_device.clone());
        let fits = rig.fits(&setup.input, &output, setup.source);
        #[cfg(test)]
        {
            self.standby.taken += u32::from(fits);
        }
        fits.then_some(rig)
    }

    /// Closes the parked devices (another device chosen, one lost, the loopback).
    pub(crate) fn drop_rig(&mut self) {
        self.standby.rig = None;
    }

    /// A run on the parked devices, or on its own; the mode and gains are the player's.
    pub(crate) fn open_live(
        &mut self,
        arrangement: Arrangement,
        settings: ScoreSettings,
        options: PracticeOptions,
        setup: &LiveSetup,
    ) -> Result<Live, LiveError> {
        let mode = self.mode();
        match self.take_rig(setup) {
            Some(rig) => {
                let gains = gains(&self.settings, &self.plugins);
                Live::on(rig, gains, arrangement, mode, settings, options, setup)
            }
            None => Live::start(arrangement, mode, settings, options, setup),
        }
    }

    /// Ear training on the parked devices, or on its own, at the prompt volume.
    pub(crate) fn open_ear(
        &mut self,
        setup: &LiveSetup,
        instrument: &Instrument,
    ) -> Result<LiveEar, String> {
        let gains = ear_gains(&self.settings, &self.plugins);
        let mut ear = match self.take_rig(setup) {
            Some(rig) => LiveEar::on(rig, setup, instrument, gains.clone()),
            None => LiveEar::start(setup, instrument)
                .map_err(|e| self.tr("error.audio", &[("reason", e.to_string())]))?,
        };
        ear.set_gains(gains);
        Ok(ear)
    }

    /// Ends tuner and ear training; their devices go on to the next mode.
    pub(crate) fn end_tuner_and_ear(&mut self) {
        self.end_tuner();
        self.end_ear();
    }

    /// Ends the tuner; its devices go on to the next mode.
    pub(crate) fn end_tuner(&mut self) {
        if let Some(tuner) = self.tuner.take() {
            self.park(tuner.into_rig());
        }
    }

    /// Ends ear training's devices; they go on to the next mode.
    pub(crate) fn end_ear(&mut self) {
        if let Some(ear) = self.live_ear.take() {
            self.park(ear.into_rig());
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{Core, QaOptions};
    use std::path::PathBuf;

    /// Long enough for every switch of a test: the recording plays in real time.
    const TAKE_S: usize = 6;
    const RATE: u32 = 48_000;

    /// A core with the null output and a silent recording as its cable input.
    fn core(name: &str) -> (Core, PathBuf) {
        let dir = std::env::temp_dir().join(format!("kontra-rig-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::create_dir_all(&dir);
        let settings = serde_json::json!({ "output_device": "kontra:null", "monitor": true });
        let _ = std::fs::write(dir.join("settings.json"), settings.to_string());
        let take = dir.join("take.wav");
        let silence = vec![0.0; TAKE_S * RATE as usize];
        kontra_session::write_mono_wav(&take, &silence, RATE).unwrap();
        let qa = QaOptions {
            input_file: Some(take),
            laptop_jack: false,
        };
        (Core::new(&dir, qa), dir)
    }

    /// Runde 20: tuner → ear training → tuner → calibration run on the devices the first one
    /// opened; none opens its own.
    #[test]
    fn modes_hand_the_devices_on() {
        let (mut core, dir) = core("switch");
        core.keep_devices_open();

        core.start_tuner().unwrap();
        core.stop_tuner();
        assert!(core.standby.rig.is_some(), "the tuner parks its devices");
        core.ear_start("echo").unwrap();
        core.ear_stop();
        core.start_tuner().unwrap();
        core.start_calibration().unwrap();
        let taken = core.standby.taken;
        let _ = std::fs::remove_dir_all(dir);

        assert_eq!(
            taken, 3,
            "ear training, the tuner and the calibration took them over"
        );
    }

    /// LH-EIN-18: only the app holds devices open; tests and the CLI never.
    #[test]
    fn without_the_app_nothing_stays_open() {
        let (mut core, dir) = core("cli");
        core.start_tuner().unwrap();
        core.stop_tuner();
        let _ = std::fs::remove_dir_all(dir);
        assert!(core.standby.rig.is_none());
    }

    /// Another output chosen: the parked devices close at once.
    #[test]
    fn another_device_closes_the_parked_rig() {
        let (mut core, dir) = core("other");
        core.keep_devices_open();
        core.start_tuner().unwrap();
        core.stop_tuner();
        core.update_settings(|s| s.output_device = Some("kontra:null-2".into()));
        let _ = std::fs::remove_dir_all(dir);
        assert!(core.standby.rig.is_none());
    }

    /// The loopback records on devices of its own: the parked ones close first.
    #[test]
    fn the_loopback_opens_its_own_devices() {
        let (mut core, dir) = core("loopback");
        core.keep_devices_open();
        core.start_tuner().unwrap();
        core.start_loopback().unwrap();
        let _ = std::fs::remove_dir_all(dir);
        assert!(core.standby.rig.is_none());
        assert_eq!(core.standby.taken, 0);
    }
}
