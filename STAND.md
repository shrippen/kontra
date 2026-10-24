# Arbeitsstand

Übergabe zwischen Sitzungen. Stand: 2026-10-24. Was erledigt ist, steht in Lastenheft und Roadmap; hier steht
nur, was offen ist und wo es liegt.

## Offene PRs

`blind-probe` (Didaktik I1, Roadmap nachgezogen). Zuletzt gemerged: #98 (Kante 1.35–1.39, G9).

Plan (Eigentümer, 24.10.): danach Stück-Generator und Etüden wie Lieder, I4-Übungen (leere Saite, nur Timing),
Abnahme LH-BIB-06 messen.

Gemerged seit 14.10.: #90 (STAND), #91 (Stem-Tests nacheinander), #92 (Dateidialoge über das Desktop-Portal),
#93–#97 (Review-Regel: PR-Agent nur auf `/review`, Claude-Agent als Rückfall, siehe `agent.md` und `REVIEW.md`).

## Offen

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
- #52: `Yin::estimate_attack`. #54: `evaluate_laptop_jack` (ersetzt `simulate_laptop_jack`). #56: `Rhythm::Ghost`.

## Für den Eigentümer

- Merge-Erlaubnis als Regel in den Claude-Code-Einstellungen eintragen (Runde 11).
- Entscheidungs-Artefakt: https://claude.ai/artifact/EsQvZ5epqB3WeBgFEhytpN
