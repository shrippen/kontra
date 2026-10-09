//! Frame sizes derived from the sample rate.

/// Analysis window length in seconds (4096 samples at 44.1/48 kHz).
/// Long enough to resolve bass partials 41 Hz apart (bin ≈ 11 Hz).
const WIN_SECONDS: f64 = 0.085;

/// Synthesis overlap: Hann² sums flat at 4x, cost stays at one FFT pair
/// per channel pair and hop; transients are handled separately.
const OVERLAP: usize = 4;

/// Onset detector window relative to the main window (1024 at 48 kHz).
const DET_RATIO: usize = 4;

/// Onset detector overlap (hop 256 at 48 kHz, 5 ms).
const DET_OVERLAP: usize = 4;

/// Smallest and largest window exponent (1024 … 32768 samples).
const WIN_LOG2_MIN: i32 = 10;
const WIN_LOG2_MAX: i32 = 15;

#[derive(Debug, Clone, Copy)]
pub(crate) struct Sizes {
    /// Main window (FFT size).
    pub(crate) win: usize,
    /// Synthesis hop.
    pub(crate) hop: usize,
    /// Onset detector window and hop.
    pub(crate) det_win: usize,
    pub(crate) det_hop: usize,
    /// Extra lookahead so the planner sees onsets early enough to steer
    /// the drift before a transient run.
    pub(crate) lead: usize,
}

impl Sizes {
    pub(crate) fn new(sample_rate: u32) -> Self {
        let ideal = f64::from(sample_rate.max(1)) * WIN_SECONDS;
        let exp = (ideal.log2().round() as i32).clamp(WIN_LOG2_MIN, WIN_LOG2_MAX);
        let win = 1usize << exp;
        let det_win = win / DET_RATIO;

        Self {
            win,
            hop: win / OVERLAP,
            det_win,
            det_hop: det_win / DET_OVERLAP,
            lead: win / 2,
        }
    }

    pub(crate) fn bins(&self) -> usize {
        self.win / 2 + 1
    }

    pub(crate) fn det_bins(&self) -> usize {
        self.det_win / 2 + 1
    }

    /// Input needed beyond a frame start before that frame may run:
    /// the frame, the steering lead and one confirmed detector step.
    pub(crate) fn lookahead(&self) -> usize {
        self.win + self.lead + self.det_win + self.det_hop
    }
}
