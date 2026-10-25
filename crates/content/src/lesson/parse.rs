//! The lesson format: a Markdown subset, line by line.
//!
//! ```text
//! # Was ist ein Intervall?        title
//! id: intervall                   header: id, for, needs, terms (units); id, for, lesson (terms)
//! for: any
//!
//! ## Hören                        a page (one step on screen)
//! Ein **Intervall** ist der Abstand zweier Töne, etwa die [[quinte|Quinte]].
//! - list item
//! ::: listen root=A steps="0 7"   one-line block
//! ::: variant for=bass            block with a body, closed by ":::"
//! Am Bass …
//! :::
//! ```

use super::{
    Block, Chapter, Course, Doc, ExerciseRef, Lang, LessonError, Page, Practice, Question, Span,
    Task, Tones,
};
use serde::Deserialize;
use std::collections::BTreeMap;

const TITLE: &str = "# ";
const PAGE: &str = "## ";
const DIRECTIVE: &str = ":::";
const ITEM: &str = "- ";
const RIGHT: &str = "[x] ";
const WRONG: &str = "[ ] ";
/// A list item continues on lines indented by this.
const CONTINUED: &str = "  ";
const ANY: &str = "any";
const SEMITONES: i32 = 12;

/// Header of a unit file.
#[derive(Debug, Default)]
pub(super) struct UnitMeta {
    /// Line where the header starts (for errors between languages).
    pub line: usize,
    pub id: String,
    pub scope: String,
    pub needs: Vec<String>,
    pub terms: Vec<String>,
}

/// Header of a glossary section.
#[derive(Debug, Default)]
pub(super) struct TermMeta {
    /// Line of the term's "## Name".
    pub line: usize,
    pub id: String,
    pub scope: String,
    pub lesson: Option<String>,
}

pub(super) fn error(file: &str, line: usize, reason: &str) -> LessonError {
    LessonError {
        file: file.to_string(),
        line,
        reason: reason.to_string(),
    }
}

/// A unit file: title, header, pages.
pub(super) fn unit(file: &str, text: &str) -> Result<(UnitMeta, Doc), LessonError> {
    let mut r = Reader::new(file, text);
    let title = r.title()?;
    let line = r.pos + 1;
    let header = r.header()?;

    let mut meta = UnitMeta {
        line,
        scope: ANY.to_string(),
        ..UnitMeta::default()
    };
    for (line, key, value) in header {
        match key.as_str() {
            "id" => meta.id = value,
            "for" => meta.scope = value,
            "needs" => meta.needs = words(&value),
            "terms" => meta.terms = words(&value),
            _ => return Err(r.err_at(line, &format!("unknown header {key}"))),
        }
    }
    if meta.id.is_empty() {
        return Err(r.err_at(1, "header needs an id"));
    }

    let pages = r.pages()?;
    Ok((meta, Doc { title, pages }))
}

/// A glossary file: one "## Name" section per term, each with a header and blocks.
pub(super) fn glossary(file: &str, text: &str) -> Result<Vec<(TermMeta, Doc)>, LessonError> {
    let mut r = Reader::new(file, text);
    let mut terms = Vec::new();

    while let Some(line) = r.next_content() {
        let Some(name) = line.strip_prefix(PAGE) else {
            return Err(r.err("a term starts with ## and its name"));
        };
        let name = name.trim().to_string();
        let line = r.pos + 1;
        r.pos += 1;

        let mut meta = TermMeta {
            line,
            scope: ANY.to_string(),
            ..TermMeta::default()
        };
        for (line, key, value) in r.header()? {
            match key.as_str() {
                "id" => meta.id = value,
                "for" => meta.scope = value,
                "lesson" => meta.lesson = Some(value),
                _ => return Err(r.err_at(line, &format!("unknown header {key}"))),
            }
        }
        if meta.id.is_empty() {
            return Err(r.err(&format!("term {name} needs an id")));
        }

        let blocks = r.body(Stop::Page)?;
        let page = Page {
            title: name.clone(),
            blocks,
        };
        terms.push((
            meta,
            Doc {
                title: name,
                pages: vec![page],
            },
        ));
    }
    Ok(terms)
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CourseFile {
    id: String,
    #[serde(rename = "for")]
    scope: String,
    chapters: Vec<ChapterFile>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ChapterFile {
    id: String,
    de: String,
    en: String,
    units: Vec<String>,
    /// A practice block's attributes, e.g. `bundled=5 speeds="70 85 100"`.
    #[serde(default)]
    finale: Option<String>,
}

/// A course file (JSON): chapters with titles per language, unit ids and a finale.
pub(super) fn course(file: &str, text: &str) -> Result<Course, LessonError> {
    let parsed: CourseFile =
        serde_json::from_str(text).map_err(|e| error(file, e.line(), &e.to_string()))?;

    // Lines of the "finale" keys, in order, so an error names its line (JSON keeps no positions).
    let mut finale_lines = text
        .lines()
        .enumerate()
        .filter(|(_, l)| l.contains("\"finale\""))
        .map(|(i, _)| i + 1);

    let mut chapters = Vec::new();
    for c in parsed.chapters {
        let finale = match &c.finale {
            Some(attrs) => {
                let line = finale_lines.next().unwrap_or(1);
                let attrs = attributes(attrs).map_err(|r| error(file, line, &r))?;
                let practice = practice(&attrs)
                    .map_err(|r| error(file, line, &format!("finale of {}: {r}", c.id)))?;
                Some(practice)
            }
            None => None,
        };
        let titles = BTreeMap::from([(Lang::De, c.de), (Lang::En, c.en)]);
        chapters.push(Chapter {
            id: c.id,
            titles,
            units: c.units,
            finale,
        });
    }
    Ok(Course {
        id: parsed.id,
        scope: parsed.scope,
        chapters,
    })
}

/// Where a body ends: at the next page, or at the ":::" closing a block.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Stop {
    Page,
    Close,
}

/// Lines of one file with the read position; line numbers in errors count from 1.
struct Reader<'a> {
    file: &'a str,
    lines: Vec<&'a str>,
    pos: usize,
}

impl<'a> Reader<'a> {
    fn new(file: &'a str, text: &'a str) -> Reader<'a> {
        Reader {
            file,
            lines: text.lines().collect(),
            pos: 0,
        }
    }

    fn err(&self, reason: &str) -> LessonError {
        error(self.file, self.pos + 1, reason)
    }

    fn err_at(&self, line: usize, reason: &str) -> LessonError {
        error(self.file, line, reason)
    }

    /// The next non-blank line (not consumed), skipping blanks.
    fn next_content(&mut self) -> Option<&'a str> {
        while let Some(line) = self.lines.get(self.pos) {
            if !line.trim().is_empty() {
                return Some(line.trim_end());
            }
            self.pos += 1;
        }
        None
    }

    /// "# Title" as the first line with text.
    fn title(&mut self) -> Result<String, LessonError> {
        let line = self.next_content().unwrap_or_default();
        let Some(title) = line.strip_prefix(TITLE) else {
            return Err(self.err("a lesson starts with # and its title"));
        };
        self.pos += 1;
        Ok(title.trim().to_string())
    }

    /// "key: value" lines right below a title: (line number, key, value).
    fn header(&mut self) -> Result<Vec<(usize, String, String)>, LessonError> {
        let mut header = Vec::new();
        while let Some(line) = self.lines.get(self.pos) {
            let Some((key, value)) = line.split_once(": ") else {
                break;
            };
            if key.is_empty() || !key.chars().all(|c| c.is_ascii_lowercase()) {
                break;
            }
            header.push((self.pos + 1, key.to_string(), value.trim().to_string()));
            self.pos += 1;
        }
        Ok(header)
    }

    /// Pages up to the end of the file; each starts with "## ".
    fn pages(&mut self) -> Result<Vec<Page>, LessonError> {
        let mut pages = Vec::new();
        while let Some(line) = self.next_content() {
            let Some(title) = line.strip_prefix(PAGE) else {
                return Err(self.err("text before the first page (## title)"));
            };
            self.pos += 1;
            let blocks = self.body(Stop::Page)?;
            pages.push(Page {
                title: title.trim().to_string(),
                blocks,
            });
        }
        if pages.is_empty() {
            return Err(self.err("a lesson needs at least one page (## title)"));
        }
        Ok(pages)
    }

    /// Blocks until `stop`: paragraphs, lists and ":::" blocks.
    fn body(&mut self, stop: Stop) -> Result<Vec<Block>, LessonError> {
        let mut blocks = Vec::new();
        let mut text = Text::default();

        while let Some(&raw) = self.lines.get(self.pos) {
            let line = raw.trim_end();
            let number = self.pos + 1;

            // A blank line ends a paragraph or list.
            if line.trim().is_empty() {
                text.flush(&mut blocks, self.file)?;
                self.pos += 1;
                continue;
            }

            // The next page ends a page's body; inside a block it is a missing ":::".
            if line.starts_with(PAGE) || line.starts_with(TITLE) {
                if stop == Stop::Close {
                    return Err(self.err("block not closed with :::"));
                }
                break;
            }

            if line == DIRECTIVE {
                if stop == Stop::Page {
                    return Err(self.err("::: closes nothing here"));
                }
                text.flush(&mut blocks, self.file)?;
                self.pos += 1;
                return Ok(blocks);
            }

            if let Some(rest) = line.strip_prefix(DIRECTIVE) {
                text.flush(&mut blocks, self.file)?;
                self.pos += 1;
                let block = self.directive(rest.trim(), number)?;
                blocks.push(block);
                continue;
            }

            // List items, with indented continuation lines. A change between paragraph and list
            // ends the one before, so both stay in order ("Vorher", "- a", "Nachher").
            if let Some(item) = line.strip_prefix(ITEM) {
                if text.in_paragraph() {
                    text.flush(&mut blocks, self.file)?;
                }
                text.item(item, number);
            } else if raw.starts_with(CONTINUED) && text.in_list() {
                text.more(line.trim());
            } else {
                if text.in_list() {
                    text.flush(&mut blocks, self.file)?;
                }
                text.line(line.trim(), number);
            }
            self.pos += 1;
        }

        if stop == Stop::Close {
            return Err(self.err("block not closed with :::"));
        }
        text.flush(&mut blocks, self.file)?;
        Ok(blocks)
    }

    /// One ":::" block; `rest` is its name and attributes, `line` its line number.
    fn directive(&mut self, rest: &str, line: usize) -> Result<Block, LessonError> {
        let (name, attrs) = rest.split_once(' ').unwrap_or((rest, ""));
        let attrs = attributes(attrs).map_err(|r| self.err_at(line, &r))?;
        let bad = |r: String| error(self.file, line, &r);

        match name {
            "figure" => Ok(Block::Figure(tones(&attrs).map_err(bad)?)),
            "listen" => Ok(Block::Listen(tones(&attrs).map_err(bad)?)),
            "try" => Ok(Block::Try(Task {
                tones: tones(&attrs).map_err(bad)?,
                any_octave: attrs.get("octave").is_some_and(|o| o == ANY),
            })),
            "practice" => Ok(Block::Practice(practice(&attrs).map_err(bad)?)),
            "tuner" => Ok(Block::Tuner),
            "video" => Ok(Block::Video {
                src: attrs
                    .get("src")
                    .cloned()
                    .ok_or_else(|| self.err_at(line, "video needs src="))?,
            }),
            "variant" => {
                let scope = attrs
                    .get("for")
                    .cloned()
                    .ok_or_else(|| self.err_at(line, "variant needs for="))?;
                let blocks = self.body(Stop::Close)?;
                Ok(Block::Variant { scope, blocks })
            }
            "hint" => {
                let blocks = self.body(Stop::Close)?;
                let mut spans = Vec::new();
                for block in blocks {
                    let Block::Text(more) = block else {
                        return Err(self.err_at(line, "a hint holds text only"));
                    };
                    if !spans.is_empty() {
                        spans.push(Span::Text(" ".to_string()));
                    }
                    spans.extend(more);
                }
                Ok(Block::Hint(spans))
            }
            "question" => self.question(line),
            _ => Err(self.err_at(line, &format!("unknown block {name}"))),
        }
    }

    /// A question body: prompt lines, then options "- [x] right" and "- [ ] wrong", then ":::".
    fn question(&mut self, line: usize) -> Result<Block, LessonError> {
        let mut prompt = Vec::new();
        let mut options = Vec::new();
        let mut answer = None;

        loop {
            let Some(&raw) = self.lines.get(self.pos) else {
                return Err(self.err("question not closed with :::"));
            };
            let text = raw.trim();
            let number = self.pos + 1;
            self.pos += 1;

            if text == DIRECTIVE {
                break;
            }
            if text.is_empty() {
                continue;
            }

            let Some(item) = text.strip_prefix(ITEM) else {
                if !prompt.is_empty() {
                    prompt.push(Span::Text(" ".to_string()));
                }
                prompt.extend(spans(text).map_err(|r| self.err_at(number, &r))?);
                continue;
            };
            let (right, option) = if let Some(o) = item.strip_prefix(RIGHT) {
                (true, o)
            } else if let Some(o) = item.strip_prefix(WRONG) {
                (false, o)
            } else {
                return Err(self.err_at(number, "an option starts with - [x] or - [ ]"));
            };
            if right {
                if answer.is_some() {
                    return Err(self.err_at(number, "a question has one right option"));
                }
                answer = Some(options.len());
            }
            options.push(spans(option).map_err(|r| self.err_at(number, &r))?);
        }

        let Some(answer) = answer else {
            return Err(self.err_at(line, "a question needs one option marked [x]"));
        };
        if prompt.is_empty() || options.len() < 2 {
            return Err(self.err_at(line, "a question needs a prompt and two options"));
        }
        Ok(Block::Question(Question {
            prompt,
            options,
            answer,
        }))
    }
}

/// Paragraph or list being collected, with the line where it started (for errors).
#[derive(Default)]
struct Text {
    paragraph: Vec<String>,
    items: Vec<String>,
    line: usize,
}

impl Text {
    fn line(&mut self, text: &str, number: usize) {
        if self.paragraph.is_empty() {
            self.line = number;
        }
        self.paragraph.push(text.to_string());
    }

    fn item(&mut self, text: &str, number: usize) {
        if self.items.is_empty() {
            self.line = number;
        }
        self.items.push(text.trim().to_string());
    }

    fn in_list(&self) -> bool {
        !self.items.is_empty()
    }

    fn in_paragraph(&self) -> bool {
        !self.paragraph.is_empty()
    }

    fn more(&mut self, text: &str) {
        if let Some(last) = self.items.last_mut() {
            last.push(' ');
            last.push_str(text);
        }
    }

    /// Ends the open paragraph or list as a block.
    fn flush(&mut self, blocks: &mut Vec<Block>, file: &str) -> Result<(), LessonError> {
        let bad = |r: String| error(file, self.line, &r);
        if !self.paragraph.is_empty() {
            blocks.push(Block::Text(spans(&self.paragraph.join(" ")).map_err(bad)?));
        }
        if !self.items.is_empty() {
            let items = self
                .items
                .iter()
                .map(|i| spans(i))
                .collect::<Result<_, _>>();
            blocks.push(Block::List(items.map_err(bad)?));
        }
        self.paragraph.clear();
        self.items.clear();
        Ok(())
    }
}

/// Inline markup: "**strong**", "*em*", "[[id]]" and "[[id|text]]".
fn spans(text: &str) -> Result<Vec<Span>, String> {
    let mut spans = Vec::new();
    let mut plain = String::new();
    let mut rest = text;

    while !rest.is_empty() {
        // Each marker: its opening, its closing, and how the inside becomes a span.
        let (open, close): (&str, &str) = if rest.starts_with("[[") {
            ("[[", "]]")
        } else if rest.starts_with("**") {
            ("**", "**")
        } else if rest.starts_with('*') {
            ("*", "*")
        } else {
            let next = rest.char_indices().nth(1).map_or(rest.len(), |(i, _)| i);
            plain.push_str(&rest[..next]);
            rest = &rest[next..];
            continue;
        };

        let inner = &rest[open.len()..];
        let Some(end) = inner.find(close) else {
            return Err(format!("{open} without {close}"));
        };
        let content = &inner[..end];
        if content.is_empty() {
            return Err(format!("empty {open}{close}"));
        }
        if !plain.is_empty() {
            spans.push(Span::Text(std::mem::take(&mut plain)));
        }
        spans.push(match open {
            "[[" => {
                let (id, shown) = content.split_once('|').unwrap_or((content, content));
                Span::Term {
                    id: id.trim().to_string(),
                    text: shown.trim().to_string(),
                }
            }
            "**" => Span::Strong(content.to_string()),
            _ => Span::Em(content.to_string()),
        });
        rest = &inner[end + close.len()..];
    }

    if !plain.is_empty() {
        spans.push(Span::Text(plain));
    }
    Ok(spans)
}

/// `key=value key="two words"` → map.
fn attributes(text: &str) -> Result<BTreeMap<String, String>, String> {
    let mut map = BTreeMap::new();
    let mut rest = text.trim();

    while !rest.is_empty() {
        let Some((key, after)) = rest.split_once('=') else {
            return Err(format!("attribute without =: {rest}"));
        };
        let key = key.trim();
        let (value, next) = if let Some(quoted) = after.strip_prefix('"') {
            let Some(end) = quoted.find('"') else {
                return Err(format!("{key}= has no closing quote"));
            };
            (&quoted[..end], &quoted[end + 1..])
        } else {
            after.split_once(' ').unwrap_or((after, ""))
        };
        if map.insert(key.to_string(), value.to_string()).is_some() {
            return Err(format!("{key}= twice"));
        }
        rest = next.trim_start();
    }
    Ok(map)
}

fn words(text: &str) -> Vec<String> {
    text.split_whitespace().map(str::to_string).collect()
}

/// Pitch class of a note name, C = 0: "C", "F#", "Bb".
pub fn note_class(name: &str) -> Option<u8> {
    let mut chars = name.chars();
    let base: i32 = match chars.next()? {
        'C' => 0,
        'D' => 2,
        'E' => 4,
        'F' => 5,
        'G' => 7,
        'A' => 9,
        'B' => 11,
        _ => return None,
    };
    let shift = match chars.as_str() {
        "" => 0,
        "#" => 1,
        "b" => -1,
        _ => return None,
    };
    u8::try_from((base + shift).rem_euclid(SEMITONES)).ok()
}

/// `root=A steps="0 7" labels="1 5"`.
fn tones(attrs: &BTreeMap<String, String>) -> Result<Tones, String> {
    let root = attrs.get("root").ok_or("root= missing")?;
    let root = note_class(root).ok_or(format!("unknown note {root}"))?;
    let steps = match attrs.get("steps") {
        Some(steps) => steps
            .split_whitespace()
            .map(|s| {
                s.parse::<i32>()
                    .map_err(|_| format!("step {s} is no number"))
            })
            .collect::<Result<Vec<_>, _>>()?,
        None => vec![0],
    };
    let labels = attrs.get("labels").map(|l| words(l)).unwrap_or_default();
    if !labels.is_empty() && labels.len() != steps.len() {
        return Err("labels= needs one label per step".to_string());
    }
    let string = match attrs.get("string") {
        Some(name) => Some(note_class(name).ok_or(format!("unknown string {name}"))?),
        None => None,
    };
    Ok(Tones {
        root,
        steps,
        labels,
        string,
    })
}

/// `bundled=5 speeds="60 80 100"` or `kind=groove variant=145 rhythm=quarters key=E bpm=70
/// judging=time`.
fn practice(attrs: &BTreeMap<String, String>) -> Result<Practice, String> {
    let get = |key: &str| attrs.get(key).cloned().unwrap_or_default();
    let exercise = if let Some(number) = attrs.get("bundled") {
        ExerciseRef::Bundled(number.parse().map_err(|_| format!("bundled={number}"))?)
    } else {
        let key = note_class(&get("key")).ok_or("key= missing or unknown")?;
        let bpm = get("bpm");
        ExerciseRef::Generated {
            kind: attrs
                .get("kind")
                .cloned()
                .ok_or("bundled= or kind= missing")?,
            variant: get("variant"),
            rhythm: get("rhythm"),
            key,
            bpm: bpm.parse().map_err(|_| format!("bpm={bpm}"))?,
            judging: get("judging"),
        }
    };
    let speeds = match attrs.get("speeds") {
        Some(s) => s
            .split_whitespace()
            .map(|v| v.parse::<u8>().map_err(|_| format!("speed {v}")))
            .collect::<Result<Vec<_>, _>>()?,
        None => Vec::new(),
    };
    Ok(Practice { exercise, speeds })
}

#[cfg(test)]
mod tests {
    use super::*;

    const UNIT: &str = "\
# Was ist eine Septime?
id: septime
for: any
needs: intervall
terms: septime

## Hören
Die **Septime** liegt einen *Ganzton* unter der [[oktave|Oktave]].
::: listen root=A steps=\"0 10\" labels=\"1 b7\"

::: variant for=bass
Am Bass:
- zwei Bünde unter der Oktave
  oder drei Bünde über der Sexte
:::

## Prüfen
::: question
Wie viele Halbtöne hat die kleine Septime?
- [ ] 9
- [x] 10
:::
::: practice kind=arpeggio variant=251 key=C bpm=80 speeds=\"60 80 100\"
";

    #[test]
    fn reads_a_unit() {
        let (meta, doc) = unit("u/de.md", UNIT).unwrap();
        assert_eq!(meta.id, "septime");
        assert_eq!(meta.needs, ["intervall"]);
        assert_eq!(doc.title, "Was ist eine Septime?");
        assert_eq!(doc.pages.len(), 2);

        let first = &doc.pages[0].blocks;
        assert_eq!(
            first[0],
            Block::Text(vec![
                Span::Text("Die ".into()),
                Span::Strong("Septime".into()),
                Span::Text(" liegt einen ".into()),
                Span::Em("Ganzton".into()),
                Span::Text(" unter der ".into()),
                Span::Term {
                    id: "oktave".into(),
                    text: "Oktave".into()
                },
                Span::Text(".".into()),
            ])
        );
        assert_eq!(
            first[1],
            Block::Listen(Tones {
                root: 9,
                steps: vec![0, 10],
                labels: vec!["1".into(), "b7".into()],
                string: None,
            })
        );
        let Block::Variant { scope, blocks } = &first[2] else {
            panic!("{:?}", first[2]);
        };
        assert_eq!(scope, "bass");
        assert_eq!(blocks.len(), 2);
        assert_eq!(
            blocks[1],
            Block::List(vec![vec![Span::Text(
                "zwei Bünde unter der Oktave oder drei Bünde über der Sexte".into()
            )]])
        );

        let second = &doc.pages[1].blocks;
        let Block::Question(q) = &second[0] else {
            panic!("{:?}", second[0]);
        };
        assert_eq!((q.options.len(), q.answer), (2, 1));
        let Block::Practice(p) = &second[1] else {
            panic!("{:?}", second[1]);
        };
        assert_eq!(p.speeds, [60, 80, 100]);
        assert!(matches!(
            &p.exercise,
            ExerciseRef::Generated {
                key: 0,
                bpm: 80,
                ..
            }
        ));
    }

    #[test]
    fn names_the_line_of_a_mistake() {
        let open = "# T\nid: t\n\n## P\n::: variant for=bass\ntext\n";
        assert_eq!(unit("u/de.md", open).unwrap_err().line, 7);

        let unknown = "# T\nid: t\n\n## P\n\n::: banjo\n";
        let e = unit("u/de.md", unknown).unwrap_err();
        assert_eq!((e.line, e.reason.as_str()), (6, "unknown block banjo"));

        let markup = "# T\nid: t\n\n## P\nein **offenes Wort\n";
        assert_eq!(unit("u/de.md", markup).unwrap_err().line, 5);

        let two = "# T\nid: t\n## P\n::: question\nQ?\n- [x] a\n- [x] b\n:::\n";
        assert_eq!(unit("u/de.md", two).unwrap_err().line, 7);
    }

    /// Review #119: text after a list without a blank line stayed before it.
    #[test]
    fn keeps_text_and_lists_in_order() {
        let text = "# T\nid: t\n\n## P\nVorher\n- a\nNachher\n";
        let (_, doc) = unit("u/de.md", text).unwrap();
        let kinds: Vec<&str> = doc.pages[0]
            .blocks
            .iter()
            .map(|b| match b {
                Block::Text(_) => "text",
                Block::List(_) => "list",
                _ => "other",
            })
            .collect();
        assert_eq!(kinds, ["text", "list", "text"]);
    }

    #[test]
    fn reads_note_names() {
        assert_eq!(note_class("C"), Some(0));
        assert_eq!(note_class("F#"), Some(6));
        assert_eq!(note_class("Bb"), Some(10));
        assert_eq!(note_class("Cb"), Some(11));
        assert_eq!(note_class("H"), None);
    }

    #[test]
    fn reads_a_glossary() {
        let text = "## Septime\nid: septime\nlesson: septime\nSieben Stufen.\n\n## Slap\nid: slap\nfor: bass\nDaumen.\n";
        let terms = glossary("g/de.md", text).unwrap();
        assert_eq!(terms.len(), 2);
        assert_eq!(terms[0].0.lesson.as_deref(), Some("septime"));
        assert_eq!(terms[1].0.scope, "bass");
        assert_eq!(terms[1].1.title, "Slap");
    }

    /// Review #119: a bad finale named line 1.
    #[test]
    fn names_the_line_of_a_bad_finale() {
        let text = "{\"id\": \"bass\", \"for\": \"bass\", \"chapters\": [\n  {\"id\": \"a\", \"de\": \"A\", \"en\": \"A\", \"units\": [],\n   \"finale\": \"speeds=x\"}]}";
        assert_eq!(course("c.json", text).unwrap_err().line, 3);
    }

    #[test]
    fn reads_a_course() {
        let text = r#"{"id": "bass", "for": "bass", "chapters": [
            {"id": "start", "de": "Loslegen", "en": "Getting started", "units": ["a", "b"],
             "finale": "bundled=5 speeds=\"70 100\""}]}"#;
        let c = course("c.json", text).unwrap();
        assert_eq!(c.chapters[0].titles[&Lang::En], "Getting started");
        assert_eq!(
            c.chapters[0].finale.as_ref().map(|f| &f.exercise),
            Some(&ExerciseRef::Bundled(5))
        );
    }
}
