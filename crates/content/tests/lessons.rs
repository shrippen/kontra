//! The bundled lessons (`lessons/`, LH-LRN-07 to -14): readable, consistent, video-free.

// Test helpers outside #[test] functions may panic too.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use kontra_content::exercise;
use kontra_content::lesson::{Block, ExerciseRef, Lessons, Practice};

fn has_video(blocks: &[Block]) -> bool {
    blocks.iter().any(|b| match b {
        Block::Video { .. } => true,
        Block::Variant { blocks, .. } => has_video(blocks),
        _ => false,
    })
}

#[test]
fn bundled_lessons_fit_together() {
    let lessons = Lessons::bundled().unwrap();
    assert_eq!(lessons.check(), Vec::<String>::new());
    assert!(lessons.courses.iter().any(|c| c.id == "bass"));
}

/// Bundled courses never carry videos; only loaded ones may (owner, 25.10.2026).
#[test]
fn bundled_lessons_have_no_video() {
    let lessons = Lessons::bundled().unwrap();
    let docs = lessons
        .units
        .iter()
        .flat_map(|u| u.texts.values())
        .chain(lessons.terms.iter().flat_map(|t| t.texts.values()));
    for doc in docs {
        for page in &doc.pages {
            assert!(!has_video(&page.blocks), "video in {}", doc.title);
        }
    }
}

fn practices<'a>(blocks: &'a [Block], out: &mut Vec<&'a Practice>) {
    for block in blocks {
        match block {
            Block::Practice(p) => out.push(p),
            Block::Variant { blocks, .. } => practices(blocks, out),
            _ => {}
        }
    }
}

/// Every bundled exercise a lesson or finale names exists in the plan.
#[test]
fn practice_names_existing_exercises() {
    let lessons = Lessons::bundled().unwrap();
    let plan = exercise::bundled_specs().len();
    let mut all = Vec::new();
    for unit in &lessons.units {
        for doc in unit.texts.values() {
            doc.pages
                .iter()
                .for_each(|p| practices(&p.blocks, &mut all));
        }
    }
    let finales = lessons.courses.iter().flat_map(|c| &c.chapters);
    all.extend(finales.filter_map(|c| c.finale.as_ref()));

    assert!(!all.is_empty());
    for practice in all {
        if let ExerciseRef::Bundled(n) = practice.exercise {
            assert!(
                (1..=plan).contains(&usize::from(n)),
                "exercise {n} not in the plan"
            );
        }
    }
}
