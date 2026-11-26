# Architecture

How this repository is meant to be built. Where the code differs today is listed under
"Known deviations". Change this file in the PR that changes the structure.

## What it does
Kontra is a bass trainer for the desktop (Linux first, Windows package): it listens to a real
bass, shows tab lane, tablature and notation running along, and tells the player how pitch and
timing were. Players learn with courses and exercises or simply play pieces they import.

## Building blocks
Every block under `crates/` is one Rust crate.

| Path | Its one task |
|---|---|
| `crates/app` | The window: QML pages (`qml/`) and the bridge between QML and Rust (`src/bridge.rs`, `src/bridge/`, one file per subject). No logic of its own. |
| `crates/app/qml/Kante` | The Kante design system, copied unchanged by `scripts/sync-kante.sh`. Never edited here. |
| `crates/appcore` | The app without Qt: everything the bridge calls. `Core` in `lib.rs`, one file per subject (library, settings store, effects, quiz, tuner, …), the texts (`i18n/de.json`, `en.json`). |
| `crates/cli` | `kontra-cli`: the same functions on the command line, for development, QA, CI and scripts. |
| `crates/session` | Practising: timeline, modes, matching played against expected notes, scoring, progress, courses; and the one-shot services a user interface needs (import, device lists, plugin search, web search, stems). |
| `crates/content` | Pieces as data: import (Guitar Pro, MIDI, MusicXML, MuseScore), tab assignment, transposing, generated exercises, lessons, `.kontra` packages. No audio, no devices. |
| `crates/engine` | Audio at run time: owns the real-time threads, turns input into note events, plays backing, click and effects, hosts plugin chains, measures the detection corpus. |
| `crates/dsp` | Signal processing without I/O: onset, pitch, tuner, effects, synthesis. |
| `crates/stretch` | Time-stretch: tempo change without pitch change. Without I/O. |
| `crates/drivers` | The outside world: sound and MIDI devices, audio files, HTTPS, the system's secret store, CLAP/VST3 plugin files, the external stem separator. No music logic. |
| `crates/model` | Domain types as plain data: notes, arrangement, tuning, instrument, input source, effect chain, device descriptions. Depends on no other block. |
| `crates/test-clap-gain`, `crates/test-vst3-gain` | Test plugins the plugin host is tested against. Use no Kontra block. |
| `tests/` | Tests across blocks: the journeys of `planung/QA.md` (`tests/journeys/`) and the layer test (`tests/physical_design.rs`). |
| `testdata/`, `lessons/`, `schema/` | Detection corpus, generated exercises, import samples; course texts; JSON Schemas of the file formats. |
| `packaging/`, `scripts/`, `tools/` | Flatpak and Windows package; QA, release and sync scripts; the structure check. |

## Layers
```
app ─► appcore ─┐
                ├─► session ─┬─► engine ─┬─► drivers
cli ────────────┘            │           ├─► dsp
                             └─► content └─► stretch
model: usable by every block except app, stretch and the test plugins; depends on nothing
```
An arrow means "may use". `tests/physical_design.rs` checks the arrows in each `Cargo.toml`
and that session hands no driver name on.

- A block uses only the block directly below it, and `model`. Tests may reach further.
- The user interfaces (`app`, `appcore`, `cli`) reach engine, dsp, stretch and drivers only
  through functions and types that session defines itself. Session hands no engine or driver
  name on unchanged (`pub use`); it wraps what a user interface needs, also for the CLI.
- Session may hand content's types and functions on unchanged: they hold no devices and no
  running state. Only session depends on content.
- `app` holds no logic: a page calls the bridge, the bridge calls `appcore`. What can be decided
  without Qt is decided in `appcore` and tested there.
- Data that crosses more than one layer (notes, device descriptions, effect chain) is a type in
  `model`, not in the block that produces it.
- Instrument and input are separate descriptions in `model` (`Instrument`, `InputSource`). A new
  instrument changes neither session's logic nor the scoring.
- `drivers` does device, audio file, web and plugin I/O. `content` reads and writes the files
  of its own formats (scores, packages, lessons). A block's own state files (settings, library,
  progress, caches) are written by that block, each file in one place.
- The audio callback (engine, drivers, dsp, stretch) never allocates, locks or waits; results
  leave it through ring buffers created beforehand.
- No text in code or QML: every text the window shows is a key in `crates/appcore/i18n/de.json`
  and `en.json`. Terminal output (`kontra-cli`, command line usage) is English text in code.
- `crates/app/qml/Kante` is changed only in the Kante repository; a component Kontra needs is
  added there first.

## Where new things go
| Change | Touch |
|---|---|
| A text or translation | `crates/appcore/i18n/de.json` and `en.json`, same key in both; QML reads it with `texts.t("key")`. |
| A setting | `appcore/src/store.rs` (field, default); a fixed choice also `appcore/src/choices.rs`; bridge: `app/src/bridge.rs` (property, call, field), `bridge/start.rs` (start value), `bridge/settings.rs` (change function, `sync_settings`); the QML page; texts. |
| A page | `crates/app/qml/<Name>Page.qml`; `crates/app/build.rs` (file list); `Main.qml` (page map, navigation entry); texts; a test in `crates/app/qml/tests/`. |
| A function a page calls | The subject's file in `appcore/src/`; `app/src/bridge.rs` (declaration) and the subject's file in `app/src/bridge/`; logic below that in `session`. |
| A practice mode | `session/src/session.rs` (`Mode`) and `session/src/practice/`; `appcore/src/choices.rs` (`PracticeMode`) and its mapping in `appcore/src/lib.rs`; `cli/src/args.rs`; `PracticePage.qml`; texts; a journey in `planung/QA.md`. |
| An import format | A reader file in `crates/content/src/`; `FileKind` and `EXTENSIONS` in `content/src/lib.rs`; a sample in `testdata/import/`; `packaging/*.mime.xml` and `*.desktop`. |
| An exercise kind | `crates/content/src/exercise.rs` and `exercise/`; `appcore/src/bundled.rs`; `ExercisePage.qml`; texts; regenerate `testdata/exercises` (`kontra-cli exercises <dir>`). |
| A built-in effect | `model/src/fx.rs` (kind, defaults); `dsp/src/fx.rs` (sound); `engine/src/plugin_group.rs` (group); `appcore/src/effects.rs` (key, order); texts `fx.kind.*`, `fx.value.*`. |
| A CLI command | `cli/src/args.rs` (`Command`), `cli/src/main.rs` (dispatch), a file of its own in `cli/src/`; the work itself is a session function. |
| A device, audio file or web access | The access in `crates/drivers`; engine uses it; session offers a function of its own; `appcore` and `cli` call only that. |
| A file of a piece, package or lesson format | `crates/content` reads and writes it; session offers it to `appcore` and `cli`. |
| A detection or signal step | `crates/dsp` (no I/O), wired in `engine/src/input/`; measured with `kontra-cli korpus measure` against `testdata/korpus/`. |

A change that needs more places than listed is a finding for the audit.

## Known deviations
- Session hands engine names on unchanged (`crates/session/src/lib.rs`, `service.rs`,
  `plugin_host.rs`), among them the driver names in `DRIVER_NAMES_PASSED_ON` of the layer test:
  the plugin host's data types, error types and the test seams `Http`, `SecretStore`,
  `MemoryStore`. The rest are mostly the corpus tools the CLI uses. The plugin host's data types
  (`PluginInfo`, `PluginFormat`) live in `crates/drivers/src/plugins.rs`, not in `model`.
  `AUDIT.md` finding 10.
- The user interfaces and session read and write score, audio and package files themselves
  where content should, for example `crates/appcore/src/package.rs` (export, open, unpack),
  `appcore/src/bundled.rs` (generated exercise MIDI), `crates/cli/src/main.rs` (`exercises`,
  `piece`), `cli/src/course.rs` (course pack) and `crates/session/src/courses.rs` (course media).
- Error reasons from the layers below reach the window in English: an `#[error(...)]` text
  goes into a translated message as `{reason}`, e.g. `crates/drivers/src/audio.rs` ("no audio
  device found") via `crates/appcore/src/calibrate.rs` into `error.audio`.
- `app` decides things that need no Qt: command line parsing (`crates/app/src/args.rs`), theme
  and style choice from the environment (`crates/app/src/main.rs`), value limits and bus names
  in the bridge (`crates/app/src/bridge/settings.rs`, `bridge/transport.rs`).
- QML compares fixed choices with literal texts (`ctl.mode === "workshop"`), so a practice mode
  or style needs more QML places than listed. Kept by the owner (round 23). `AUDIT.md` finding 6.
