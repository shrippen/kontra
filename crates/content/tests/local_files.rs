//! Imports every file in `testdata/import/local/` (not in git, see `testdata/import/README.md`).
//! Run with `cargo test -p kontra-content --test local_files -- --ignored --nocapture`.

// Test helpers outside #[test] functions may panic too.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use kontra_content::{import_file, ImportError};
use std::path::PathBuf;

fn local_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../testdata/import/local")
}

#[test]
#[ignore = "needs real-world files in testdata/import/local/"]
fn imports_local_files() {
    let Ok(entries) = std::fs::read_dir(local_dir()) else {
        println!("no local files");
        return;
    };
    let mut paths: Vec<PathBuf> = entries.filter_map(|e| e.ok().map(|e| e.path())).collect();
    paths.sort();

    let mut failures = Vec::new();
    for path in paths.iter().filter(|p| p.is_file()) {
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        if name.ends_with(".md") {
            continue;
        }
        let result = import_file(path);
        let imported = match result {
            Ok(imported) => imported,
            Err(ImportError::Empty) => {
                println!("empty {name}");
                continue;
            }
            Err(error) => {
                println!("FAIL {name}: {error}");
                failures.push(name);
                continue;
            }
        };
        let mut line = format!("ok   {name}:");
        for (index, track) in imported.tracks().iter().enumerate() {
            match imported.arrangement(index) {
                Ok(a) => line.push_str(&format!(
                    " [{}{} {}s {}n {}b]",
                    track.name(),
                    if Some(index) == imported.suggested() {
                        "*"
                    } else {
                        ""
                    },
                    track.strings(),
                    a.notes.len(),
                    a.bars.len()
                )),
                Err(error) => {
                    line.push_str(&format!(" [{}: {error}]", track.name()));
                    failures.push(format!("{name} track {index}"));
                }
            }
        }
        println!("{line}");
    }
    assert!(failures.is_empty(), "failed: {failures:?}");
}

#[test]
#[ignore = "needs real-world files in testdata/import/local/"]
fn local_files_survive_corruption() {
    let Ok(entries) = std::fs::read_dir(local_dir()) else {
        return;
    };
    let mut state = 0x2545_f491_4f6c_dd1du64;
    for path in entries.filter_map(|e| e.ok().map(|e| e.path())) {
        let Ok(data) = std::fs::read(&path) else {
            continue;
        };
        let Some(kind) = path
            .extension()
            .and_then(|e| e.to_str())
            .and_then(kontra_content::FileKind::from_extension)
        else {
            continue;
        };
        for round in 0..ROUNDS {
            let mut damaged = data.clone();
            for _ in 0..=round % 4 {
                state ^= state << 13;
                state ^= state >> 7;
                state ^= state << 17;
                let at = (state % damaged.len().max(1) as u64) as usize;
                if let Some(byte) = damaged.get_mut(at) {
                    *byte = (state >> 32) as u8;
                }
            }
            if round % 10 == 0 {
                damaged.truncate(damaged.len() * round / ROUNDS);
            }
            if let Ok(imported) = kontra_content::import_bytes(&damaged, kind) {
                for track in 0..imported.tracks().len() {
                    let _ = imported.arrangement(track);
                }
            }
        }
    }
}

/// Corruptions per local file.
const ROUNDS: usize = 200;
