//! `kontra --probe-plugin <file>`: the plugin search starts this program once per plugin file,
//! so a plugin that crashes or hangs ends only that process (LH-EIN-20). Handled first in
//! `main`, before Qt and audio start. Packages ship only `kontra`, so no second program is needed.

use kontra_appcore::fx_plugins::{probe_plugin_line, PROBE_FLAG};
use std::ffi::OsString;
use std::path::PathBuf;

/// The file to probe, when `args` (program name first) are a probe request.
fn request(args: impl IntoIterator<Item = OsString>) -> Option<PathBuf> {
    let mut args = args.into_iter().skip(1);
    if args.next()? != PROBE_FLAG {
        return None;
    }
    args.next().map(PathBuf::from)
}

/// Answers a probe request with one line on stdout; false if the command line is no such request.
pub(crate) fn answer() -> bool {
    let Some(file) = request(std::env::args_os()) else {
        return false;
    };
    println!("{}", probe_plugin_line(&file));
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_the_flag_first_makes_a_request() {
        let args = |a: &[&str]| a.iter().map(OsString::from).collect::<Vec<_>>();
        let file = request(args(&["kontra", "--probe-plugin", "/a/b.clap"]));
        assert_eq!(file, Some(PathBuf::from("/a/b.clap")));
        assert_eq!(request(args(&["kontra", "--probe-plugin"])), None);
        assert_eq!(request(args(&["kontra", "--page", "tuner"])), None);
        assert_eq!(
            request(args(&["kontra", "x.mid", "--probe-plugin", "f"])),
            None
        );
    }
}
