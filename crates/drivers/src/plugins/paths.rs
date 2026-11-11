//! Where plugin files live (CLAP spec, "Search paths").
//!
//! ```text
//! Linux    $CLAP_PATH (':'-separated), ~/.clap, /usr/lib/clap
//! Windows  %CLAP_PATH% (';'), %COMMONPROGRAMFILES%\CLAP, %LOCALAPPDATA%\Programs\Common\CLAP
//! VST3     ~/.vst3, /usr/lib/vst3, /usr/local/lib/vst3
//! Flatpak  also the plugin extension point /app/extensions/Plugins/{clap,vst3}
//!          (org.freedesktop.LinuxAudio.Plugins.*; the host's /usr is not visible there)
//!          Windows: %COMMONPROGRAMFILES%\VST3, %LOCALAPPDATA%\Programs\Common\VST3
//!          (bundles `*.vst3` are folders)
//! ```

use super::PluginFormat;
#[cfg(windows)]
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

const CLAP_PATH_ENV: &str = "CLAP_PATH";
/// Where Flatpak mounts the plugin extensions (`add-extensions` in the manifest).
const FLATPAK_PLUGINS: &str = "/app/extensions/Plugins";
/// Plugin folders nest ("vendor/plugin.clap"); deeper than this is not a plugin folder.
const MAX_DEPTH: usize = 6;

/// The folders to search, in the order of the spec, without duplicates.
pub fn search_paths(format: PluginFormat) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    match format {
        PluginFormat::Clap => clap_paths(&mut paths),
        PluginFormat::Vst3 => vst3_system_paths(&mut paths),
    }
    if crate::in_sandbox() {
        paths.push(extension_dir(format));
    }
    let mut unique: Vec<PathBuf> = Vec::new();
    for p in paths {
        if !unique.contains(&p) {
            unique.push(p);
        }
    }
    unique
}

fn clap_paths(paths: &mut Vec<PathBuf>) {
    if let Some(list) = std::env::var_os(CLAP_PATH_ENV) {
        paths.extend(std::env::split_paths(&list).filter(|p| !p.as_os_str().is_empty()));
    }
    clap_system_paths(paths);
}

/// The folder of the Flatpak plugin extensions for `format`.
fn extension_dir(format: PluginFormat) -> PathBuf {
    Path::new(FLATPAK_PLUGINS).join(format.key())
}

#[cfg(not(windows))]
fn clap_system_paths(paths: &mut Vec<PathBuf>) {
    if let Some(home) = std::env::var_os("HOME") {
        paths.push(Path::new(&home).join(".clap"));
    }
    paths.push(PathBuf::from("/usr/lib/clap"));
}

#[cfg(windows)]
fn clap_system_paths(paths: &mut Vec<PathBuf>) {
    let join = |var: &str, rest: &str| -> Option<PathBuf> {
        let base: OsString = std::env::var_os(var)?;
        Some(Path::new(&base).join(rest))
    };
    paths.extend(join("COMMONPROGRAMFILES", "CLAP"));
    paths.extend(join("LOCALAPPDATA", r"Programs\Common\CLAP"));
}

#[cfg(not(windows))]
fn vst3_system_paths(paths: &mut Vec<PathBuf>) {
    if let Some(home) = std::env::var_os("HOME") {
        paths.push(Path::new(&home).join(".vst3"));
    }
    paths.push(PathBuf::from("/usr/lib/vst3"));
    paths.push(PathBuf::from("/usr/local/lib/vst3"));
}

#[cfg(windows)]
fn vst3_system_paths(paths: &mut Vec<PathBuf>) {
    let join = |var: &str, rest: &str| -> Option<PathBuf> {
        let base: OsString = std::env::var_os(var)?;
        Some(Path::new(&base).join(rest))
    };
    paths.extend(join("COMMONPROGRAMFILES", "VST3"));
    paths.extend(join("LOCALAPPDATA", r"Programs\Common\VST3"));
}

/// All plugin files in the search paths, sorted.
pub fn plugin_files(format: PluginFormat) -> Vec<PathBuf> {
    let mut files = Vec::new();
    for dir in search_paths(format) {
        collect(format, &dir, 0, &mut files);
    }
    files.sort();
    files.dedup();
    files
}

fn collect(format: PluginFormat, dir: &Path, depth: usize, files: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        // Follows symlinks: ~/.clap often links to a vendor's folder.
        let Ok(meta) = fs::metadata(&path) else {
            continue;
        };
        // A VST3 bundle is a folder; it is the plugin, not a folder to search.
        let bundle = format == PluginFormat::Vst3 && PluginFormat::of_file(&path) == Some(format);
        if meta.is_dir() && bundle {
            files.push(path);
            continue;
        }
        if meta.is_dir() {
            if depth < MAX_DEPTH {
                collect(format, &path, depth + 1, files);
            }
            continue;
        }
        if PluginFormat::of_file(&path) == Some(format) && meta.is_file() {
            files.push(path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flatpak_extensions_have_a_folder_per_format() {
        let dir = |f| extension_dir(f).to_string_lossy().replace('\\', "/");
        assert_eq!(dir(PluginFormat::Clap), "/app/extensions/Plugins/clap");
        assert_eq!(dir(PluginFormat::Vst3), "/app/extensions/Plugins/vst3");
    }

    #[test]
    fn finds_clap_files_in_nested_folders() {
        let dir = std::env::temp_dir().join(format!("kontra-paths-{}", std::process::id()));
        let nested = dir.join("vendor");
        fs::create_dir_all(&nested).unwrap();
        fs::write(dir.join("a.clap"), b"").unwrap();
        fs::write(nested.join("b.CLAP"), b"").unwrap();
        fs::write(nested.join("c.txt"), b"").unwrap();

        let mut files = Vec::new();
        collect(PluginFormat::Clap, &dir, 0, &mut files);
        files.sort();
        fs::remove_dir_all(&dir).unwrap();

        assert_eq!(files.len(), 2);
        assert!(files[0].ends_with("a.clap"));
        assert!(files[1].ends_with("b.CLAP"));
    }

    #[test]
    fn vst3_bundle_is_one_hit_and_not_searched() {
        let dir = std::env::temp_dir().join(format!("kontra-vst3-paths-{}", std::process::id()));
        let inside = dir.join("Foo.vst3").join("Contents").join("x86_64-linux");
        fs::create_dir_all(&inside).unwrap();
        fs::write(inside.join("Foo.so"), b"").unwrap();
        fs::write(dir.join("Single.vst3"), b"").unwrap();
        fs::write(dir.join("a.clap"), b"").unwrap();

        let mut files = Vec::new();
        collect(PluginFormat::Vst3, &dir, 0, &mut files);
        files.sort();
        fs::remove_dir_all(&dir).unwrap();

        assert_eq!(files.len(), 2);
        assert!(files[0].ends_with("Foo.vst3"));
        assert!(files[1].ends_with("Single.vst3"));
    }
}
