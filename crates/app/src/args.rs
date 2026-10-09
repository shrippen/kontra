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
                "--style" => out.style = Some(value()),
                "--language" => out.language = Some(value()),
                "--mode" => out.mode = Some(value()),
                "--view" => out.view = Some(value()),
                "--calibrate" => out.calibrate = true,
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
    fn parses_qa_options() {
        let args = [
            "--input-file",
            "take.wav",
            "--input-profile",
            "laptop-jack",
            "--autoplay",
            "--page",
            "practice",
        ];
        let a = AppArgs::parse(args.iter().map(|s| s.to_string()));
        assert_eq!(a.input_file, Some(PathBuf::from("take.wav")));
        assert!(a.laptop_jack && a.autoplay);
        assert_eq!(a.page.as_deref(), Some("practice"));
        assert_eq!(a.screenshot_after_ms, 1500);
    }
}
