# Arbeitsstand

Übergabe zwischen Sitzungen. Stand: 2026-10-25. Was erledigt ist, steht in Lastenheft und Roadmap; hier steht
nur, was offen ist und wo es liegt.

## Offene PRs

#127 `kann-ich-schon` (LH-LRN-10); Kante #77 (1.41 Ziel-Band im Histogramm, wartet auf den Eigentümer; danach Kontra-Branch `pocket-band`).

Lernen (Runde 13, `LERNEN.md`): Plan (#118), Format und Kapitel 1 (#119), Kapitel 2–5 (#121), Kapitel 6–15 (#123),
Ausprobieren per Klick (#124), Wiederholen (#125), „?“ an Fachbegriffen in Übungsziel, Generator und Auswertung (#126)
gemerged. Offen: „?“ am Akkordnamen und an den Technik-Zeichen in
Tab-Lane und Tabulatur (LH-LRN-09, Rest), Kurspakete und Kurswerkstatt (LH-LRN-14, -15, für 1.0).

Bestand: In `testdata/exercises` heißen die Ordner 4 und 5 vertauscht („4 Groove“ hält die Walking-Bass-Übungen,
„5 Walking Bass“ die Grooves); Lektionen verweisen nur auf Nummern und sind nicht betroffen.

Neue pub-Schnittstellen zur Freigabe (nächste Runde): `kontra_content::lesson` (`Lessons`, Blocktypen, `scopes`),
`kontra_session::{Learning, LearnState, LearnStore, UnitState, place, positions}`, `EarQuiz::fixed`, Core
`learn_json`, `lesson_json`, `lesson_seen`, `lesson_ear`, `term_ear`, `lesson_exercise`, `finale_exercise`,
`glossary_json`, `glossary_list_json`.

Plan (Eigentümer, 24.10.): Didaktik I4 (#106), I5 (#107), I6 umgesetzt. Offen aus
I5: Ziel-Band im Histogramm (Kante). I6 Phrasengrenzen aus Zögern: `zoegern`.

CI läuft seit 24.10. auf GitHub (`.github/workflows/ci.yml`, Status `github / test` an Gitea, #102); Gitea führt nur
PR-Agent aus.

Gemerged seit 14.10.: #90–#97 (STAND, Tests, Dateidialoge, Review-Regel), #98 (Kante 1.35–1.39, G9), #99 (I1 Ohne
Hilfe prüfen; dabei: später letzter Ton im Loop zählt für seinen Durchgang, gespeichertes Tempo = gespieltes), #100
(Stück-Generator, Stufe 9), #101 (Beat-Erkennung 12,8 ms zu früh, Abnahme LH-BIB-06), #102 (CI auf GitHub), #103
(CI-Meldung wiederholen), #104 (flakige Treiber-Tests), #105, #106 (I4 erst der Puls), #107 (I5 Lage-Ziel), #108
(I6 Bausteine), #109 (Vorzähler: Klicks 2–4 fehlten nach gesetztem Tempo), #110 (ProgressPage `data` umbenannt), #111
(Stück-Generator: Schwierigkeit), #112, #113 (Kante 1.40), #114 (Generator: nur Timing), #115 (Bausteine nach Phrasen).

## Offen

- **Flakiger Journey-Test** J-C5 (`c5_techniques.rs:107`): unter Last (Load > 120) einmal ein Extra-Ton bei
  Hammer-on/Pull-off (24.10., einmal in `make check`, dreimal danach grün). Reparieren, nicht ausnehmen. 25.10.:
  nicht nachgestellt (172 Läufe bei Load bis 170, auch 8 parallel). Verdacht: Bei `Pace::RealTime` stempelt die
  Engine Noten auf ihre Uhr; ein verspäteter Block verschiebt einen Ton aus dem Fenster.
- **Speicherdialoge:** seit #92 über das Desktop-Portal; Vorschlag (`selectedFile`) und Titel im echten KDE-Dialog
  ungeprüft (Paket, Bericht, Daten).

- **Dauertest ohne Last** (Runde 12): in der nächsten Nacht allein, nach allen Builds, auf dem Solo; jetzt über den
  PipeWire-Host (#73) mit Puffer 128 und Echtzeit-Priorität. Vorher 55 Xruns in 60 min (Puffer 1024, Builds nebenher).
- **Scarlett-Eingang** ist wieder ein PipeWire-Knoten (24.10., Quelle und Senke). Offen: am Gerät prüfen, dass
  Latenz und Pegel stimmen (H1, H8).
- **Offen aus der Hardware-Prüfung:** H5 Ausklingen der leeren Saite als Extra (zählt nicht mehr in Quoten), H6
  Slap/Pop (Aufnahmen weiterer Spieler), Echtzeit-Priorität ohne `realtime`-Gruppe (rtkit nur über MPL-Crate).
- **Songsterr-Suche** (#71/#73): undokumentierte API; ändert Songsterr sie, sagt Kontra „unlesbar“. Flatpak hat jetzt
  `--share=network`, `--filesystem=xdg-download:ro`, `xdg-run/pipewire-0` (Flathub-Prüfung beachten).
- **Flow-Tests im echten Fenster:** Xvfb `:99` + `xdotool` + `import`, App mit `-qwindowgeometry` (bei `QT_SCALE_FACTOR`
  logische Größe), immer `--input-file` und Ausgang `kontra:null`, `QT_FORCE_STDERR_LOGGING=1`, `--focus-log`.
- **Flageoletts** (#51): Bewertung oktavtolerant; künstliche ohne eigenes Zeichen in Kante. GP4-Arten 15/17 mit echter
  GP4-Datei gegenprüfen; MusicXML-Naturflageolett ohne Saite/Bund bleibt beim gegriffenen Ton.
- **Zweiter Leser 13.10.** (#50–#54): Befunde und Antworten in den PRs; offen: `heard()`-Testkopie (#50),
  Einschwingen der Brummfilter (~0,6 s ab Start des Eingangs, der mit dem Lauf startet; der Einzähler deckt es, ohne
  Einzähler die ersten Töne) am Gerät prüfen.
- **Stem-Trennung** (LH-BIB-09): mit echtem Demucs gespielt und gemessen (QA-Runde 14.10., PR demucs-gui); im Flatpak
  ausgegraut; Regler „Original-Bass“ im Mixer (#76), am Gerät noch nicht gehört.
- **MIDI-Auswahl** (LH-EIN-12): mit zwei echten Keyboards nicht geprüft.
- **Hardware-Prüfungen vor 0.1**, noch offen: Klang, Transponieren mit Bandaufnahme, echtes MIDI-Keyboard, Mikrofon
  mit Lautsprechern (J-E2), Soak ohne Last, Mithör-Latenz (nach #58), Tastatur/Screenreader am Gerät.
- **Profilwahl „Keyboard“** erst mit den Tasteninstrumenten nach 1.0.

## Neue pub-Schnittstellen (nächste Runde zur Freigabe)

- #48/#49: `Core::home_json`, `Core::loaded_title`, `Library::plain_paths`, `TrackInfo::is_percussion`.
- #50: `TechniqueTarget`, `technique_signs`, `TechniqueResult::sign`, `Annotation::slide_cents`.
- Runde 12 und danach (freigegeben in Runde 12 bis #66; neu): `startup_language`, `Settings::piece_mode`,
  `Library::move_folders`, `DeviceInfo::past_pipewire`, `audio_priority`/`Priority`, `Http`/`Web`/`FileHttp`.
- #63/#66: `Calibration::beat_at`, `LiveCalibration::beat`, `LiveTuner::input_block`, `Core::calibration_beat`,
  `Core::setlist_add`; QA-Schalter `--screenshot-notes`, `--focus-log`.
- #58: `Calibration::count_in`, `calibration::COUNT_IN`, `CheckTake::player_offset_ms`. #60: `PitchRevision`,
  `Event::Pitch`, `revise_pitch`, `NoteTracker::pitch_revised`, `Session::on_pitch_revision`.
- #76: `BackingAudio::{stereo, with_bass}` (jetzt ein Typ statt `Arc<Vec<f32>>`), `Gains::bass`,
  `BackingTrack::with_bass_of`, `LiveSetup::original`, `Settings::bass_volume`.
- demucs-gui: `Core::forget_stems`, `Core::qa_separator`, `store::cache_dir`, `BassRemoval::{size, remove, usage}`,
  `StemJob::remaining`, `SeparationJob::remaining`.
- blind-probe (I1): `Mode::Probe`, `Run::spread_ms`, `Session::played_speed`, `Core::{start_probe, is_probing, bars_at}`,
  `pub(crate) Matcher::has_pitch`; Brücke: Invokables `startProbe`, `loopAt`, Properties `probing`, `autoprobe`;
  `AppArgs::probe` (QA-Schalter `--probe`).
- chaining (I6): `Chain` (`new`, `step`, `index`, `count`, `is_last`, `on_pass`), `ChainOrder`,
  `Core::{start_chain, stop_chain, chain_json}`, `UiEvent::ChainStep`; Brücke: Property `chainJson`, Invokable
  `startChain`.
- laid-back (I5): `Pocket` (`key`, `from_key`, `band_ms`, `judge`), `PocketVerdict`, `Settings::pocket`;
  Brücke: Property `pocket`, Invokable `changePocket`.
- #106 (I4): `Judging`, `Arrangement::judging`, `ExpectedNote::any_pitch`, `ExerciseSpec::judging`,
  `pub(crate) TIMING_TEXT`.
- phrase-blocks: `Blocks` (`Bars`, `Phrases`); `Chain::new(a, bars, blocks, order, w)`, `Core::start_chain(blocks, order)`;
  Brücke `startChain(blocks, order)` mit "off" | "2" | "4" | "phrases". #114: `exercise::Description::judging`.
- #111: `exercise::Level` (`ALL`, `key`, `from_key`, `Default`); `ExerciseKind::Song(Style, Level)`.
- #100: `exercise::Style` (`ALL`, `key`, `from_key`, `minor`), `ExerciseKind::Song`.
- #52: `Yin::estimate_attack`. #54: `evaluate_laptop_jack` (ersetzt `simulate_laptop_jack`). #56: `Rhythm::Ghost`.

## Für den Eigentümer

- Merge-Erlaubnis als Regel in den Claude-Code-Einstellungen eintragen (Runde 11).
- Entscheidungs-Artefakt: https://claude.ai/artifact/EsQvZ5epqB3WeBgFEhytpN
