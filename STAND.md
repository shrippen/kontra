# Arbeitsstand

Übergabe zwischen Sitzungen. Stand: 2026-10-13, abends. Was erledigt ist, steht in Lastenheft und Roadmap; hier
steht nur, was offen ist und wo es liegt.

## Offene PRs

Merges: Runde 11 „Erlaubnis-Regel“; der Auto-Modus lässt den Merge-Aufruf auf Zuruf des Eigentümers durch, die Regel
in den Claude-Code-Einstellungen trägt der Eigentümer selbst ein (Claude darf die eigenen Rechte nicht ändern).
`/review` zweimal gefragt (PR-Agent bis 1.11. aus): grüne CI genügt.

- #58 Hardware-Prüfung H1–H4 (Puffer 128, lautester Eingang, Vorzähler Kalibrierung, `check` mit Spieler-Versatz).
  Vor dem Merge: Loopback mit diesem Stand am Scarlett Solo nachmessen (Ziel < 30 ms).
- #60 Tonhöhe nach dem Einschwingen (Runde 11, LH-BEW-07, J-E3 ⚙); `main` eingemerged (ROADMAP/LASTENHEFT).
- Kante #58 (1.35), #61 (1.36), #62 (1.37 Ghost Note) in dieser Reihenfolge: Eigentümer merged. Danach
  `scripts/sync-kante.sh`, `loopStart`/`loopEnd` an tabStaff, bassStaff, combinedBass, combinedTab binden (G9).

## Offen

- **Hardware-Prüfung 13.10.** (ROADMAP H1–H8): Dauertest mit dem Stand vor #58 (Puffer 1024): 55 Xruns in 60 min,
  LH-NF-04 nicht erfüllt; während des Tests liefen Builds. Wiederholen ohne Last, mit #58 (Puffer 128), Ausgabe aufs
  Solo per `PIPEWIRE_NODE=alsa_output.usb-Focusrite_Scarlett_Solo_USB-00.analog-stereo`. Offen: H5 Fehlalarme (mittel),
  H6 Slap/Pop (braucht Aufnahmen weiterer Spieler), H7 Zwei-Bund-Slide (Runde 12), H8 Geräteauswahl neben PipeWire.
  Aufnahmen: `../Kontra-integ/testdata/korpus/local/own/check-2026-10-13/` (lokal).
- **Runde 12** vorbereiten: H7 (Zwei-Bund-Slides als Hammer-on zählen?), neue pub-Schnittstellen (unten).
- **Flageoletts** (#51): Bewertung oktavtolerant; künstliche ohne eigenes Zeichen in Kante. GP4-Arten 15/17 mit echter
  GP4-Datei gegenprüfen; MusicXML-Naturflageolett ohne Saite/Bund bleibt beim gegriffenen Ton.
- **Zweiter Leser 13.10.** (#50–#54): Befunde und Antworten in den PRs; offen: `heard()`-Testkopie (#50),
  Einschwingen der Brummfilter (~0,6 s ab Start des Eingangs, der mit dem Lauf startet; der Einzähler deckt es, ohne
  Einzähler die ersten Töne) am Gerät prüfen.
- **Stem-Trennung** (LH-BIB-09): mit echtem Demucs nicht gelaufen; im Flatpak ausgegraut.
- **MIDI-Auswahl** (LH-EIN-12): mit zwei echten Keyboards nicht geprüft.
- **Hardware-Prüfungen vor 0.1**, noch offen: Klang, Transponieren mit Bandaufnahme, echtes MIDI-Keyboard, Mikrofon
  mit Lautsprechern (J-E2), Soak ohne Last, Mithör-Latenz (nach #58), Tastatur/Screenreader am Gerät.
- **Profilwahl „Keyboard“** erst mit den Tasteninstrumenten nach 1.0.

## Neue pub-Schnittstellen (nächste Runde zur Freigabe)

- #48/#49: `Core::home_json`, `Core::loaded_title`, `Library::plain_paths`, `TrackInfo::is_percussion`.
- #50: `TechniqueTarget`, `technique_signs`, `TechniqueResult::sign`, `Annotation::slide_cents`.
- #58: `Calibration::count_in`, `calibration::COUNT_IN`, `CheckTake::player_offset_ms`. #60: `PitchRevision`,
  `Event::Pitch`, `revise_pitch`, `NoteTracker::pitch_revised`, `Session::on_pitch_revision`.
- #52: `Yin::estimate_attack`. #54: `evaluate_laptop_jack` (ersetzt `simulate_laptop_jack`). #56: `Rhythm::Ghost`.

## Für den Eigentümer

- Merge-Erlaubnis als Regel in den Claude-Code-Einstellungen eintragen (Runde 11), Kante #58, #61, #62 mergen.
- Entscheidungs-Artefakt: https://claude.ai/artifact/EsQvZ5epqB3WeBgFEhytpN
- OpenRouter-Schlüssel von PR-Agent: Monatslimit erreicht (Reset 1.11.).
