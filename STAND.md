# Arbeitsstand

Übergabe zwischen Sitzungen. Stand: 2026-10-14, früh. Was erledigt ist, steht in Lastenheft und Roadmap; hier
steht nur, was offen ist und wo es liegt.

## Offene PRs

Gemerged in der Nacht zum 14.10.: #58, #59, #60, #61, #62, #63 (enthält #64), #65; #64 geschlossen (in #63 enthalten).

- #66 Setliste direkt füllen, Gehörbildung „ohne Fehler“, Latenz „noch nicht gemessen“: Merge nach grüner CI läuft.
- Kante #58 (1.35), #61 (1.36), #62 (1.37 Ghost Note), #63 (1.38 Transport-Kästchen) in dieser Reihenfolge: Eigentümer
  merged. Danach `scripts/sync-kante.sh`, `loopStart`/`loopEnd` an tabStaff, bassStaff, combinedBass, combinedTab
  binden (G9).

## Offen

- **Hardware-Prüfung 13.10.** (ROADMAP H1–H8): Dauertest mit dem Stand vor #58 (Puffer 1024): 55 Xruns in 60 min,
  LH-NF-04 nicht erfüllt; während des Tests liefen Builds. Wiederholen ohne Last, mit #58 (Puffer 128), Ausgabe aufs
  Solo per `PIPEWIRE_NODE=alsa_output.usb-Focusrite_Scarlett_Solo_USB-00.analog-stereo`. Offen: H5 Fehlalarme (mittel),
  H6 Slap/Pop (braucht Aufnahmen weiterer Spieler), H7 Zwei-Bund-Slide (Runde 12), H8 Geräteauswahl neben PipeWire.
  Aufnahmen: `../Kontra-integ/testdata/korpus/local/own/check-2026-10-13/` (lokal).
- **Runde 12** gestellt (Artefakt, `antworten/runde-12`): Zwei-Bund-Slides, Startmodus der Plan-Übungen, Stufen-
  reihenfolge, Songsterr-Anfrage, Dauertest, Kirigami-Sprache, pub-Schnittstellen.
- **Flow-Tests im echten Fenster:** Xvfb `:99` + `xdotool` + `import`, App mit `-qwindowgeometry` (bei `QT_SCALE_FACTOR`
  logische Größe), stummer Ausgang `kontra:null` in den Einstellungen, `QT_FORCE_STDERR_LOGGING=1`, `--focus-log`.
- **Flageoletts** (#51): Bewertung oktavtolerant; künstliche ohne eigenes Zeichen in Kante. GP4-Arten 15/17 mit echter
  GP4-Datei gegenprüfen; MusicXML-Naturflageolett ohne Saite/Bund bleibt beim gegriffenen Ton.
- **Zweiter Leser 13.10.** (#50–#54): Befunde und Antworten in den PRs; offen: `heard()`-Testkopie (#50),
  Einschwingen der Brummfilter (~0,6 s ab Start des Eingangs, der mit dem Lauf startet; der Einzähler deckt es, ohne
  Einzähler die ersten Töne) am Gerät prüfen.
- **Stem-Trennung** (LH-BIB-09): mit echtem Demucs gespielt und gemessen (QA-Runde 14.10., PR demucs-gui); im Flatpak
  ausgegraut; offen: Regler für den Original-Bass im Mixer.
- **MIDI-Auswahl** (LH-EIN-12): mit zwei echten Keyboards nicht geprüft.
- **Hardware-Prüfungen vor 0.1**, noch offen: Klang, Transponieren mit Bandaufnahme, echtes MIDI-Keyboard, Mikrofon
  mit Lautsprechern (J-E2), Soak ohne Last, Mithör-Latenz (nach #58), Tastatur/Screenreader am Gerät.
- **Profilwahl „Keyboard“** erst mit den Tasteninstrumenten nach 1.0.

## Neue pub-Schnittstellen (nächste Runde zur Freigabe)

- #48/#49: `Core::home_json`, `Core::loaded_title`, `Library::plain_paths`, `TrackInfo::is_percussion`.
- #50: `TechniqueTarget`, `technique_signs`, `TechniqueResult::sign`, `Annotation::slide_cents`.
- #63/#66: `Calibration::beat_at`, `LiveCalibration::beat`, `LiveTuner::input_block`, `Core::calibration_beat`,
  `Core::setlist_add`; QA-Schalter `--screenshot-notes`, `--focus-log`.
- #58: `Calibration::count_in`, `calibration::COUNT_IN`, `CheckTake::player_offset_ms`. #60: `PitchRevision`,
  `Event::Pitch`, `revise_pitch`, `NoteTracker::pitch_revised`, `Session::on_pitch_revision`.
- demucs-gui: `Core::forget_stems`, `Core::qa_separator`, `store::cache_dir`, `BassRemoval::{size, remove, usage}`,
  `StemJob::remaining`, `SeparationJob::remaining`.
- #52: `Yin::estimate_attack`. #54: `evaluate_laptop_jack` (ersetzt `simulate_laptop_jack`). #56: `Rhythm::Ghost`.

## Für den Eigentümer

- Merge-Erlaubnis als Regel in den Claude-Code-Einstellungen eintragen (Runde 11), Kante #58, #61, #62 mergen.
- Entscheidungs-Artefakt: https://claude.ai/artifact/EsQvZ5epqB3WeBgFEhytpN
- OpenRouter-Schlüssel von PR-Agent: Monatslimit erreicht (Reset 1.11.).
