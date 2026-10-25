//! Consistency of a lesson folder: both languages alike, every id known, needs before use, and
//! what a lesson needs or links reaches every instrument the lesson is for.

use super::{covers, known, Block, Doc, Lang, Lessons, Span};
use std::collections::{BTreeMap, BTreeSet};

/// All problems, one sentence each.
pub(super) fn problems(lessons: &Lessons) -> Vec<String> {
    let mut out = Vec::new();
    let term_ids: BTreeSet<&str> = lessons.terms.iter().map(|t| t.id.as_str()).collect();
    let term_scopes: BTreeMap<&str, &str> = lessons
        .terms
        .iter()
        .map(|t| (t.id.as_str(), t.scope.as_str()))
        .collect();

    // Units: both languages with the same structure; known needs and terms, as wide as the unit.
    for unit in &lessons.units {
        let name = format!("unit {}", unit.id);
        let scoped = format!("{name} ({})", unit.scope);
        if !known(&unit.scope) {
            out.push(format!("{name} is for unknown scope {}", unit.scope));
        }
        languages(&name, &unit.texts, &mut out);
        for need in &unit.needs {
            match lessons.unit(need) {
                None => out.push(format!("{name} needs unknown unit {need}")),
                Some(n) if !covers(&n.scope, &unit.scope) => {
                    out.push(format!("{scoped} needs unit {need} ({})", n.scope));
                }
                Some(_) => {}
            }
        }
        for term in &unit.terms {
            if !term_ids.contains(term.as_str()) {
                out.push(format!("{name} teaches unknown term {term}"));
            }
        }
        links(&name, &unit.texts, &term_ids, &mut out);
        reach(&scoped, &unit.scope, &unit.texts, &term_scopes, &mut out);
    }

    // Terms: both languages alike; the teaching unit exists.
    for term in &lessons.terms {
        let name = format!("term {}", term.id);
        languages(&name, &term.texts, &mut out);
        if let Some(lesson) = &term.lesson {
            if lessons.unit(lesson).is_none() {
                out.push(format!("{name} points to unknown unit {lesson}"));
            }
        }
        links(&name, &term.texts, &term_ids, &mut out);
    }

    // Courses: known units, each once, after the units they need.
    for course in &lessons.courses {
        let mut seen: BTreeSet<&str> = BTreeSet::new();
        for chapter in &course.chapters {
            for lang in Lang::ALL {
                if chapter.titles.get(&lang).is_none_or(|t| t.is_empty()) {
                    out.push(format!(
                        "course {} chapter {} lacks a title",
                        course.id, chapter.id
                    ));
                }
            }
            for id in &chapter.units {
                let Some(unit) = lessons.unit(id) else {
                    out.push(format!("course {} lists unknown unit {id}", course.id));
                    continue;
                };
                for need in &unit.needs {
                    if !seen.contains(need.as_str()) {
                        out.push(format!("course {}: {id} comes before {need}", course.id));
                    }
                }
                if !seen.insert(id) {
                    out.push(format!("course {} lists {id} twice", course.id));
                }
            }
        }
    }
    out
}

/// Every language present, and all with the same shape (pages, blocks, attributes, terms).
fn languages(name: &str, texts: &std::collections::BTreeMap<Lang, Doc>, out: &mut Vec<String>) {
    for lang in Lang::ALL {
        if !texts.contains_key(&lang) {
            out.push(format!("{name} has no {} text", lang.key()));
        }
    }
    let shapes: Vec<(Lang, String)> = texts.iter().map(|(l, d)| (*l, shape(d))).collect();
    for pair in shapes.windows(2) {
        if pair[0].1 != pair[1].1 {
            out.push(format!(
                "{name}: {} and {} differ ({} / {})",
                pair[0].0.key(),
                pair[1].0.key(),
                pair[0].1,
                pair[1].1
            ));
        }
    }
}

/// Term links point to glossary entries.
fn links(
    name: &str,
    texts: &std::collections::BTreeMap<Lang, Doc>,
    terms: &BTreeSet<&str>,
    out: &mut Vec<String>,
) {
    for doc in texts.values() {
        let mut ids = Vec::new();
        for page in &doc.pages {
            page.blocks.iter().for_each(|b| block_terms(b, &mut ids));
        }
        for id in ids {
            if !terms.contains(id.as_str()) {
                out.push(format!("{name} links unknown term {id}"));
            }
        }
    }
}

/// Linked terms hold wherever the text (or its variant) does: `[[bund]]` in a lesson for every
/// string instrument is wrong for the fretless bass, inside `::: variant for=fretted` it is right.
fn reach(
    name: &str,
    scope: &str,
    texts: &BTreeMap<Lang, Doc>,
    terms: &BTreeMap<&str, &str>,
    out: &mut Vec<String>,
) {
    for doc in texts.values() {
        let mut links = Vec::new();
        for page in &doc.pages {
            for block in &page.blocks {
                scoped_terms(block, scope, &mut links, out);
            }
        }
        for (id, at) in links {
            let Some(&wide) = terms.get(id.as_str()) else {
                continue;
            };
            if !covers(wide, &at) {
                out.push(format!("{name} links term {id} ({wide})"));
            }
        }
    }
}

/// Term ids linked in a block with the scope they appear under; unknown variant scopes noted.
fn scoped_terms(
    block: &Block,
    scope: &str,
    links: &mut Vec<(String, String)>,
    out: &mut Vec<String>,
) {
    if let Block::Variant {
        scope: inner,
        blocks,
    } = block
    {
        if !known(inner) {
            out.push(format!("variant for unknown scope {inner}"));
        }
        blocks
            .iter()
            .for_each(|b| scoped_terms(b, inner, links, out));
        return;
    }
    let mut ids = Vec::new();
    block_terms(block, &mut ids);
    links.extend(ids.into_iter().map(|id| (id, scope.to_string())));
}

/// The language-free structure of a text: what must match between de and en.
fn shape(doc: &Doc) -> String {
    let mut out = Vec::new();
    for page in &doc.pages {
        out.push("page".to_string());
        page.blocks.iter().for_each(|b| block_shape(b, &mut out));
    }
    out.join(" ")
}

fn block_shape(block: &Block, out: &mut Vec<String>) {
    let mut terms = Vec::new();
    block_terms(block, &mut terms);
    let shape = match block {
        Block::Text(_) => "text".to_string(),
        Block::List(items) => format!("list{}", items.len()),
        Block::Hint(_) => "hint".to_string(),
        Block::Figure(t) => format!("figure{t:?}"),
        Block::Listen(t) => format!("listen{t:?}"),
        Block::Try(t) => format!("try{t:?}"),
        Block::Practice(p) => format!("practice{p:?}"),
        Block::Question(q) => format!("question{}/{}", q.options.len(), q.answer),
        Block::Tuner => "tuner".to_string(),
        Block::Video { src } => format!("video:{src}"),
        Block::Variant { scope, blocks } => {
            out.push(format!("variant:{scope}("));
            blocks.iter().for_each(|b| block_shape(b, out));
            ")".to_string()
        }
    };
    out.push(shape);
    if !terms.is_empty() && !matches!(block, Block::Variant { .. }) {
        out.push(format!("[{}]", terms.join(",")));
    }
}

/// Term ids linked in a block (nested variants included).
fn block_terms(block: &Block, out: &mut Vec<String>) {
    let mut from = |spans: &[Span]| {
        for span in spans {
            if let Span::Term { id, .. } = span {
                out.push(id.clone());
            }
        }
    };
    match block {
        Block::Text(s) | Block::Hint(s) => from(s),
        Block::List(items) => items.iter().for_each(|i| from(i)),
        Block::Question(q) => {
            from(&q.prompt);
            q.options.iter().for_each(|o| from(o));
        }
        Block::Variant { blocks, .. } => blocks.iter().for_each(|b| block_terms(b, out)),
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::super::Lessons;

    const DE: &str = "# Intervall\nid: intervall\n\n## Hören\nEin [[quinte|Quinte]].\n::: listen root=A steps=\"0 7\"\n";
    const EN: &str = "# Interval\nid: intervall\n\n## Listen\nA [[quinte|fifth]].\n::: listen root=A steps=\"0 7\"\n";
    const SEPT: &str = "# Septime\nid: septime\nneeds: intervall\n\n## Hören\nText.\n";
    const GLOSSARY: &str = "## Quinte\nid: quinte\nlesson: intervall\nFünf.\n";
    const COURSE: &str = r#"{"id": "bass", "for": "bass", "chapters": [
        {"id": "a", "de": "A", "en": "A", "units": ["septime", "intervall"]}]}"#;

    #[test]
    fn finds_what_does_not_fit() {
        let files = [
            ("courses/bass.json", COURSE),
            ("units/intervall/de.md", DE),
            ("units/intervall/en.md", EN),
            ("units/septime/de.md", SEPT),
            ("glossary/de.md", GLOSSARY),
        ];
        let lessons = Lessons::parse(&files).unwrap();
        let problems = lessons.check();
        assert_eq!(
            problems,
            [
                "unit septime has no en text",
                "term quinte has no en text",
                "course bass: septime comes before intervall",
            ]
        );
    }

    /// Review #119: a lesson for every string instrument needed one for the e-bass only, and
    /// linked a term only true with frets; the double bass and fretless lost them.
    #[test]
    fn needs_and_terms_reach_as_far_as_the_lesson() {
        let unit = |id: &str, scope: &str, extra: &str| {
            format!("# {id}\nid: {id}\nfor: {scope}\n{extra}\n## P\nText.\n")
        };
        let teile = unit("teile", "e-bass", "");
        let stimmen = unit("stimmen", "strings", "needs: teile\n");
        let greifen = "# G\nid: greifen\nfor: strings\n\n## P\nDie [[bund]].\n::: variant for=fretted\nDer [[bund]].\n:::\n";
        let glossary = "## Bund\nid: bund\nfor: fretted\nText.\n";
        let files = [
            ("units/teile/de.md", teile.as_str()),
            ("units/teile/en.md", teile.as_str()),
            ("units/stimmen/de.md", stimmen.as_str()),
            ("units/stimmen/en.md", stimmen.as_str()),
            ("units/greifen/de.md", greifen),
            ("units/greifen/en.md", greifen),
            ("glossary/de.md", glossary),
            ("glossary/en.md", glossary),
        ];
        let problems = Lessons::parse(&files).unwrap().check();
        assert_eq!(
            problems,
            [
                "unit greifen (strings) links term bund (fretted)",
                "unit greifen (strings) links term bund (fretted)",
                "unit stimmen (strings) needs unit teile (e-bass)",
            ]
        );
    }

    #[test]
    fn notices_languages_that_drift_apart() {
        let other = EN.replace("\"0 7\"", "\"0 5\"");
        let files = [
            ("units/intervall/de.md", DE),
            ("units/intervall/en.md", other.as_str()),
            ("glossary/de.md", GLOSSARY),
            (
                "glossary/en.md",
                "## Fifth\nid: quinte\nlesson: intervall\nFive.\n",
            ),
        ];
        let problems = Lessons::parse(&files).unwrap().check();
        assert_eq!(problems.len(), 1, "{problems:?}");
        assert!(problems[0].starts_with("unit intervall: de and en differ"));
    }
}
