//! `course new` / `course check` / `course pack` (LH-LRN-15): what the course workshop does,
//! without a window, for a provider's own pipeline.

use crate::args::{CourseArgs, CourseCommand};
use crate::CliResult;
use kontra_session::lesson::{self, Author, CoursePack, Lessons, MAX_COURSE_BYTES};
use std::path::Path;

pub fn run(a: &CourseArgs) -> CliResult {
    match &a.command {
        CourseCommand::New {
            dir,
            id,
            provider,
            license,
        } => {
            let author = Author {
                provider: provider.clone(),
                license: license.clone(),
            };
            lesson::new_course(dir, id, &author).map_err(|e| e.to_string())?;
            println!("{}: new course {id}", dir.display());
            Ok(())
        }
        CourseCommand::Check { path } => check(&read(path)?),
        CourseCommand::Pack { dir, out } => {
            check(&read(dir)?)?;
            let data = lesson::pack(dir).map_err(|e| e.to_string())?;
            std::fs::write(out, data).map_err(|e| format!("{}: {e}", out.display()))?;
            println!("{}: packed", out.display());
            Ok(())
        }
    }
}

/// A folder as the author keeps it, or a packed file.
fn read(path: &Path) -> Result<CoursePack, String> {
    if path.is_dir() {
        return CoursePack::from_dir(path).map_err(|e| e.to_string());
    }
    // Size first: a video picked by mistake is refused before it is read.
    let size = std::fs::metadata(path)
        .map_err(|e| format!("{}: {e}", path.display()))?
        .len();
    if size > MAX_COURSE_BYTES {
        return Err(format!(
            "{}: {size} bytes, more than a course",
            path.display()
        ));
    }
    let data = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    CoursePack::read(&data).map_err(|e| e.to_string())
}

/// Prints every problem; fails if there is one.
fn check(pack: &CoursePack) -> CliResult {
    let bundled = Lessons::bundled().map_err(|e| e.to_string())?;
    let problems = pack.problems(&bundled);
    for p in &problems {
        println!("{p}");
    }
    if !problems.is_empty() {
        return Err(format!("{} problems", problems.len()));
    }
    let units = pack.lessons.units.len();
    println!("{}: {units} lessons, no problems", pack.manifest.id);
    Ok(())
}
