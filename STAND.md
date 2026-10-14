# Arbeitsstand

Übergabe zwischen Sitzungen. Stand: 2026-10-14, mittags. Was erledigt ist, steht in Lastenheft und Roadmap; hier
steht nur, was offen ist und wo es liegt.

## Offene PRs

Gemerged am 14.10.: #58–#63, #65, #66, #68 (Stufen), #69 (Runde 12), #70 (Seitenleiste), #72 (Fehlalarme, Werkstatt-
Quote), #73 (PipeWire-Host, enthält #71 Songsterr-Suche), #74 (MIDI ohne Pegel), #75–#82 (Doku, Original-Bass-Regler,
Mixer-Höhe, Griffbrett in Quiz/Gehör, Stimmung und Mikrofon schmal, Song-Suche schmal, englische Texte, QA-Runde
14.10. Tag). #64 und #71 geschlossen (enthalten).
#67 (CI-Cache) stammt aus einer Kaiwa-Sitzung: nicht anfassen.

- Kante #58 (1.35), #61 (1.36), #62 (1.37 Ghost Note), #63 (1.38 Transport-Kästchen), #64 (1.39 Schritte schmal) in
  dieser Reihenfolge: Eigentümer
  merged. Danach `scripts/sync-kante.sh`, `loopStart`/`loopEnd` an tabStaff, bassStaff, combinedBass, combinedTab
  binden (G9).

## Offen

- **Dauertest ohne Last** (Runde 12): in der nächsten Nacht allein, nach allen Builds, auf dem Solo; jetzt über den
  PipeWire-Host (#73) mit Puffer 128 und Echtzeit-Priorität. Vorher 55 Xruns in 60 min (Puffer 1024, Builds nebenher).
- **Scarlett-Eingang fehlt in PipeWire** seit 13.10. 23:45 (ein Flow-Test hielt das Interface über ALSA): Kontra nimmt es
  jetzt direkt über ALSA und nennt die Abhilfe; Eigentümer: Solo neu einstecken oder `systemctl --user restart
  wireplumber`. Danach am Gerät prüfen, dass es als PipeWire-Knoten erscheint und Latenz/Pegel stimmen (H1, H8).
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
- #52: `Yin::estimate_attack`. #54: `evaluate_laptop_jack` (ersetzt `simulate_laptop_jack`). #56: `Rhythm::Ghost`.

## Für den Eigentümer

- Merge-Erlaubnis als Regel in den Claude-Code-Einstellungen eintragen (Runde 11), Kante #58, #61, #62 mergen.
- Entscheidungs-Artefakt: https://claude.ai/artifact/EsQvZ5epqB3WeBgFEhytpN
- OpenRouter-Schlüssel von PR-Agent: Monatslimit erreicht (Reset 1.11.).
