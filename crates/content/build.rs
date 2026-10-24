//! Embeds the bundled lessons (`lessons/` at the repository root) as `FILES` for `lesson.rs`:
//! (path below `lessons/`, text), sorted by path so the build is reproducible.

use std::path::{Path, PathBuf};
use std::{env, fs, io};

/// Lesson files are Markdown and JSON; everything else (README, media) stays out.
const EXTENSIONS: [&str; 2] = ["md", "json"];
const README: &str = "README.md";

fn main() -> io::Result<()> {
    let manifest = PathBuf::from(env::var("CARGO_MANIFEST_DIR").map_err(io::Error::other)?);
    let root = manifest.join("../../lessons");
    println!("cargo::rerun-if-changed={}", root.display());

    let mut files = Vec::new();
    collect(&root, &root, &mut files)?;
    files.sort();

    // One entry per file: ("units/septime/de.md", include_str!("/abs/…")).
    let mut out = String::from("static FILES: &[(&str, &str)] = &[\n");
    for (relative, absolute) in files {
        out.push_str(&format!(
            "    ({relative:?}, include_str!({absolute:?})),\n"
        ));
    }
    out.push_str("];\n");

    let target = PathBuf::from(env::var("OUT_DIR").map_err(io::Error::other)?).join("lessons.rs");
    fs::write(target, out)
}

fn collect(root: &Path, dir: &Path, files: &mut Vec<(String, String)>) -> io::Result<()> {
    if !dir.exists() {
        return Ok(());
    }
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            collect(root, &path, files)?;
            continue;
        }
        let wanted = path
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| EXTENSIONS.contains(&e));
        if !wanted || path.file_name().is_some_and(|n| n == README) {
            continue;
        }
        let relative = path.strip_prefix(root).map_err(io::Error::other)?;
        let relative = relative.to_string_lossy().replace('\\', "/");
        let absolute = path.canonicalize()?.to_string_lossy().into_owned();
        files.push((relative, absolute));
    }
    Ok(())
}
