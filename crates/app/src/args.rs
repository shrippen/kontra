//! Command line options of the app (QA runs, screenshots).

use std::path::PathBuf;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct AppArgs {
    pub open: Option<PathBuf>,
    pub page: Option<String>,
    pub input_file: Option<PathBuf>,
    pub laptop_jack: bool,
    pub autoplay: bool,
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
    /// `--section data|about`: the settings page opens scrolled to that section (screenshots).
    pub section: Option<String>,
    /// `--align`: practice opens the backing-track alignment (screenshots).
    pub align: bool,
    /// `--loop-bars 3-4`: loop these bars (1-based) before autoplay.
    pub loop_bars: Option<(i32, i32)>,
}

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
            let mut value = || args.next().unwrap_or_default();
            match arg.as_str() {
                "--open" => out.open = Some(PathBuf::from(value())),
                "--page" => out.page = Some(value()),
                "--input-file" => out.input_file = Some(PathBuf::from(value())),
                "--input-profile" => out.laptop_jack = value() == "laptop-jack",
                "--autoplay" => out.autoplay = true,
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
                "--loop-bars" => {
                    let v = value();
                    let mut parts = v.split('-').map(|p| p.trim().parse::<i32>());
                    if let (Some(Ok(a)), b) = (parts.next(), parts.next()) {
                        let b = b.and_then(Result::ok).unwrap_or(a);
                        out.loop_bars = Some((a.min(b), a.max(b)));
                    }
                }
                // A file from the file manager (desktop file `%f`).
                other if !other.starts_with('-') && out.open.is_none() => {
                    out.open = Some(PathBuf::from(other));
                }
                _ => {}
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn theme_is_not_qts_style_option() {
        let parse = |a: &[&str]| AppArgs::parse(a.iter().map(|s| s.to_string())).style;
        assert_eq!(
            parse(&["--theme", "kante-light"]),
            Some("kante-light".into())
        );
        assert_eq!(parse(&["--style", "kante-light"]), None);
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
        ];
        let a = AppArgs::parse(args.iter().map(|s| s.to_string()));
        assert_eq!(a.input_file, Some(PathBuf::from("take.wav")));
        assert!(a.laptop_jack && a.autoplay && a.mixer);
        assert_eq!(a.page.as_deref(), Some("practice"));
        assert_eq!(a.screenshot_after_ms, 1500);
    }
}
