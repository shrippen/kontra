//! Command line arguments.

use clap::{Args, Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(
    name = "kontra-cli",
    version,
    about = "Kontra bass trainer on the command line"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// List audio devices.
    Devices,
    /// Import a Guitar Pro, MIDI, MusicXML or MuseScore file and show the bass part.
    Import(ImportArgs),
    /// Detect notes in an audio recording.
    Analyze(AnalyzeArgs),
    /// Score a recording against a score file.
    Score(ScoreArgs),
    /// Generate or check the synthetic test corpus.
    Korpus(KorpusArgs),
    /// Practice live with an audio device or a file as input.
    Practice(PracticeArgs),
    /// Chromatic tuner.
    Tune(TuneArgs),
    /// Render a synthetic performance of a score (QA input files).
    Render(RenderArgs),
    /// Endurance test (LH-NF-04): loop a score with time-stretched backing, count dropouts.
    Soak(SoakArgs),
    /// Write the bundled exercises (one folder per level) as MIDI files.
    Exercises(ExercisesArgs),
    /// Hardware check (round 10): loopback latency, play on the click, bass takes for the
    /// corpus, optionally soak and stem timing; every measurement goes into a report.
    Check(CheckArgs),
    /// Course packages (LH-LRN-15): lay out, check and pack a `.kontrakurs`.
    Course(CourseArgs),
}

#[derive(Debug, Args)]
pub struct CourseArgs {
    #[command(subcommand)]
    pub command: CourseCommand,
}

#[derive(Debug, Subcommand)]
pub enum CourseCommand {
    /// Lay out a new course in an empty folder: manifest, course, one lesson.
    New {
        dir: PathBuf,
        /// Prefix of every id in the course, a-z and - (e.g. "studio-weber").
        #[arg(long)]
        id: String,
        #[arg(long)]
        provider: String,
        /// Licence of the texts and media (e.g. "CC-BY-4.0" or "all rights reserved").
        #[arg(long)]
        license: String,
    },
    /// Check a course folder or `.kontrakurs` file; lists every problem, fails if any.
    Check { path: PathBuf },
    /// Check the folder and pack it as a `.kontrakurs` file.
    Pack { dir: PathBuf, out: PathBuf },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum CheckStep {
    Loopback,
    Calibrate,
    Takes,
    Soak,
    Stems,
}

#[derive(Debug, Args)]
pub struct CheckArgs {
    /// Folder for the report and takes (default: testdata/korpus/local/own/check-<date>).
    #[arg(long)]
    pub out: Option<PathBuf>,
    /// Input device id (see `devices`).
    #[arg(long)]
    pub input: Option<String>,
    /// Output device id (see `devices`).
    #[arg(long)]
    pub output: Option<String>,
    /// The bass goes into a laptop combo jack.
    #[arg(long)]
    pub laptop_jack: bool,
    /// Steps in order (default: loopback, calibrate, takes).
    #[arg(long, value_delimiter = ',')]
    pub steps: Vec<CheckStep>,
    /// Takes to record by name (default: all; see the list when it starts).
    #[arg(long, value_delimiter = ',')]
    pub takes: Vec<String>,
    /// Soak: minutes and the score it loops.
    #[arg(long, default_value_t = 60.0)]
    pub soak_minutes: f64,
    #[arg(long, default_value = "testdata/demo/groove-techniken.musicxml")]
    pub soak_score: PathBuf,
    /// Stems: the song Demucs separates (LH-BIB-09: a 4-minute song in under 3 min).
    #[arg(long)]
    pub song: Option<PathBuf>,
    /// Latency for the take annotations when `calibrate` is not run.
    #[arg(long, default_value_t = 0.0)]
    pub latency_ms: f64,
    /// Do not wait for Enter (scripted runs).
    #[arg(long)]
    pub yes: bool,
    /// A recording instead of the audio input (checks the check itself).
    #[arg(long)]
    pub input_file: Option<PathBuf>,
}

#[derive(Debug, Args)]
pub struct ExercisesArgs {
    pub dir: PathBuf,
}

#[derive(Debug, Args)]
pub struct SoakArgs {
    pub score: PathBuf,
    #[arg(long, default_value_t = 60.0)]
    pub minutes: f64,
    /// Playback speed; anything but 1 time-stretches the backing.
    #[arg(long, default_value_t = 0.8)]
    pub speed: f64,
    /// Backing track (default: a synthetic performance of the score).
    #[arg(long)]
    pub backing: Option<PathBuf>,
    /// Output device id (see `devices`); `null` = a timer instead of a sound card.
    #[arg(long, default_value = "null")]
    pub output: String,
    /// Fail when more dropouts than this are counted.
    #[arg(long, default_value_t = 0)]
    pub max_xruns: u64,
}

#[derive(Debug, Args)]
pub struct ImportArgs {
    pub file: PathBuf,
    /// Track index (default: the recognized bass track).
    #[arg(long)]
    pub track: Option<usize>,
    /// Bars of tab to print.
    #[arg(long, default_value_t = 4)]
    pub bars: usize,
    /// Write the arrangement as JSON.
    #[arg(long)]
    pub json: Option<PathBuf>,
}

#[derive(Debug, Args)]
pub struct AnalyzeArgs {
    pub file: PathBuf,
    #[arg(long, default_value_t = 4)]
    pub strings: u8,
    #[arg(long)]
    pub fretless: bool,
    /// Simulate a laptop combo jack.
    #[arg(long)]
    pub laptop_jack: bool,
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum StrictnessArg {
    Loose,
    Normal,
    Strict,
}

impl From<StrictnessArg> for kontra_session::Strictness {
    fn from(value: StrictnessArg) -> Self {
        match value {
            StrictnessArg::Loose => kontra_session::Strictness::Loose,
            StrictnessArg::Normal => kontra_session::Strictness::Normal,
            StrictnessArg::Strict => kontra_session::Strictness::Strict,
        }
    }
}

#[derive(Debug, Args)]
pub struct ScoreArgs {
    pub score: PathBuf,
    pub recording: PathBuf,
    #[arg(long)]
    pub track: Option<usize>,
    /// Speed the recording was played at (0.25–1.5).
    #[arg(long, default_value_t = 1.0)]
    pub speed: f64,
    /// Seconds into the recording where the score starts.
    #[arg(long, default_value_t = 0.0)]
    pub offset: f64,
    #[arg(long, value_enum, default_value = "normal")]
    pub strictness: StrictnessArg,
    #[arg(long, default_value_t = 0.0)]
    pub latency_ms: f64,
    #[arg(long)]
    pub laptop_jack: bool,
}

#[derive(Debug, Subcommand)]
pub enum KorpusCommand {
    /// Write every set as WAV and JSON annotation into a directory.
    Write { dir: PathBuf },
    /// Check detection quality on the generated sets (or WAV+JSON pairs in a directory).
    Check { dir: Option<PathBuf> },
    /// Measure detection on every WAV+JSON pair in the given directories (real recordings);
    /// reports, does not fail.
    Measure {
        dirs: Vec<PathBuf>,
        /// Matching window in ms (score-grid annotations need 50).
        #[arg(long, default_value_t = 30.0)]
        window_ms: f64,
        /// Pass the audio through the laptop-jack simulation first.
        #[arg(long)]
        laptop_jack: bool,
        /// Count a note as found only with the right rounded pitch.
        #[arg(long)]
        pitch: bool,
        /// Fail if total recall (percent) falls below this (regression guard in CI).
        #[arg(long)]
        min_recall: Option<f64>,
        /// Fail if total precision (percent) falls below this.
        #[arg(long)]
        min_precision: Option<f64>,
        /// List misses and extra detections per file.
        #[arg(long)]
        verbose: bool,
    },
    /// How the techniques of real recordings are told apart (LH-BEW-06): per annotated
    /// technique, what the tracker said. Needs annotations with techniques (IDMT import).
    Techniques {
        dirs: Vec<PathBuf>,
        /// Matching window in ms (score-grid annotations need 50–80).
        #[arg(long, default_value_t = 80.0)]
        window_ms: f64,
    },
}

#[derive(Debug, Args)]
pub struct KorpusArgs {
    #[command(subcommand)]
    pub command: KorpusCommand,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum ModeArg {
    Play,
    Song,
    Wait,
    Workshop,
}

#[derive(Debug, Args)]
pub struct PracticeArgs {
    pub score: PathBuf,
    #[arg(long)]
    pub track: Option<usize>,
    #[arg(long, value_enum, default_value = "play")]
    pub mode: ModeArg,
    #[arg(long, default_value_t = 1.0)]
    pub speed: f64,
    /// Loop bars, e.g. `5-8` (1-based, inclusive).
    #[arg(long)]
    pub r#loop: Option<String>,
    /// Workshop: target speed.
    #[arg(long, default_value_t = 1.0)]
    pub target: f64,
    /// Use a recording instead of the audio input (QA journeys).
    #[arg(long)]
    pub input_file: Option<PathBuf>,
    /// With --input-file: simulate a laptop combo jack.
    #[arg(long)]
    pub laptop_jack: bool,
    /// Backing track audio file.
    #[arg(long)]
    pub backing: Option<PathBuf>,
    /// Seconds into the backing file where the score starts.
    #[arg(long, default_value_t = 0.0)]
    pub backing_offset: f64,
    #[arg(long)]
    pub no_metronome: bool,
    /// Play the expected notes as reference tones.
    #[arg(long)]
    pub reference: bool,
    #[arg(long, default_value_t = 1)]
    pub count_in: u32,
    #[arg(long, default_value_t = 0.0)]
    pub latency_ms: f64,
    /// Input device id (see `devices`).
    #[arg(long)]
    pub input: Option<String>,
    /// Output device id (see `devices`); with `--input-file` silent unless given.
    #[arg(long)]
    pub output: Option<String>,
}

#[derive(Debug, Args)]
pub struct TuneArgs {
    /// Use a recording instead of the audio input.
    #[arg(long)]
    pub input_file: Option<PathBuf>,
    #[arg(long, default_value_t = 5)]
    pub strings: u8,
    /// Seconds to run.
    #[arg(long, default_value_t = 30.0)]
    pub seconds: f64,
    #[arg(long)]
    pub input: Option<String>,
}

#[derive(Debug, Args)]
pub struct RenderArgs {
    pub score: PathBuf,
    pub out: PathBuf,
    #[arg(long)]
    pub track: Option<usize>,
    #[arg(long, default_value_t = 1.0)]
    pub speed: f64,
    /// Silence before the first bar, e.g. for a count-in of one 4/4 bar at 120 BPM: 2.0.
    #[arg(long, default_value_t = 0.0)]
    pub lead_in: f64,
    /// Play every note this many ms late (negative = early).
    #[arg(long, default_value_t = 0.0, allow_negative_numbers = true)]
    pub offset_ms: f64,
    #[arg(long, default_value_t = 0)]
    pub wrong_every: usize,
    #[arg(long, default_value_t = 0)]
    pub miss_every: usize,
    #[arg(long, default_value_t = 0.0, allow_negative_numbers = true)]
    pub detune_cents: f64,
    /// Play the score's techniques (slides, hammer-ons, slaps, ghost notes, LH-BEW-06).
    #[arg(long)]
    pub techniques: bool,
    /// With `--techniques`: every n-th technique note plucked instead (0 = never).
    #[arg(long, default_value_t = 0)]
    pub plain_every: usize,
}
