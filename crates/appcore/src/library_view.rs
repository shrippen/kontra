//! What the library page shows (LH-BIB-17): one JSON from the pieces the library knows, the
//! files of the linked folders and the bundled exercises, filtered by a source, sorted by a
//! column, grouped and searched, plus the counts for the source tree.
//!
//! ```text
//! recent entries ──┐
//! linked folders ──┼─► rows ─► source ─► query ─► sort ─► group ─► JSON
//! bundled files ───┘      └─► counts (all rows, whatever the source)
//! ```

use crate::origin::{Origin, Places};
use crate::{Core, RecentEntry};
use kontra_session::ProgressStore;
use serde_json::{json, Value};
use std::cell::OnceCell;
use std::cmp::Ordering;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};

/// Best result and last run per song id, read when the first unopened file needs it.
type Played = OnceCell<HashMap<String, (Option<f64>, Option<u64>)>>;

const SOURCE_FOLDER: &str = "folder:";
const SOURCE_SETLIST: &str = "setlist:";
/// Separates the sort column from its direction: "tempo:desc".
const ORDER_SEPARATOR: char = ':';

/// Which pieces the page lists; the tree on its left picks one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Source {
    All,
    /// Pieces opened or played, wherever they lie.
    Recent,
    Origin(Origin),
    /// A linked folder or a folder below one, with everything under it.
    Folder(PathBuf),
    Setlist(String),
}

impl Source {
    /// "all", "recent", an origin's key, "folder:/path" or "setlist:name".
    pub(crate) fn from_key(key: &str) -> Option<Source> {
        if let Some(path) = key.strip_prefix(SOURCE_FOLDER) {
            return Some(Source::Folder(PathBuf::from(path)));
        }
        if let Some(name) = key.strip_prefix(SOURCE_SETLIST) {
            return Some(Source::Setlist(name.to_string()));
        }
        match key {
            "all" => Some(Source::All),
            "recent" => Some(Source::Recent),
            _ => Origin::from_key(key).map(Source::Origin),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SortKey {
    Title,
    Origin,
    Tempo,
    LastPlayed,
    Result,
}

impl SortKey {
    pub(crate) fn from_key(key: &str) -> Option<SortKey> {
        match key {
            "title" => Some(SortKey::Title),
            "origin" => Some(SortKey::Origin),
            "tempo" => Some(SortKey::Tempo),
            "lastPlayed" => Some(SortKey::LastPlayed),
            "result" => Some(SortKey::Result),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Direction {
    Ascending,
    Descending,
}

/// A column and a direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Order {
    pub(crate) key: SortKey,
    pub(crate) direction: Direction,
}

impl Order {
    /// "title", "tempo:desc", "lastPlayed:asc"; no direction means ascending.
    pub(crate) fn from_key(text: &str) -> Option<Order> {
        let (key, direction) = text.split_once(ORDER_SEPARATOR).unwrap_or((text, "asc"));
        let direction = match direction {
            "asc" => Direction::Ascending,
            "desc" => Direction::Descending,
            _ => return None,
        };
        Some(Order {
            key: SortKey::from_key(key)?,
            direction,
        })
    }
}

impl Default for Order {
    fn default() -> Self {
        Order {
            key: SortKey::Title,
            direction: Direction::Ascending,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Group {
    None,
    Origin,
    Artist,
    Folder,
}

impl Group {
    pub(crate) fn from_key(key: &str) -> Option<Group> {
        match key {
            "none" => Some(Group::None),
            "origin" => Some(Group::Origin),
            "artist" => Some(Group::Artist),
            "folder" => Some(Group::Folder),
            _ => None,
        }
    }
}

/// One line of the table.
#[derive(Debug, Clone)]
struct Row {
    path: PathBuf,
    title: String,
    /// `title` in lower case, for sorting and search.
    title_key: String,
    artist: String,
    tags: Vec<String>,
    origin: Origin,
    /// The linked folder the file lies in.
    root: Option<PathBuf>,
    /// Below `root`, "/" between the levels; empty at the top.
    folder: String,
    bpm: Option<f64>,
    last_played: Option<u64>,
    best_rate: Option<f64>,
    missing: bool,
}

impl Row {
    /// The folder as shown and grouped: "tabs/bass/slap"; empty outside linked folders.
    fn folder_label(&self) -> String {
        let Some(name) = self.root.as_deref().and_then(Path::file_name) else {
            return String::new();
        };
        let name = name.to_string_lossy();
        match self.folder.as_str() {
            "" => name.into_owned(),
            below => format!("{name}/{below}"),
        }
    }

    fn matches(&self, words: &[String]) -> bool {
        let folder = self.folder_label().to_lowercase();
        words.iter().all(|w| {
            self.title_key.contains(w)
                || self.artist.to_lowercase().contains(w)
                || self.tags.iter().any(|t| t.to_lowercase().contains(w))
                || folder.contains(w)
        })
    }
}

impl Core {
    /// The page's data: `{"rows": […], "groups": [{"key", "title", "count"}], "counts": {…}}`.
    /// `source`, `sort` ("title:desc") and `group` are keys (an unknown one means "all", title
    /// ascending, none); `query` is words that must each be in title, artist, a tag or the
    /// folder. A piece without a value in the sorted column comes last, in either direction.
    pub fn library_json(&self, source: &str, sort: &str, group: &str, query: &str) -> String {
        let source = Source::from_key(source).unwrap_or(Source::All);
        let order = Order::from_key(sort).unwrap_or_default();
        let group = Group::from_key(group).unwrap_or(Group::None);
        let all = self.library_rows();
        let counts = self.library_counts(&all);

        let words: Vec<String> = query.split_whitespace().map(str::to_lowercase).collect();
        let mut rows: Vec<Row> = all
            .into_iter()
            .filter(|r| self.in_source(r, &source) && r.matches(&words))
            .collect();
        rows.sort_by(|a, b| compare(a, b, order));
        // Stable: the order inside a group stays the one of the column.
        rows.sort_by_cached_key(|r| group_rank(r, group));

        let mut headers: Vec<(String, String, usize)> = Vec::new();
        let list: Vec<Value> = rows
            .iter()
            .map(|r| {
                let (key, title) = self.group_of(r, group);
                match headers.last_mut() {
                    Some(last) if last.0 == key => last.2 += 1,
                    _ => headers.push((key.clone(), title, 1)),
                }
                self.row_json(r, &key)
            })
            .collect();
        let groups: Vec<Value> = match group {
            Group::None => Vec::new(),
            _ => headers
                .iter()
                .map(|(key, title, count)| json!({ "key": key, "title": title, "count": count }))
                .collect(),
        };
        json!({ "rows": list, "groups": groups, "counts": counts }).to_string()
    }

    fn row_json(&self, row: &Row, group: &str) -> Value {
        // Lossy: a name that is not text must not end the program (review #235).
        let path = row.path.to_string_lossy();
        let format = row
            .path
            .extension()
            .map_or_else(String::new, |e| e.to_string_lossy().to_uppercase());
        json!({
            "id": path,
            "path": path,
            "title": row.title,
            "artist": row.artist,
            "format": format,
            "origin": row.origin.key(),
            "originLabel": self.origin_label(row.origin),
            "folder": row.folder,
            "root": row.root.as_ref().map(|r| r.to_string_lossy()),
            "bpm": row.bpm,
            "lastPlayed": row.last_played,
            "bestPercent": row.best_rate.map(|b| (b * 100.0).round()),
            "missing": row.missing,
            "tags": row.tags,
            "group": group,
        })
    }

    fn origin_label(&self, origin: Origin) -> String {
        self.tr(&format!("library.origin.{}", origin.key()), &[])
    }

    /// The group a row falls in: its key and the header's text.
    fn group_of(&self, row: &Row, group: Group) -> (String, String) {
        match group {
            Group::None => (String::new(), String::new()),
            Group::Origin => (row.origin.key().into(), self.origin_label(row.origin)),
            Group::Artist => match row.artist.trim() {
                "" => (String::new(), self.tr("library.group.artist.none", &[])),
                artist => (artist.to_lowercase(), artist.into()),
            },
            Group::Folder => match row.folder_label() {
                label if label.is_empty() => {
                    (String::new(), self.tr("library.group.folder.none", &[]))
                }
                // Case-folded like the artist: the rank is, so equal names stay together.
                label => (label.to_lowercase(), label),
            },
        }
    }

    fn in_source(&self, row: &Row, source: &Source) -> bool {
        match source {
            Source::All => true,
            Source::Recent => row.last_played.is_some(),
            Source::Origin(origin) => row.origin == *origin,
            Source::Folder(dir) => row.path.starts_with(dir),
            Source::Setlist(name) => self
                .library
                .setlists
                .iter()
                .find(|s| &s.name == name)
                .is_some_and(|s| s.pieces.contains(&row.path)),
        }
    }

    /// Every piece the page can show: the library's entries, then files of the linked folders
    /// and bundled exercises the library does not know yet, with what the background read found
    /// in them (`folders::meta`).
    fn library_rows(&self) -> Vec<Row> {
        let places = self.places();
        let store = self.progress_store();
        let mut seen: HashSet<PathBuf> = HashSet::new();
        let mut rows: Vec<Row> = Vec::new();

        for entry in &self.library.recent {
            seen.insert(entry.path.clone());
            let progress = store.load_id(&entry.song_id);
            let best = progress.as_ref().and_then(|p| p.best_hit_rate());
            // Played, or at least opened: `last_unix` follows every opening.
            let last_run = progress
                .as_ref()
                .and_then(|p| p.runs.iter().map(|r| r.finished_unix).max());
            let last = last_run.max(Some(entry.last_unix).filter(|t| *t > 0));
            rows.push(self.entry_row(entry, &places, best, last));
        }

        let unknown = self
            .library
            .linked_folders
            .iter()
            .filter_map(|root| self.folders.walked(root))
            .flat_map(|walked| walked.files.iter())
            .cloned()
            .chain(self.bundled_files());
        let played = Played::new();
        for path in unknown {
            if !seen.insert(path.clone()) {
                continue;
            }
            rows.push(self.unopened_row(path, &places, &played));
        }
        rows
    }

    /// Best result and last run per song id, read once for all unopened files (one file read
    /// each would cost thousands in a large folder, at every refresh).
    fn played(store: &ProgressStore) -> HashMap<String, (Option<f64>, Option<u64>)> {
        store
            .all()
            .into_iter()
            .map(|p| {
                let last = p.runs.iter().map(|r| r.finished_unix).max();
                (p.song_id.clone(), (p.best_hit_rate(), last))
            })
            .collect()
    }

    /// A file the library has not opened: what the background read found in it (`meta`), a
    /// bundled exercise's tempo from its plan, else the file name alone.
    fn unopened_row(&self, path: PathBuf, places: &Places, played: &Played) -> Row {
        let row = self.file_row(path, places);
        // Only an exercise can be bundled; asking costs file system calls (`exercises_dir`).
        let bundled = (row.origin == Origin::Exercise)
            .then(|| self.bundled_bpm(&row.path))
            .flatten();
        if let Some(bpm) = bundled {
            return Row {
                bpm: Some(bpm),
                ..row
            };
        }
        let Some(meta) = self.folders.metas.get(&row.path) else {
            return row;
        };
        let (best_rate, last_played) = played
            .get_or_init(|| Self::played(&self.progress_store()))
            .get(&meta.song_id)
            .copied()
            .unwrap_or_default();
        Row {
            title_key: meta.title.to_lowercase(),
            title: meta.title.clone(),
            artist: meta.artist.clone(),
            bpm: meta.bpm,
            best_rate,
            last_played,
            ..row
        }
    }

    fn entry_row(
        &self,
        entry: &RecentEntry,
        places: &Places,
        best_rate: Option<f64>,
        last_played: Option<u64>,
    ) -> Row {
        let title = self.library_title(entry);
        Row {
            title_key: title.to_lowercase(),
            title,
            artist: entry.artist.clone(),
            tags: entry.tags.clone(),
            bpm: entry
                .tempo_bpm
                .or_else(|| self.folders.metas.get(&entry.path).and_then(|m| m.bpm)),
            last_played,
            best_rate,
            missing: !entry.path.exists(),
            ..self.file_row(entry.path.clone(), places)
        }
    }

    /// A row from the path alone: the file name for a title.
    fn file_row(&self, path: PathBuf, places: &Places) -> Row {
        let found = places.classify(&path);
        let (origin, root) = (found.origin, found.root.map(Path::to_path_buf));
        let folder = root
            .as_deref()
            .and_then(|r| path.parent()?.strip_prefix(r).ok())
            .map(|rel| {
                let parts: Vec<_> = rel.iter().map(|p| p.to_string_lossy()).collect();
                parts.join("/")
            })
            .unwrap_or_default();
        let stem = path
            .file_stem()
            .map_or_else(String::new, |s| s.to_string_lossy().into_owned());
        // Only an exercise has a plan title; building the plan for every file of a large folder
        // cost most of a refresh.
        let title = (origin == Origin::Exercise)
            .then(|| self.bundled_title(&stem))
            .flatten()
            .unwrap_or(stem);
        Row {
            title_key: title.to_lowercase(),
            title,
            artist: String::new(),
            tags: Vec::new(),
            origin,
            root,
            folder,
            bpm: None,
            last_played: None,
            best_rate: None,
            missing: false,
            path,
        }
    }

    /// The bundled exercises (`level/file.mid`), by name only.
    fn bundled_files(&self) -> Vec<PathBuf> {
        let list = |dir: &Path| -> Vec<PathBuf> {
            let mut paths: Vec<PathBuf> = std::fs::read_dir(dir)
                .map(|d| d.flatten().map(|e| e.path()).collect())
                .unwrap_or_default();
            paths.sort();
            paths
        };
        list(&Self::exercises_dir())
            .iter()
            .filter(|p| p.is_dir())
            .flat_map(|level| list(level))
            .filter(|p| p.extension().is_some_and(|x| x == "mid"))
            .collect()
    }

    /// The numbers for the tree: all pieces, recent ones, each origin, each linked folder with
    /// its subfolders, each setlist.
    fn library_counts(&self, rows: &[Row]) -> Value {
        let origins: BTreeMap<&str, usize> = Origin::ALL
            .iter()
            .map(|o| (o.key(), rows.iter().filter(|r| r.origin == *o).count()))
            .collect();
        let folders: Vec<Value> = self
            .library
            .linked_folders
            .iter()
            .map(|root| {
                let mut tree = Node::default();
                // As the `folder:` source: everything below, also what a nested link claims.
                for row in rows.iter().filter(|r| r.path.starts_with(root)) {
                    let below = row.path.parent().and_then(|d| d.strip_prefix(root).ok());
                    tree.add(
                        below
                            .into_iter()
                            .flat_map(|d| d.iter())
                            .filter_map(|p| p.to_str()),
                    );
                }
                let walked = self.folders.walked(root);
                let missing = walked.is_some_and(|w| w.missing);
                let cut_off = walked.is_some_and(|w| w.cut_off);
                let mut node = tree.json(root, &name_of(root));
                node["missing"] = json!(missing);
                node["cutOff"] = json!(cut_off);
                node
            })
            .collect();
        let setlists: Vec<Value> = self
            .library
            .setlists
            .iter()
            .map(|s| json!({ "name": s.name, "count": s.pieces.len() }))
            .collect();
        json!({
            "all": rows.len(),
            "recent": rows.iter().filter(|r| r.last_played.is_some()).count(),
            "origins": origins,
            "folders": folders,
            "setlists": setlists,
        })
    }
}

/// A folder of the tree and the number of pieces in it and below.
#[derive(Default)]
struct Node {
    count: usize,
    below: BTreeMap<String, Node>,
}

impl Node {
    fn add<'a>(&mut self, mut parts: impl Iterator<Item = &'a str>) {
        self.count += 1;
        if let Some(part) = parts.next() {
            self.below.entry(part.to_string()).or_default().add(parts);
        }
    }

    fn json(&self, path: &Path, name: &str) -> Value {
        let children: Vec<Value> = self
            .below
            .iter()
            .map(|(n, node)| node.json(&path.join(n), n))
            .collect();
        json!({
            "path": path.to_string_lossy(),
            "name": name,
            "count": self.count,
            "children": children,
        })
    }
}

fn name_of(path: &Path) -> String {
    path.file_name().map_or_else(
        || path.to_string_lossy().into_owned(),
        |n| n.to_string_lossy().into_owned(),
    )
}

/// Orders by the column; a missing value is last in both directions; ties by title, then path.
fn compare(a: &Row, b: &Row, order: Order) -> Ordering {
    let by_column = match order.key {
        SortKey::Title => directed(a.title_key.cmp(&b.title_key), order.direction),
        SortKey::Origin => directed(
            origin_index(a.origin).cmp(&origin_index(b.origin)),
            order.direction,
        ),
        SortKey::Tempo => optional(a.bpm, b.bpm, order.direction, f64::total_cmp),
        SortKey::LastPlayed => optional(a.last_played, b.last_played, order.direction, u64::cmp),
        SortKey::Result => optional(a.best_rate, b.best_rate, order.direction, f64::total_cmp),
    };
    by_column
        .then_with(|| a.title_key.cmp(&b.title_key))
        .then_with(|| a.path.cmp(&b.path))
}

fn directed(ordering: Ordering, direction: Direction) -> Ordering {
    match direction {
        Direction::Ascending => ordering,
        Direction::Descending => ordering.reverse(),
    }
}

fn optional<T: Copy>(
    a: Option<T>,
    b: Option<T>,
    direction: Direction,
    cmp: fn(&T, &T) -> Ordering,
) -> Ordering {
    match (a, b) {
        (Some(a), Some(b)) => directed(cmp(&a, &b), direction),
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => Ordering::Equal,
    }
}

fn origin_index(origin: Origin) -> usize {
    Origin::ALL.iter().position(|o| *o == origin).unwrap_or(0)
}

/// Where a row's group stands among the groups: origins as in the tree, the others by name
/// with the nameless one last.
fn group_rank(row: &Row, group: Group) -> (bool, usize, String) {
    let name = match group {
        Group::None | Group::Origin => String::new(),
        Group::Artist => row.artist.trim().to_lowercase(),
        Group::Folder => row.folder_label().to_lowercase(),
    };
    let index = match group {
        Group::Origin => origin_index(row.origin),
        _ => 0,
    };
    (
        name.is_empty() && group != Group::None && group != Group::Origin,
        index,
        name,
    )
}
