//! Command line options of the app (QA runs, screenshots).

use std::path::PathBuf;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct AppArgs {
    pub open: Option<PathBuf>,
    pub page: Option<String>,
    pub input_file: Option<PathBuf>,
    pub laptop_jack: bool,
    pub autoplay: bool,
    /// `--probe`: start a blind probe of the `--loop-bars` (screenshots, DIDAKTIK I1).
    pub probe: bool,
    pub screenshot: Option<PathBuf>,
    pub screenshot_after_ms: i32,
    pub quit_after_ms: i32,
    pub style: Option<String>,
    pub language: Option<String>,
    pub mode: Option<String>,
    pub view: Option<String>,
    pub calibrate: bool,
    /// `--mixer`: practice and settings open with the mixer in view (screenshots).
    pub mixer: bool,
    /// `--present`: the practice page opens in presentation mode (screenshots, LH-ANZ-09).
    pub present: bool,
    /// `--section data|about|make`: settings or exercises open scrolled to that section
    /// (screenshots).
    pub section: Option<String>,
    /// `--align`: practice opens the backing-track alignment (screenshots).
    pub align: bool,
    /// `--loop-bars 3-4`: loop these bars (1-based) before autoplay.
    pub loop_bars: Option<(i32, i32)>,
    /// `--remove-bass`: the backing track plays without its bass, separated first if needed
    /// (screenshots of J-D1).
    pub remove_bass: bool,
    /// `--songsterr-answer FILE`: the Songsterr search answers with this file, never the network
    /// (screenshots of J-B6).
    pub songsterr_answer: Option<PathBuf>,
    /// `--song-search QUERY`: search Songsterr at start (screenshots of J-B6).
    pub song_search: Option<String>,
    /// `--course-answer FILE`: every course address answers with this file, never the network
    /// (QA of J-G5).
    pub course_answer: Option<PathBuf>,
    /// `--help` or `-h`: print `USAGE` and quit, never open the audio input (QA 27.10.).
    pub help: bool,
    /// The first `--option` Kontra does not know: an error instead of a start with real audio.
    pub unknown: Option<String>,
}

/// What `--help` prints. Options for QA runs and screenshots; Qt's own options pass through.
pub const USAGE: &str = "\
Usage: kontra [FILE] [OPTIONS]

  FILE                     a piece (Guitar Pro, MusicXML, MuseScore, MIDI) or a .kontrakurs
  --language de|en         language of the interface
  --theme kante|kante-light|system
  --page NAME              open this page (home, library, practice, settings, tuner, ear, ...)
  --open FILE              open this piece
  --mode play|song|wait|workshop
  --view lane|tab|notation|combined|keys
  --input-file WAV         a recording instead of the audio input (QA)
  --input-profile laptop-jack
  --autoplay, --probe, --calibrate, --mixer, --present, --align, --remove-bass
  --loop-bars A-B, --section NAME
  --screenshot PNG, --screenshot-after MS, --quit-after MS
  --songsterr-answer FILE, --song-search TEXT, --course-answer FILE
  --focus-log, --screenshot-notes
  -h, --help               this text
";

impl AppArgs {
    pub fn from_env() -> AppArgs {
        AppArgs::parse(std::env::args().skip(1))
    }

    pub fn parse(args: impl Iterator<Item = String>) -> AppArgs {
        let mut out = AppArgs {
            screenshot_after_ms: 1500,
            ..AppArgs::default()
        };
        let mut args = args.peekable();
        while let Some(arg) = args.next() {
            let arg = single_dash(arg);
            let mut value = || args.next().unwrap_or_default();
            match arg.as_str() {
                "--open" => out.open = Some(PathBuf::from(value())),
                "--page" => out.page = Some(value()),
                "--input-file" => out.input_file = Some(PathBuf::from(value())),
                "--input-profile" => out.laptop_jack = value() == "laptop-jack",
                "--autoplay" => out.autoplay = true,
                "--probe" => out.probe = true,
                "--screenshot" => out.screenshot = Some(PathBuf::from(value())),
                "--screenshot-after" => out.screenshot_after_ms = value().parse().unwrap_or(1500),
                "--quit-after" => out.quit_after_ms = value().parse().unwrap_or(0),
                // Not "--style": QGuiApplication takes that for a Qt Quick Controls style and
                // fails to load the UI (QA round 9.10.2026).
                "--theme" => out.style = Some(value()),
                "--language" => out.language = Some(value()),
                "--mode" => out.mode = Some(value()),
                "--view" => out.view = Some(value()),
                "--calibrate" => out.calibrate = true,
                "--mixer" => out.mixer = true,
                "--present" => out.present = true,
                "--section" => out.section = Some(value()),
                "--align" => out.align = true,
                "--remove-bass" => out.remove_bass = true,
                "--songsterr-answer" => out.songsterr_answer = Some(PathBuf::from(value())),
                "--song-search" => out.song_search = Some(value()),
                "--course-answer" => out.course_answer = Some(PathBuf::from(value())),
                "--loop-bars" => {
                    let v = value();
                    let mut parts = v.split('-').map(|p| p.trim().parse::<i32>());
                    if let (Some(Ok(a)), b) = (parts.next(), parts.next()) {
                        let b = b.and_then(Result::ok).unwrap_or(a);
                        out.loop_bars = Some((a.min(b), a.max(b)));
                    }
                }
                // Qt's own options with a value (QGuiApplication reads them too): the value is
                // not a file to open.
                other if QT_VALUED.contains(&other) => {
                    let _ = value();
                }
                "--help" | "-h" => out.help = true,
                // Read by Main.qml (QA): known, nothing to keep here.
                "--focus-log" | "--screenshot-notes" => {}
                // A file from the file manager (desktop file `%f`).
                other if !other.starts_with('-') && out.open.is_none() => {
                    out.open = Some(PathBuf::from(other));
                }
                other if other.starts_with("--") && out.unknown.is_none() => {
                    out.unknown = Some(other.to_string());
                }
                _ => {}
            }
        }
        out
    }
}

/// Qt's own options that take a value.
const QT_VALUED: [&str; 10] = [
    "-qwindowgeometry",
    "-geometry",
    "-platform",
    "-platformtheme",
    "-plugin",
    "-qwindowtitle",
    "-qwindowicon",
    "-style",
    "-stylesheet",
    "-display",
];

/// Qt's own options without a value.
const QT_FLAGS: [&str; 5] = ["-reverse", "-nograb", "-dograb", "-sync", "-widgetcount"];

/// Qt reads its options with two dashes too ("--platform offscreen"): those become the
/// one-dash form, so they pass like Qt's and are not unknown.
fn single_dash(arg: String) -> String {
    let Some(short) = arg.strip_prefix('-') else {
        return arg;
    };
    if !short.starts_with('-') {
        return arg;
    }
    if QT_VALUED.contains(&short) || QT_FLAGS.contains(&short) {
        return short.to_string();
    }
    arg
}

#[cfg(test)]
mod tests {
    use super::*;

    // QA 27.10.: "kontra --help" started the app with the real audio input.
    #[test]
    fn help_and_unknown_options_do_not_start_the_app() {
        let parse = |a: &[&str]| AppArgs::parse(a.iter().map(|s| s.to_string()));
        assert!(parse(&["--help"]).help);
        assert!(parse(&["-h"]).help);
        assert_eq!(parse(&["--bogus"]).unknown, Some("--bogus".into()));
        assert_eq!(parse(&["--language", "de"]).unknown, None);
        assert_eq!(parse(&["--focus-log"]).unknown, None);
        // Qt's own options pass, with one dash or two; their values are no file.
        assert_eq!(parse(&["-reverse"]).unknown, None);
        let qt = parse(&["--platform", "offscreen", "--reverse", "song.gp5"]);
        assert_eq!(qt.unknown, None);
        assert_eq!(qt.open, Some(PathBuf::from("song.gp5")));
    }

    #[test]
    fn theme_is_not_qts_style_option() {
        let parse = |a: &[&str]| AppArgs::parse(a.iter().map(|s| s.to_string())).style;
        assert_eq!(
            parse(&["--theme", "kante-light"]),
            Some("kante-light".into())
        );
        assert_eq!(parse(&["--style", "kante-light"]), None);
    }

    // QA 14.10.: "-qwindowgeometry 1280x800" opened a file "1280x800" (an error on every start).
    #[test]
    fn qt_options_keep_their_values() {
        let args = [
            "-qwindowgeometry",
            "1280x800+0+0",
            "-platform",
            "xcb",
            "song.gp5",
        ];
        let parsed = AppArgs::parse(args.iter().map(|s| s.to_string()));
        assert_eq!(parsed.open, Some(PathBuf::from("song.gp5")));
    }

    #[test]
    fn a_bare_path_opens_the_file() {
        let args = AppArgs::parse(["song.gp5".to_string()].into_iter());
        assert_eq!(args.open, Some(PathBuf::from("song.gp5")));
        let args = AppArgs::parse(["--open", "a.mid", "b.mid"].iter().map(|s| s.to_string()));
        assert_eq!(args.open, Some(PathBuf::from("a.mid")));
    }

    #[test]
    fn parses_loop_bars() {
        let parse =
            |v: &str| AppArgs::parse(["--loop-bars", v].iter().map(|s| s.to_string())).loop_bars;
        assert_eq!(parse("3-4"), Some((3, 4)));
        assert_eq!(parse("5"), Some((5, 5)));
        assert_eq!(parse("4-2"), Some((2, 4)));
        assert_eq!(parse("x"), None);
    }

    #[test]
    fn parses_qa_options() {
        let args = [
            "--input-file",
            "take.wav",
            "--input-profile",
            "laptop-jack",
            "--autoplay",
            "--page",
            "practice",
            "--mixer",
            "--probe",
        ];
        let a = AppArgs::parse(args.iter().map(|s| s.to_string()));
        assert_eq!(a.input_file, Some(PathBuf::from("take.wav")));
        assert!(a.laptop_jack && a.autoplay && a.mixer && a.probe);
        assert_eq!(a.page.as_deref(), Some("practice"));
        assert_eq!(a.screenshot_after_ms, 1500);
    }
}
