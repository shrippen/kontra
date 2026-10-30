# Arbeitsstand

Übergabe zwischen Sitzungen. Stand: 2026-10-30. Was erledigt ist, steht in Lastenheft und Roadmap; hier steht
nur, was offen ist und wo es liegt.

## Übergabe (30.10.2026, Branch `device-clock`, PR #209, ersetzt #186)

R3 erster Teil: Gerätestempel über eine DLL (`engine/src/device_clock.rs`), Verluste unter 4 ms zählen nicht, Sperre
am Wiedereinsatz nur nach echtem Verlust mit klingender Saite, 10 ms, im Onset-Detektor (Eigentümer, Runde 17,
Option 1). Neuer PR statt #186. Neue `pub`: `OnsetDetector::mute`, `NoteTracker::mute_onsets` (kontra-dsp); aus #186
`Event::{Stalled, Resumed}`, `Session::on_stalled`, `Session::on_resumed`, `PracticeEvent::{InputStalled,
InputResumed}`. Offen: Prüfung am Solo (Aussetzer erzwingen, Latenz und Xruns).

## Übergabe (30.10.2026, Runde 17)

Runde 17 beantwortet (ROADMAP, „Entscheidungen Runde 17“): Doppelgriffe über Mikrofon aus (Branch `runde-17`),
Meldezeit 70 ms je Note außer Tönen unter E1, Einmessung der Techniken je Bass (LH-BEW-17) geplant. #198 (Brummen,
ohne Drift-Regel) und `device-clock` (PR #209, ersetzt #186) laufen in eigenen Branches. Nächste Sitzung:
Hardware-Prüfung mit dem Eigentümer, geführt. Nächster Schwerpunkt offen; der Eigentümer wird zu R5 (pYIN) erneut gefragt.

## Übergabe (29.10.2026, Harmonicon-Analyse)

Harmonicon untersucht, 39 Vorschläge gewählt (ROADMAP, „Entscheidungen Harmonicon-Analyse“; Lastenheft ergänzt).
Zuerst F1 und F2 (Fehler, je ein PR mit rotem Test), dann nach Wert und Aufwand: A1, A2, L5 (Auswertung), E3, A5, E1,
E2, W2, W5. W2, W4, W6, W7 brauchen beim PR die Bestätigung des Eigentümers in Worten.

## Übergabe (28.10.2026, nach Runde 16)

`main` mit #169–#175: Runde 16 umgesetzt (Kante 1.49 und 1.50, Probenzeichen, Pop in Dreiklängen, Zwischenspeicher,
Pegel in der Notenansicht, Wayland-Regel). QA-Runde 28.10. (J-B5, J-F1) in der ROADMAP, B1 behoben (#175).

Offen:
- Kontra #176 (B2, B3, B5, B7, B9): auf Kante 1.51 umgestellt (`DrawerSkin`, `FocusTrap` entfallen),
  wartet auf Review und Merge.
- Trenner der Seitenleiste für Screenreader stumm schalten: in Kante (`KanteScope.popup`), nicht lokal.
- Danach Hell/Dunkel (M20) als Kante-PR mit Einstellung in Kontra; B8 (niedrig).
- Extreme Vergrößerung und kleine Auflösungen getrennt anschauen: Screenshots je Seite, Ideen, Entscheidung mit dem
  Eigentümer (ROADMAP, QA-Runde 28.10.; enthält B6 vor dem Abspielen).
- pub aus #172, #174, #176 und Kante 1.50 in Runde 17 freigegeben (Liste unten).
- Kleinigkeiten: `pressByKey` ohne `toggle` (Qt < 6.8); Hinweis „Paket geöffnet“ (#164); Paketgröße ohne Test;
  MusicXML `<degree>`/`<offset>`; `staffSpace` ohne Untergrenze (#171); Werkstatt- und Warten-Zeilen in niedrigen
  Fenstern (#175).
- Bruch (#174): vor dem 28.10. erzeugte Stücke tragen Akkorde als MIDI-Marker; neu erzeugen.

## Übergabe (28.10.2026, nach Runde 15)

Keine offenen Kontra-PRs; `main` mit #151–#165. Runde 16 steht im Entscheidungs-Artefakt (Dokument
`antworten/runde-16`).

Offen:
- Kante #86 (1.49: Return auf Knöpfen, `KanteFocusFollow`, `KanteStyle.scheme`, `KanteSteps.skipped`,
  `axisTitle`, Rahmen um falsche Töne): drei Review-Runden, offen ein Wichtig (`scheme` schaltet auch Kante Light
  dunkel). Angehalten nach der Review-Regel; Runde 16 fragt, ob `scheme` herausgenommen wird, und bestätigt das
  Release 1.49.
- Nach Kante 1.49: Kante in Kontra übernehmen (`scripts/sync-kante.sh`), `KanteFocusFollow` auf langen Seiten,
  `skipped` für die übersprungene Latenz in der Einrichtung, Histogramm-`axisTitle` übersetzen; Hell/Dunkel als
  Einstellung mit Schalter im Präsentationsmodus (M20); dann J-F1 und J-B5 erneut spielen.
- Kleinigkeiten aus den Reviews, nicht behoben: `.part`-Reste im Backing-Cache bei Abbruch (#162); Hinweis
  „Paket geöffnet“ nennt Zieltempo und „Fortschritt bei null“ auch, wenn nichts übernommen wurde (#164);
  Grenzen der Paketgröße ohne Test (#164); MusicXML `<degree>` und negativer `<offset>` (#156).
- Offene Befunde mittel: S12, J10 (Teilnamen, Kante-Baustein), J11 (Glossar ohne Bilder/Hörbeispiele).
- Zur Freigabe in Runde 16 gelistet: die neuen pub-Schnittstellen aus #152–#164 und Kante 1.49.

## Übergabe (28.10.2026, QA-Nacht)

QA-Runde 27.10. mit allen Personas (ROADMAP). Die Korrekturen der Nacht sind als #137–#143 und #145–#150 gemerged,
dieser Bericht als #144 zuletzt.

Runde 15 steht im Entscheidungs-Artefakt (Einrichtung, Übungsseite, „Lage“, Instrument, `.kontra`, Kante Light,
Kante-Bausteine, Demo-Modus, Kurs aus Ordner, QA-Schalter, pub, Reihenfolge). Nach den Antworten: Lastenheft und
ROADMAP-Abschnitt „Entscheidungen Runde 15“.

Offene Befunde mittel (ROADMAP, QA-Runde 27.10.): M4, M5, M6, M9, S3, S12, J2, J3, J10.
Offen in Kante (J-F1 hängt daran): S1-Rest (Return löst `KanteButton` nicht aus), S2 (Scrollen zum Fokus), S6 (Deuteranopie).
In `pocket-band` (unten) liegt weiter das Ziel-Band; Kante-Stand der Kopie 1.40, Kante `main` 1.48.

## Übergabe (25.10.2026, abends)

Keine offenen Kontra-PRs. `main` = #135. Kante #77 (1.41 Ziel-Band im Histogramm) wartet auf den Eigentümer;
danach Kontra-Branch `pocket-band` (Worktree `~/.cache/claude/kontra-wt/band`, Target `kontra-band-target`; sonst
keine Worktrees).

Nächste Schritte (Runde 14, `next`: courses):
1. Kurswerkstatt mit Fenster (LH-LRN-15, J-G6, Ziel 1.0): Kurs anlegen, prüfen, packen in der App statt
   `kontra-cli course new|check|pack`. Personas P4 (baut Kurse), P2.
2. Danach die Hardware-Prüfungen (geführte Sitzung mit Bass und Scarlett Solo, siehe „Offen“ unten).
3. In der nächsten Entscheidungsrunde (15) zur Freigabe listen: `CourseStore::add(path, cache)`,
   `CourseStore::subtitles(id, src, cache)`, `PackError::Newer`, `CoursePack::open`,
   `lesson::read_media(path, src)` (vorher Bytes); `Learning::term_texts` filtert jetzt nach Profil.

Eigentümer im Chat (25.10.): „merge alles wenn die runden durch sind“ galt für #131, #134, #135 (erledigt)
und bestätigte deren CI- und packaging-Änderungen; keine Freigabe für spätere PRs.

Bewusst offen gelassen (Review #134/#135, beantwortet):
- Fehler beim Lesen eines Kurses nennt in der App den Pfad nicht (Nutzer hat die Datei eben gewählt).
- Bestand #133: nach einem Xrun füllt der stumme Ausgang sofort einen zweiten Block (Vorfüllen; kein Überschuss).
- Nebenbefund: Tests hinterlassen `kontra-core-*`-Ordner im `TMPDIR` (Core-Tests ohne Aufräumen).

Kurse (LH-LRN-14) gemerged: #131 Format und `kontra-cli course` (Zip über die Crate `zip`), #134 Kurse in der App
(Datei, https-Adresse, Update mit `revision`, Entfernen, Glossar über alle Kurse, Video mit Untertiteln, WebM und
MP4, Dateityp `application/x-kontrakurs`, Qt Multimedia in der CI), #135 Glossar nach Instrument, Kurse von der
Datei statt im Speicher. #133 Null-Ausgang-Test unter Last.

Lernen (Runde 13, `LERNEN.md`): Kapitel 1–15, Glossar, „?“ an Fachbegriffen und Legende an Zeichen (#118–#130)
gemerged. Offen: Kurswerkstatt mit Fenster (LH-LRN-15, für 1.0). Kursfassung `revision` im Manifest: Das Update sagt
neuer, gleich oder älter. Runde 14 beantwortet (ROADMAP): danach zuerst Kurse fertig, dann Hardware-Prüfungen.

Neue pub-Schnittstellen, in Runde 14 freigegeben: `kontra_content::lesson` (`Lessons`, Blocktypen, `scopes`),
`kontra_session::{Learning, LearnState, LearnStore, UnitState, place, positions}`, `EarQuiz::fixed`, Core
`learn_json`, `lesson_json`, `lesson_seen`, `lesson_ear`, `term_ear`, `lesson_exercise`, `finale_exercise`,
`glossary_json`, `glossary_list_json`; `kurspaket`: `lesson::{CoursePack, CourseManifest, PackError, Author,
new_course, pack, COURSE_*}`, `Block::Video::subtitles`, `pub(crate) Zip::{read_text, size}`, CLI `CourseArgs`, `CourseCommand`, `course::run`. `kurs-adresse`: `CourseStore`, `CourseFetch`,
`CourseError`, `Learning::{with_courses, courses, term_sources, term_texts}`, `Http::download`, `lesson::{read_media,
cues, Cue}`, `MAX_COURSE_BYTES`, `Settings::course`, Core `course_preview`, `course_add`, `course_remove`,
`course_fetch`, `course_fetch_poll`, `choose_course`, `lesson_video`, `is_course`, `fake_course_web`,
`set_course_http`; Brücke `coursePreview`, `courseAdd`, `courseRemove`, `courseFetch`, `chooseCourse`,
`lessonVideo`, `startCourse`, `courseFetchJson`. Zur nächsten Runde: siehe Übergabe oben.

Plan (Eigentümer, 24.10.): Didaktik I4 (#106), I5 (#107), I6 umgesetzt. Offen aus
I5: Ziel-Band im Histogramm (Kante). I6 Phrasengrenzen aus Zögern: #128.

CI läuft seit 24.10. auf GitHub (`.github/workflows/ci.yml`, Status `github / test` an Gitea, #102); Gitea führt nur
PR-Agent aus.

Gemerged seit 14.10.: #90–#97 (STAND, Tests, Dateidialoge, Review-Regel), #98 (Kante 1.35–1.39, G9), #99 (I1 Ohne
Hilfe prüfen; dabei: später letzter Ton im Loop zählt für seinen Durchgang, gespeichertes Tempo = gespieltes), #100
(Stück-Generator, Stufe 9), #101 (Beat-Erkennung 12,8 ms zu früh, Abnahme LH-BIB-06), #102 (CI auf GitHub), #103
(CI-Meldung wiederholen), #104 (flakige Treiber-Tests), #105, #106 (I4 erst der Puls), #107 (I5 Lage-Ziel), #108
(I6 Bausteine), #109 (Vorzähler: Klicks 2–4 fehlten nach gesetztem Tempo), #110 (ProgressPage `data` umbenannt), #111
(Stück-Generator: Schwierigkeit), #112, #113 (Kante 1.40), #114 (Generator: nur Timing), #115 (Bausteine nach Phrasen).

## Offen

- **Flakiger Journey-Test** J-C5 (`c5_techniques.rs`): vermutete Ursache (nicht nachgestellt): die Analyse lag
  unter Last mehr als 250 ms hinter der Uhr; die Session wartet jetzt auf sie (`Event::Heard`, #132). Tritt er
  wieder auf, ist es eine andere Ursache.
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
- **Flow-Tests im echten Fenster:** `scripts/qa-session.sh --input-file <Datei> --run '<xdotool …; import -window
  "$KONTRA_QA_WINDOW" shot.png>' -- -qwindowgeometry 1280x800+0+0 …` (bei `QT_SCALE_FACTOR` logische Größe),
  `QT_FORCE_STDERR_LOGGING=1`, `--focus-log`; Fenster erst nach etwa 5 s zeichnen lassen, sonst ist das Bild schwarz.
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
  `Library::find_moved`, `DeviceInfo::past_pipewire`, `audio_priority`/`Priority`, `Http`/`Web`/`FileHttp`.
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
- Runde 17 (`runde-17`): `ChordHearing::OneTone`, `Summary::untold_double_stops`.
- Kleinigkeiten nach R1/R8: `SessionEvent::Reopened` (Note nach einer Rücknahme wieder offen).

## pub-Schnittstellen, in Runde 17 freigegeben

- F3: `DetectedNote::{is_pitchless, stands_out, over_background}`, `Evaluation::pitched`; `pub(crate) yin::power_at`.
- A2: `Summary::recalibrate`, `Summary::{before_beat_hits, after_beat_hits}`.
- R4 (loopback-cal): `LiveLoopback` (`start`, `poll`), `LoopbackResult` (`from_trip`), `LatencyFrom`,
  `Settings::{latency_from, loopback_ms, click_ms}`, `Core::{start_loopback, lean_ms, latency_note}`,
  `pub(crate) Core::{poll_calibration, latency_hint, check_audio_path, audio_path}`, `pub(crate) forget_on_switch`,
  `AudioPath`, `Settings::measured_on`; Brücke: Invokable `startLoopback`, Properties
  `cableMeasuring`, `latencyNote`. `CalibrationResult::Measured::latency_ms` kann jetzt negativ sein.
- R2: `ExpectedStrike` (`new`, `with`, `tones`, `MAX_TONES`), `ExpectedTone`, `NoteTracker::{expect, forget_expected}`,
  `Input::{expect, forget_expected}`, `Expecting`, `evaluate_expecting` (Session: `evaluate_korpus_expecting`),
  `Evaluation::{first_pitch_correct, turned_wrong, shifted_first, shifted_final, report_ms}`;
  `pub(crate)`: `Yin::estimate_expected`, `Session::{input_s, expected}`, `Practice::take_told`, `Told`.
- R7: `StringVerdict`, `StringHeard`, `Unclear`, `Instrument::string_log_b`, `InharmonicityMeter` (`new`, `window`,
  `measure`), `Stiffness`, `StringModel` (`for_instrument`, `with_log_b`, `fret`, `pick`), `Pick`, `FROM_PEAK_S`,
  `WINDOW_S`, `LOG_B_PER_FRET`, `TrackerConfig::strings`, `NoteTracker::string_verdicts`, `Event::String`,
  `tell_strings`/`StringTold` (Session: `tell_korpus_strings`, `read_korpus_strings`), `Session::{on_string,
  strings_heard}` (Einsatz, Durchgang, Note, Urteil), `Summary::strings`, `StringTally` (`of`); `pub(super)`
  `Expectations::near`, `forget_string`. Zurückgenommene Noten bekommen kein Urteil; Urteile überleben das Neuwerten.
- R1: `NoteRetraction`, `NoteTracker::{retracted, told}`, `Event::Retracted`, `kontra_engine::retract`,
  `Session::on_retraction`, `Evaluation::retracted`; `pub(crate)`: `Yin::estimate_short`, `ShortEstimate`,
  `Matcher::release`.
- R8: `DoubleStopVerdict`, `DoubleStopHeard`, `ChordHearing`, `InputSource::chord_hearing`,
  `NoteTracker::double_stops`, `Event::DoubleStop`, `Session::on_double_stop`; Korpus: `generate_double_stops`,
  `announce_second_tone`, `double_stop_mixtures`, `evaluate_double_stops`, `DoubleStopSet`, `DoubleStopStrike`,
  `DoubleStopPlayed`, `DoubleStopTally` (`add`, `detected`, `false_alarms`), `DOUBLE_STOP_INTERVALS` (Session
  reicht sie durch, ohne `DoubleStopPlayed`); `pub(crate)`: `dyad::{DyadCheck, Heard, FRAME_S, SKIP_S}`.
- #172: `CacheEntry`, `BackingCache::{entries, forget, clear}`, `BassRemoval::{entries, forget, clear}`,
  `Core::{caches_json, forget_cached, clear_cache}`. #174: `song::parts`, `CHORD_TEXT`, `BarHead.markers`.
  #176 (QML): `SummarySheet.returnTo`, `.behind`, `GlossaryReturn`. Kante 1.50: `KanteRehearsalMark`,
  `KanteStyle.partText`, `rehearsalText`.

## Für den Eigentümer

- LH-NF-15 (Meldezeit): in Runde 17 entschieden, 70 ms je Note außer Tönen unter E1 (gemessen; heute 69 und 80 ms).
  Am echten Bass offen (Hardware-Prüfung): Fingertipp vor dem Anschlag wird zurückgenommen (etwa 5 min).

- #131 (Kurspakete): drei Review-Runden mit offenem Wichtig (pub-Liste, Zip-Bombe über Deflate, dann über STORED).
  Nach der Review-Regel angehalten; Vorschlag im PR (eigene Zip-Lesung behalten, Größen zentral erzwungen). Merge
  erst nach deiner Entscheidung; die Folgearbeit (Kurse in der App) wartet darauf.

- Merge-Erlaubnis als Regel in den Claude-Code-Einstellungen eintragen (Runde 11).
- Entscheidungs-Artefakt: https://claude.ai/artifact/EsQvZ5epqB3WeBgFEhytpN
