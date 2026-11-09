# Regel → Prüfung (W4)

Welche Regel aus `agent.md` und `../CLAUDE.md` eine Maschine prüft und welche nur das Review (`REVIEW.md`).
„Hook“: `scripts/hooks/` (einmal `make hooks`); „CI“: `.github/workflows/ci.yml`, Job `core` oder `app`.
Eine Regel, die eine Prüfung bekommt, wandert hier von „Review“ nach oben.

```text
commit ─► commit-msg (Regeln 1–6) ─► push ─► pre-push (Nachrichten, make check-lite)
                                           └► CI: Nachrichten des Pushs, make check, Korpus, App
```

## Maschinell geprüft

| Regel | Prüfung | Wo |
|---|---|---|
| Commit-Nachricht 1–4, 6 (Leerzeile, Betreff ≤ 72, ab 51 Warnung, groß, kein Punkt, Umbruch 72) | `scripts/commit-msg.py` | Hook commit-msg, pre-push, CI |
| Commit-Nachricht 5 (Imperativ), nur Vermutung | `commit-msg.py`: „Added/Adds/Fixing …“ häufiger Verben scheitern, andere -ed/-es/-ing warnen | wie oben |
| Schichten nur zum Nachbarn darunter | `tests/physical_design.rs` (`crates_depend_only_downwards`) | CI |
| Dateigröße (1200 Zeilen, Abbauliste) | `tests/physical_design.rs` (`files_stay_in_budget`) | CI |
| Kein `unwrap`/`expect`/`panic!` außerhalb von Tests | Clippy-Lints im Workspace (`Cargo.toml`, `clippy.toml`) | CI, `make lint` |
| Kein `unsafe` außerhalb von `drivers/` und Brücke | `unsafe_code = "deny"`; Ausnahmen per `#![allow]` sichtbar im Diff | CI |
| cargo fmt, Clippy ohne Warnungen | `cargo fmt --check`, `cargo clippy -D warnings` | pre-push (fmt), CI |
| `{}` auch bei einzeiligem `if` | Rust erzwingt es selbst; QML nur Review | – |
| Lizenzen und Quellen der Abhängigkeiten | `cargo deny check licenses bans sources`; Lizenzliste `scripts/licences.py --check` | pre-push (Liste), CI |
| Bekannte Sicherheitslücken | `cargo deny check advisories` | `advisories.yml` (main, wöchentlich) |
| Kante: keine eigenen Farben, Maße, Schriften | `scripts/check-design.sh` | pre-push, CI |
| Sonderzeichen in einer mitgelieferten Schrift | `tests/glyphs.rs` | CI |
| Texte de und en, gleiche Schlüssel, kein Schlüssel doppelt | `crates/appcore/src/i18n.rs` (`catalogs_have_same_keys_and_placeholders`, `catalogs_have_no_key_twice`) | CI |
| Audio-/Analyse-Thread ohne Allokation | `crates/dsp/tests/expect_no_alloc.rs`, `crates/stretch/tests/no_alloc.rs` (zählender Allocator) | CI |
| Echtzeit: Anteil am Blockbudget | `cargo bench -p kontra-dsp` (E4), CI baut nur | lokal |
| Mitgelieferte Übungen generiert | `kontra-cli exercises` gegen `testdata/exercises` | CI |
| JSON-Schemas aktuell (W8) | `kontra-cli schemas` gegen `schema/` | CI |
| Erkennungsqualität, Meldezeit, Leerlauf | `kontra-cli korpus check`, IDMT `korpus measure` mit Untergrenze | CI |
| Journeys ⚙ | `tests/journeys/`, App-Tests | CI |
| Keine abgeschnittenen Elemente bei P4 und P7 (LH-ANZ-09), Barrierefreiheitsnamen | `scripts/check-layout.sh` (über `scripts/qactl.py layout`) | lokal (W3) |
| Metainfo, Desktop-Datei, Flatpak-Manifest gültig (W6) | `scripts/lint-packaging.sh` (`appstreamcli`, `desktop-file-validate`, `flatpak-builder-lint`) | CI, Flatpak-Job (meldet nur) |
| Release vollständig: Version, Changelog de/en, Metainfo, Landing Page (W6) | `scripts/release.sh X.Y.Z`, Test `scripts/release.test.sh`; Überschriften `scripts/release-notes.sh --check` | Release, CI |

## Nur im Review

| Regel | Warum keine Maschine |
|---|---|
| Wenige Wörter, keine Superlative | Sprache |
| Commit-Nachricht 7 (was und warum) | Inhalt |
| Konstanten statt magischer Zahlen | Ob ein Wert „sprechend“ ist, entscheidet der Leser |
| Wenig Einrückung, frühes `return` | Clippy fängt nur Teile (`collapsible_if`) |
| Funktionsnamen unter 30 Zeichen | prüfbar, noch nicht gebaut |
| Enums statt `bool`-Parameter | Clippy `fn_params_excessive_bools` greift erst ab drei |
| Leerzeilen und Kommentare je Block | Lesbarkeit |
| `pub` nur, wo nötig; neue Schnittstellen im PR nennen | Design |
| Nur ändern, was zur Sache gehört | Diff lesen |
| UI ruft nie Engine oder Treiber | Crate-Kanten maschinell; innerhalb `appcore` nicht |
| Fehler zuerst als roter Test | Reihenfolge der Commits |
| Information nie nur über Farbe | Ansicht (Screenshots, QA) |
| Einheit im Namen (`offsetMs`) | Namen |
| Kante-Kopie unverändert | `scripts/sync-kante.sh` überschreibt; Review prüft den Diff |
| Kein Demo-Code in Auslieferungen | noch kein Release-Build |
| Personas und Journeys im PR | Inhalt |
| CI-Läufe nicht stapeln, Runner-Labels | Workflow lesen |
