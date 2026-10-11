# Arbeitsstand

Übergabe zwischen Sitzungen. Stand: 2026-10-10. Was erledigt ist, steht in Lastenheft und Roadmap; hier steht nur,
was gerade offen ist und wo es liegt.

## Runde 7: Antworten des Eigentümers

| Frage | Antwort |
|---|---|
| Neue `pub`-Schnittstellen (OffPitch, off_pitch, Gains::monitor, Input::start mit monitor, Practice::set_gains, Live::can_monitor, content::exercise, dsp::{track_beats, BeatGrid}, .kontra-Pakete, Density, Run::hardest_bars, Arrangement::drums) | alle freigegeben |
| Prüfung mit echter Hardware (Mithör-Latenz, Interface abziehen, 60-min-Soak, Beat-Erkennung an Bandaufnahmen) | später, vor 0.1 |
| Drop bei 5/6 Saiten (Drop A) | so lassen |
| Automatische Eingangswahl | bei jedem Start neu, nichts speichern |
| Daten laden | Ergänzen und Ersetzen anbieten, Ergänzen vorausgewählt (im Dialog prüfen) |
| Übungstitel | in der Oberflächensprache |
| Kante-Wünsche 2 | Claude legt Kante-PR an, Eigentümer merged |
| Als Nächstes | Klangformung fürs Mithören, Gehörbildung, Backing transponieren |

Eingetragen in `ROADMAP.md` (Entscheidungen Runde 7) und `LASTENHEFT.md`.

## Offene Branches

- **lh-klang** (PR #15): Schlagzeug zu den Grooves, Stufe „5 Groove“. Lokal alle Prüfungen grün; wird gemerged, wenn
  die CI grün ist. Nicht mit dem Ohr am Interface geprüft.
- **lh-titel** (nur leerer WIP-Commit): Übungstitel übersetzen. Befund: Der Fortschritt hängt an `song_id`, einem
  Hash über den Titel in der MIDI-Datei und die Noten (`session/src/progress.rs`). Deshalb bleiben Dateinamen, Titel in
  den Dateien und Stufenordner deutsch; übersetzt wird nur die Anzeige. Plan: `ExerciseSpec` je mitgelieferter Übung
  abrufbar machen (Nummer „NN“ im Dateinamen), Titel und Stufennamen in `bundled_json` aus i18n-Schlüsseln bauen,
  `generate_exercise` benennt neue Dateien in der Oberflächensprache.
- **Kante `kontra-wuensche-2`** (nur leerer WIP-Commit, kein PR): Kante 1.28.
  - Saitenfarben: Nur `KanteTabLane` hat `stringColors` (Saite 0 = tiefste, eingebaute Palette). Gleicher Name für
    `KanteTabStaff` und `KanteFretboard`, leer als Vorgabe. Vorschlag: Palette nach `KanteStyle` verschieben
    (Eigentümer fragen).
  - `KanteSegmented` hat keine Deaktivierung; Pfeiltasten gehen einfach zum Nachbarn.
  - Info-Knopf aus Kontras `InfoTip.qml` (KanteToolButton „?“ mit Tooltip) verallgemeinern, Screenreader-Name als
    Text-Eigenschaft statt `texts.t()`.
- **lh-gehoer** (nichts gebaut, kein Branch auf dem Server):
  - Gehörbildung (LH-LRN-03: Ton/kurze Linie nachspielen, Intervalle, Grundton eines Akkords): neue `QuizKind`s in
    `appcore/src/quiz.rs`, `QuizPage.qml` erweitern. Klang: `kontra_dsp::synth` (`render`, `bass_tone`); wie die App
    einen einzelnen Vorschauton spielt, ist noch zu klären (`engine/player.rs`, `session/practice.rs`).
  - Transponieren (LH-WIE-02): Das Backing wird schon einmal vorab gerendert (`BackingTrack::interleaved` in
    `engine/src/backing.rs`, `kontra_stretch::stretch_buffer_with`). Tonhöhe = strecken mit `s/r`, dann
    `kontra_dsp::resample` mit `round(sr/r)`, `r = 2^(n/12)`; die Länge bleibt, die Beat-Map bleibt gültig. Der
    Stretcher kann nur 0,25–1,5, also eventuell zwei Durchgänge. Vorschlag: im Engine-Backing statt neuem Vocoder in
    dsp (Eigentümer fragen).
- **lh-klangformung** (nur leerer WIP-Commit mit Plan): Klangformung fürs Mithören, LH-EIN-08.
  - `dsp/src/biquad.rs` hat RBJ-Biquads, aber nur Tief- und Hochpass. Dazu kommen Low-Shelf 100 Hz, Peak 800 Hz
    (Q 0,7), High-Shelf 4 kHz, Koeffizientenwechsel ohne Zustandsverlust und eine Frequenzgang-Funktion für Tests.
  - Neues `dsp/src/tone.rs`: `ToneShape { low_db, mid_db, high_db, compress }` und `ToneShaper` (drei Biquads und ein
    Peak-Kompressor). Ein Regler `a` (0–1): Schwelle −6−24a dB, Ratio 1+5a, automatischer Makeup, Attack 10 ms, Release
    150 ms. Parameter in Schritten alle 32 Samples nachziehen (kein Knacken).
  - Der Mithör-Ton wird in `Mixer::render` (`engine/src/player.rs`) gemischt; die Tonerkennung liest den Eingang
    getrennt und bleibt trocken. `Gains` läuft über die Befehlswarteschlange: ein `ToneShape`-Feld dort.
  - Settings `tone_low_db`, `tone_mid_db`, `tone_high_db`, `tone_compress` (flach/aus); Bridge `changeTone` und
    `resetTone`; in Mixer.qml die Zeilen Tiefen, Mitten, Höhen, Kompressor und „Neutral“, solange Mithören an ist.
  - Leichte Sättigung (auch in LH-EIN-08) als eigener Folgeschritt (Eigentümer fragen).

## Für den Eigentümer

- Cloud-Umgebung: `GITEA_TOKEN` und Setup-Skript.
- Hardware-Tests vor 0.1 (siehe Tabelle).
