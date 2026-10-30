# Kontra: Roadmap

Phasen und Inhalte: [`KONZEPT.md`](KONZEPT.md), Abschnitt 7. Anforderungen je Phase: [`LASTENHEFT.md`](LASTENHEFT.md).

| Phase | Ziel | Stand |
|---|---|---|
| 0 Machbarkeit | Qt-Brücke, Audio-Engine, Erkennung gegen Testkorpus, eigener Time-Stretch, Latenz, Xruns | weitgehend erledigt (synthetisch); echte Aufnahmen und Latenzmessung am Gerät fehlen |
| 1 MVP (0.1) | E-Bass, Import GP/MIDI, Tab-Lane, Tab, Noten, Spielen/Song/Warten/Werkstatt | große Teile umgesetzt, siehe unten |
| 2 Lernen (0.2–0.4) | Groove, Griffbrett, adaptive Schwierigkeit, MusicXML | vorgezogen und umgesetzt: Groove-Übungen, Griffbrett-Quiz, Generator, Notendichte, Werkstatt-Zielband, MusicXML |
| 3 Inhalte (0.5–0.7) | eigene Audiodatei, `.kontra`-Pakete, Etüden, Gehör | vorgezogen und umgesetzt: Backing-Abgleich (Abnahme LH-BIB-06 synthetisch bestanden, #101), `.kontra`, 150 Übungen in neun Stufen mit Stück-Generator (#100), Gehör; offen: Gegenprobe LH-BIB-06 mit echter Aufnahme |
| 4 Bass komplett (0.8–1.0) | Kontrabass am Mikrofon, Bass über MIDI, Techniken, Stem-Trennung | MIDI-Eingang mit Klaviatur-Ansicht, Mikrofon mit Echo-Kompensation, Techniken erkannt; Rest offen |
| Didaktik (laufend) | Wie vermittelt Kontra Bass am besten? Ideen entwickeln, als Prototypen umsetzen, mit Personas vergleichen | I1, I4, I5, I6 und I7 umgesetzt (24.10.), siehe unten |
| Lernen | Lektionen und Glossar „Was ist …?“: erklären, am Bass ausprobieren, üben, prüfen | Plan [`LERNEN.md`](LERNEN.md), Runde 13 entschieden; Umsetzung ab 25.10. mit Kapitel 1–5 |

## Phase 0

- [x] Cargo-Workspace, `make check` (fmt, clippy, Tests, cargo-deny), CI-Workflow für Gitea
- [x] cxx-qt 0.10 mit Kirigami 6.30 und Kante-QML (Qt 6.10, `scripts/setup-qt-env.sh`)
- [x] Audio-Eingang/-Ausgang mit cpal, allokationsfreie Callbacks, Analyse-Thread
- [ ] Vergleich cpal gegen direkten PipeWire-Treiber (braucht Gerät)
- [x] Onset (Flux mit Perioden-Maximum + Zeitverfeinerung) und Tonhöhe (YIN + Phasenverfeinerung)
- [x] Synthetischer Testkorpus, Prüfung LH-NF-02/03 in CI (`kontra-cli korpus check`)
- [ ] Echte Aufnahmen für den Korpus (Interface, Laptop-Buchse, bundlos)
- [x] Freie Fremdaufnahmen als Regressionsprobe (NSynth, IDMT, FiloBass; `testdata/korpus/QUELLEN.md`,
      `kontra-cli korpus measure`); erste Korrekturen: F1 IDMT 66,6 → 80,8, NSynth 76,5 → 85,7
- [x] IDMT als Regressionsprüfung in der CI (Untergrenzen Recall 81 %, Präzision 77 %, mit Tonhöhe)
- [ ] Oktavfehler bei Kontrabass (FiloBass rund 100 von 842 Noten), leise Töne nach lauten, Laptop-Pegel
      13.10.: Wummern eine Oktave unter dem Ton beim Anschlag erkannt (`Yin::estimate_attack`); FiloBass Oktave zu
      tief 81 → 30, Tonhöhe 57,2 → 64,3 %, IDMT (CI) 89,6/79,7 → 90,3/80,4 %, NSynth Tonhöhe 90,4 → 92,4 %,
      synthetisch unverändert (`testdata/korpus/QUELLEN.md`). Laptop-Pegel: Profil laptop-jack mit Brumm-Kerben und
      niedrigerer Einsatzschwelle, IDMT-Laptop-Simulation 37,9/63,8 → 87,7/89,5 %. Offen: 30 Oktavfehler, leise Töne
      nach lauten, echte Laptop-Aufnahme
- [ ] Latenz Anschlag → Anzeige am echten Gerät messen
- [x] Eigener Time-Stretch (Phasen-Vocoder), Tests synthetisch; Hörvergleich mit Signalsmith offen
- [x] Kante-Bauteile als PR in Kante (#37): Tab-Lane, Tabulatur, Notenzeile, Griffbrett, Stimmgerät, Pegel, Histogramm, Transport

## Phase 1 (Stand)

| Anforderung | Stand |
|---|---|
| LH-BIB-01/02 Import GP3–8, MIDI | umgesetzt, gegen Fremddateien geprüft |
| LH-BIB-03 MusicXML | umgesetzt (vorgezogen) |
| LH-BIB-10 MuseScore `.mscz`/`.mscx` | umgesetzt (14.10.), gegen MuseScore-4.7-Dateien geprüft; Sprungzeichen (D.S., Coda) nur nach Formatbeschreibung |
| LH-BIB-11 Songsterr, MuseScore.com | Suche im Browser, „Neu in Downloads“, Ziehen und Ablegen (14.10., J-B6 ⚙); Songsterr-Suche in Kontra mit Bassspur (Runde 12, 14.10.) |
| LH-BIB-04 Bibliothek | umgesetzt: Suche, Tags, Sortierung, Setlisten, Ordner als Setliste |
| LH-ANZ-01/02/03 Tab-Lane, Tab, Noten | umgesetzt (Kante) |
| LH-MOD-01..04 Spielen, Song, Warten, Werkstatt | umgesetzt, Werkstatt mit Takten, Zielband und Notendichte; Namen von Spielen/Song offen (Runde 11) |
| LH-MOD-05 Metronom, Einzähler | umgesetzt; Lautstärke im Mixer |
| LH-WIE-01 Time-Stretch | umgesetzt; Backing-Track je Stück wählbar |
| LH-WIE-03 Referenzton, Mixer | umgesetzt: Backing, Metronom, Referenzton, Mithören (Standard aus; bei verschiedenen Abtastraten von Ein- und Ausgang kein Mithören, mit Hinweis) |
| LH-BEW-01..05 Bewertung | umgesetzt |
| LH-TUN-01/03 Stimmgerät | umgesetzt |
| LH-EIN-01 Einrichtungsassistent | umgesetzt (5 Schritte, überspringbar, startet die erste Übung) |
| LH-EIN-03/04 Pegel, Laptop-Hinweis | umgesetzt, Hinweise entscheidet der Core (J-A1 ⚙) |
| LH-EIN-05 Latenz-Kalibrierung | „Spiel auf den Klick“ umgesetzt, mit Aufnahme getestet (40 ms ± 3) |
| LH-EIN-06 Bluetooth-Hinweis | umgesetzt |
| LH-FOR-01/03 Fortschritt | umgesetzt: Fortschrittsseite (Tage, Stücke, Abschnitte), Daten sichern und laden |
| LH-ALL-01/02 Sprache, Stil | umgesetzt |
| LH-NF-07 Barrierefreiheit | Kante-Bauteile mit Namen und Formen; Tastaturdurchlauf und echter Screenreader offen (braucht Sitzung am Gerät) |
| LH-ALL-06 Startseite | umgesetzt (#48, 13.10.) |
| Vor 0.1 offen | Hardware-Prüfungen (Runde 10: zuerst), Tastatur/Screenreader am Gerät, Release-Log, Screenshots, Flatpak |

## Entscheidungen Runde 3 (9.10.2026)

- 3D-Highway (nach 1.0): Qt bleibt. Zuerst eine perspektivische Lane (2,5D) in Qt Quick (LGPL, gleiches Fenster,
  Kante-Farben, Screenreader); echtes 3D später neu bewerten. Qt Quick 3D ist in Qt 6.10 nur GPL-3 oder kommerziell.
- Veröffentlichung: Flatpak erst lokal bauen und testen, Flathub nach 0.1.
- Landing Page jetzt im Kontra-Repo (`docs/`), kein Eintrag in der öffentlichen Übersicht bis 1.0.
- Logo: Claude entwirft 10 Vorschläge zur Auswahl.
- Didaktik: Bestandsaufnahme jetzt (`DIDAKTIK.md`), Prototypen nach 0.1.

## Entscheidungen Runde 5 (9.10.2026)

- Rückmeldung: während des Spiels dosiert (Zeichen), ausführlich danach (KONZEPT 5.1 angepasst).
- Adaptive Schwierigkeit (Phase 2): nur zwischen Durchläufen, sichtbar, als Vorschlag (LH-MOD-06).
- Werkstatt: Zielband 80–90 % Treffer statt „3 fehlerfrei“, umgesetzt.
- Didaktik-Prototypen nach 0.1: Blind-Probe, Stellen-Wiederholung über Tage, Groove-Start.
- Testspieler: nur der Eigentümer.
- Eigene Aufnahmen: teilweise, was vorhanden ist (welche Bässe und Eingänge: Runde 6).
- 0.1 ist fertig, wenn Phase 1 vollständig ist; der Erkennungsstand wird im Release-Log offen genannt.
- Zuerst: Werkstatt-Bedienung, Tastatur und Screenreader, Pegel-Assistent (alle drei umgesetzt).
- Flatpak-Bau in der Gitea-CI (geändert: Entscheidungen 10.10.2026).
- Logo: vereinfachte Kleinversion für 16–24 px (umgesetzt).
- „Hinweis“ als eigene Gruppe im Änderungsverlauf: Kante-PR #42 (offen).

## Entscheidungen Runde 6 (9.10.2026)

- Spielen-Modus: keine Auswertung am Ende, nur „Auswertung anzeigen“; der Lauf wird gespeichert (umgesetzt).
- Intonation: eigener Zustand „unsauber“ (richtiger Ton, mehr Cent daneben als die Grenze) mit eigener Form,
  Cent-Mittel in der Auswertung (Logik umgesetzt; Form kommt mit dem Kante-PR).
- Bewegung: der Systemeinstellung folgen, dazu ein Schalter; bei weniger Bewegung Tabulatur statt Lane (umgesetzt).
- Mixer: drei Regler (Backing, Metronom, Referenzton) und zusätzlich der Bass durch Kontra hörbar, abschaltbar,
  aus als Standard (LH-WIE-03).
- Eigene Aufnahmen: 4-Saiter über Interface und über die Laptop-Buchse (`line-4`, `laptop-jack-4`, `sixteenths`
  in `testdata/korpus/AUFNAHME.md`); 5-Saiter, bundlos und Kontrabass bleiben synthetisch oder aus offenen Quellen.
- Kante-Wünsche aus der QA-Runde: ein Kante-PR mit allem, der Eigentümer prüft und merged.
- Neue öffentliche Felder freigegeben: `Bar.key_fifths`, `Evaluation.p95_abs_cents`, `Evaluation.max_abs_cents`.
- Kante-PR #42 („Hinweis“): der Eigentümer prüft und merged.

## Entscheidungen 10.10.2026

- Flatpak-Bau auf GitHub statt in der Gitea-CI: der Gitea-Runner erlaubt in Job-Containern keine
  User-Namespaces, die flatpak-builder braucht (`.github/workflows/flatpak.yml`).
- Mit 1.0 kommt Kontra als öffentliches Repo auf GitHub (Push-Mirror von Gitea); erst dann läuft der
  Flatpak-Bau dort. Bis dahin lokal mit `scripts/flatpak.sh`.

## Entscheidungen Runde 7 (10.10.2026)

- Neue `pub`-Schnittstellen freigegeben: `OffPitch`, `off_pitch`, `Gains::monitor`, `Input::start` mit Mithören,
  `Practice::set_gains`, `Live::can_monitor`, `content::exercise`, `dsp::{track_beats, BeatGrid}`, `.kontra`-Pakete,
  `Density`, `Run::hardest_bars`, `Arrangement::drums`.
- Prüfung mit echter Hardware (Mithör-Latenz, Interface abziehen, 60-Minuten-Soak, Beat-Erkennung an Bandaufnahmen):
  später, vor 0.1.
- Drop bei 5/6 Saiten bleibt Drop A.
- Automatische Eingangswahl: bei jedem Start neu, nichts gespeichert (LH-EIN-02, so umgesetzt).
- Daten laden: Ergänzen und Ersetzen, Ergänzen vorausgewählt (LH-FOR-03, so umgesetzt).
- Übungstitel in der Oberflächensprache (LH-BIB-08); Dateinamen und Titel in den Dateien bleiben, weil der
  Fortschritt an `song_id` hängt.
- Kante-Wünsche 2: Claude legt den Kante-PR an, der Eigentümer merged.
- Als Nächstes: Klangformung fürs Mithören (LH-EIN-08), Gehörbildung (LH-LRN-03), Backing transponieren (LH-WIE-02).

## Entscheidungen 11.10.2026

- Neue `pub`-Schnittstellen sind für Kontra generell freigegeben (Eigentümer im Gespräch, 11.10.2026): keine
  Rückfrage mehr, jede neue Schnittstelle wird im PR genannt (`agent.md`, Schichten).
- Neue `pub`-Schnittstellen der Klangformung freigegeben: `dsp::{EqKind, Shaper, Shaping, SHAPING_RANGE_DB}`,
  `Biquad::{eq, retune, response_db}`, `Gains::shaping` (mit Re-Exporten), `Core::shaping_range_db`,
  `Settings::monitor_{low,mid,high}_db`, `monitor_compress`.

## Entscheidungen Runde 8 (11.10.2026)

- Transponieren: eigener Pitch-Shifter in `dsp` (Echtzeit, Wechsel ohne Neustart) statt Strecken und Umrechnen im
  Engine-Backing (#21 wird umgebaut).
- Transponieren: Töne außerhalb des Instruments rücken eine Oktave (so umgesetzt).
- „Lage“ in der Oberfläche wird „Zeigefinger am n. Bund“ (LH-ALL-05 gilt weiter).
- Saitenfarben: Palette in `KanteStyle`, Tab-Lane, Tabulatur und Griffbrett nutzen sie (Kante-PR).
- Gehörbildung: eigener Regler „Vorspielen“ statt des Referenzton-Reglers.
- Tasteninstrumente jetzt im Modell: `BassKind` wird `InstrumentKind`, ein Instrument hat Saiten oder einen
  Tastenumfang; Test mit Dummy-Keyboard.
- PR-Agent: scheitert das Review, einmal `/review` wiederholen; scheitert es erneut, genügt grüne CI (in allen
  `agent.md`).

## Entscheidungen Runde 9 (11.10.2026)

- Nächster Schwerpunkt: Techniken anzeigen und bewerten (LH-BEW-06, Teil 2) vor Release 0.1.
- Techniken zählen in einer eigenen Auswertung („Slide: 8 von 10“); Ton und Timing zählen wie bisher.
- Stem-Trennung: externes Programm (Demucs), wenn installiert, mit Anleitung zum Installieren; Kontra liefert
  nichts davon mit, ohne das Programm ist die Funktion ausgegraut und erklärt (LH-BIB-09).
- Mehrere MIDI-Geräte: Auswahl unter Anschluss, Wahl wird gespeichert; fehlt das Gerät, nimmt Kontra das erste und
  sagt es (LH-EIN-12).
- Hardware-Prüfungen: später, erst weitere Funktionen.
- Kante 1.31 (Technik-Zeichen) darf Claude nach grüner CI und sauberem `./build.sh` selbst mergen (Plan vom 11.10.,
  abgenommen).

## Entscheidungen Runde 10 (12.10.2026)

- Nächster Schwerpunkt: Hardware-Prüfungen vor Release 0.1, mit einem Prüfmodus, der jede Messung aufzeichnet
  (Latenz per Loopback, Aufnahmen für den Korpus, Soak).
- Ghost Notes und Hammer-on-/Pull-off-Töne in der Trefferquote: bleibt wie umgesetzt (LH-BEW-06).
- Slide-Zeichen und sein Ergebnis an der Ausgangsnote, wie in gedruckter Tabulatur; bewertet wird weiter an der
  Zielnote (LH-BEW-06, LH-ANZ-01).
- Dead Notes (x) werden als gedämpft bewertet; Ghost Notes ( ) bekommen nur ihr Zeichen (Kante 1.32 „x“).
- Stem-Trennung im Flatpak: später anders lösen (z. B. eigene Erweiterung); bis dahin ausgegraut (LH-BIB-09).
- Demucs installiert der Eigentümer; Claude misst.
- Umgesetzt: Prüfmodus `kontra-cli check` (Loopback-Latenz, „auf den Klick“, geführte Aufnahmen als Korpus-Paare mit
  sofortiger Messung, Soak, Demucs-Zeit, Bericht mit Handprüfliste; `testdata/korpus/AUFNAHME.md`). Software-Schleife
  über PipeWire (Null-Senke): 26,9 ms Rundlauf, 12 von 12 Tönen. Die Prüfungen am echten Bass macht der Eigentümer.

## Entscheidungen Runde 11 (13.10.2026)

- Merges: Erlaubnis-Regel für `tea api … /pulls/N/merge` (trägt der Eigentümer in die Claude-Code-Einstellungen ein;
  Claude darf die eigenen Rechte nicht ändern). Am 13.10. auf Zuruf alle PRs #45–#57 gemerged.
- Modi umbenannt: „Spielen“ → „Frei spielen“, „Song“ → „Durchlauf“ (LH-MOD-01/02); Kürzel 1–4 bleiben (umgesetzt).
- Ghost Note: nur der Bund in Klammern, wie gedruckte Tabulatur; das Ergebnis trägt ein Rahmen um den geklammerten
  Bund (Kante-PR).
- Startseite: Karte „Stellen von gestern“, die zwei schwierigsten Takte der letzten Tage, ein Klick in die Werkstatt
  (erster Didaktik-Prototyp, LH-ALL-06).
- Tastenkürzel Strg+1…9 in der Reihenfolge der Navigation: bleibt.
- Slide über einen Bund, als Hammer-on gehört: zählt als gespielt, wenn die Partitur Slide sagt, und umgekehrt
  (LH-BEW-06). Am echten Bass auch über zwei Bünde (Hardware-Prüfung H7): Frage für Runde 12.
- Tonhöhe nach dem Einschwingen korrigieren (`pitch_revision = revise`): umgesetzt (LH-BEW-07, J-E3 ⚙). Anzeige so
  schnell wie bisher, die Note wird bis etwa 160 ms nach dem Anschlag neu bewertet, nur nach oben. FiloBass: Halbton
  zu tief 88 → 37, Oktave zu tief 29 → 14, keine richtige gekippt, Tonhöhe 64,3 → 75,1 %; IDMT ±80 ms R/P 90,3/80,4 →
  91,2/81,2 %; eigene Aufnahmen unverändert (testdata/korpus/QUELLEN.md, „Tonhöhen-Revision“).
- Hardware-Prüfung gemeinsam: am 13.10. begonnen (Abschnitt Hardware-Prüfung unter QA-Runden).

## Forschung zur Audio-Engine (29.10.2026)

Neun Punkte aus Literatur und Praxis, je als Prototyp gemessen (vorher/nachher, gleiche Daten; Berichte in
[FORSCHUNG-AUDIO.md](FORSCHUNG-AUDIO.md)). Eigentümer 29.10.: umsetzen, soweit es trägt.

- [x] R1 Meldezeit (LH-NF-15): YIN ab dem Anschlag mit wachsendem Fenster (1 Periode, Bestätigung), das volle Fenster
  revidiert in beide Richtungen, auch nur die Cent. Prototyp: IDMT Median 79 → 61 ms, `line-4`/`line-5` p95 81 → 69,
  100 → 80 ms, Endgenauigkeit gleich; erste Meldung 1–2,5 Punkte öfter falsch, Fingertipp vor dem Anschlag wird Note.
  Nur zusammen mit „Note zurückgenommen“ und vorläufigem Urteil in der Session. Umgesetzt in #206 (LH-BEW-15):
  freie Schätzung ab dem Anschlag mit Lags bis 1,06 Perioden, zweimal gleich; `NoteRetraction` für einen früh gemeldeten Vorläufer, die Session
  nimmt das Urteil zurück. `korpus check` p95 `line-4` 81 → 69, `line-5` 100 → 80 ms (LH-NF-15: 4 Saiten erfüllt).
  IDMT p50/p90 90,5/124,8 → 75,5/111,0 ms, FiloBass p50 88,5 → 86,5 ms; Endgenauigkeit IDMT 95,3 → 95,3 %, FiloBass
  74,5 → 74,5 %, mit `--expect` IDMT 95,5 → 95,3 %; erste Meldung IDMT 94,7 → 93,6 %, FiloBass 64,4 → 61,2 %. Falsche
  Noten weniger (IDMT 1060 → 1052 Meldungen), 5 zurückgenommen. Lags über zwei Perioden (Oktave darunter sichtbar)
  gaben den Gewinn fast ganz zurück (`line-5` 97 ms). R2-Nachtrag: Flageoletts im Korpus nicht mehr erwartet, Zahlen
  unverändert.
- [x] R2 Erwartete Note an die Erkennung, nur als Torwächter der ersten Meldung, nie als Entscheider: locker gewertet
  machte sie aus falsch gespielten Noten zwei- bis dreimal so viele Treffer (IDMT +1: 16 → 47, FiloBass −1: 53 → 97).
  Grundlage für R7 und R8. Umgesetzt in #203 (LH-BEW-14):
  Session → Eingang (rtrb) → `NoteTracker::expect`; `korpus measure --expect [--expect-shift N]`. Ohne wachsendes
  Fenster (R1) ist der Gewinn klein: erste Meldung IDMT 94,7 → 95,1 %, FiloBass 64,4 → 65,2 %, IDMT Median
  90,5 → 85,2 ms; Endgenauigkeit 95,3 → 95,5 % bzw. 74,5 → 74,4 %. Falsche Erwartung (±1, ±12, +5): Endurteil nie
  öfter auf der verschobenen Tonhöhe als ohne; erste Meldung bis +6 (FiloBass −1: 104 → 110), danach korrigiert.
  Toleranzen auf der Hälfte der Dateien bestätigt, auf der anderen gleich.
- [ ] R3 Gerätezeitstempel (cpal `capture`/`playback`) mit DLL statt Anker am ersten Callback. Simulation: heute
  37 ms Fehler bei anderem Quantum als beim Kalibrieren, zwei Geräte nach 5 min p95 24 ms; mit Stempeln < 0,2 ms.
  Offen: Semantik der Stempel am Scarlett Solo (Loopback, etwa 15 min mit dem Eigentümer).
- [x] R4 Latenz per Loopback oder Gerät, nicht per Klick: die Klick-Kalibrierung zieht die Neigung des Spielers ab
  (15 ms früh gilt als „Auf den Punkt“). Klick zeigt danach nur „deine Neigung“; LH-EIN-14 und LH-BEW-09 passen sich an.
  Umgesetzt in #202 (LH-EIN-05): „Mit Kabel messen“, die Klicks zeigen danach die Neigung.
- [x] R5 pYIN statt fester Schwelle: abgelehnt, bei gleicher Meldezeit schlechter (IDMT 91,6 → 90 %, Leerlauf
  41 → 48/min); der Vorteil kam nur vom Warten.
- [x] R6 Techniken ehrlich messen: IDMT-SMT-Bass (Einzelnoten, drei Bässe) als zweiter Satz; Slap 80 → 42 %,
  Pop 66 → 15 % auf fremden Bässen. Schwellen auf beiden Sätzen wählen; Merkmale nach Abeßer erst mit mehr Bässen.
  Umgesetzt 29.10. in #204 (`tech-sb`): Konverter `idmt-sb`, `korpus techniques` je Bass mit Zeilen für Palm Mute,
  Vibrato und Bending (müssen normal bleiben), `make korpus-real`. Richtig je Bass (Fame = Bass der Linien / Warwick /
  Yamaha): Slap 67 / 19 / 44 %, Pop 30 / 1 / 14 %, Dead Note 99 / 97 / 95 %, normal 98 / 100 / 90 %.
  Nebenbefund „Vibrato 15,6 % als Muted, Bending 8 %“ kam vom Konverter des Prototyps: Er setzte den Einsatz auf ein
  leises Antippen der Saite 100–400 ms vor dem Anschlag, Kontra hörte dort richtig eine tonlose Note. Mit dem Einsatz
  am Anschlag: Vibrato 99,8 %, Bending 100 %, Palm Mute 100 %; normale Noten als Technik 9,7 → 1,8 %.
  Schwellen bleiben: Neu gewählt auf Linien + Einzelnoten (Raster über alle fünf Schwellen, je ohne einen Bass gewählt
  und an ihm geprüft) gewinnt Slap nur auf Kosten von Pop oder normalen Noten. Bestes Maß (Mittel der F1): Einzelnoten
  Slap 43 → 59 %, Pop 15 → 17 %, aber normale Noten als Technik 1,8 → 9,2 %, Linien 1,4 → 6,1 %. Mit der Bedingung
  „nicht mehr Fehlalarme als heute“: Slap 43 → 53 %, Pop 15 → 8 %, Linien-Pop 66 → 20 %. Die zwei Merkmale
  (Höhenanteil, Abklingen) tragen nicht über Bässe: am Warwick wird ein Pop zu 79 % als normal gehört.
  Offen: Merkmale nach Abeßer mit Klassifikator, erst mit Aufnahmen weiterer Bässe und Spieler (`AUFNAHME.md`), sonst
  lernt er einen Bass.
- [x] R7 Saite erkennen über Inharmonizität (Prototyp 29.10.: 93,8 % auf fremden Bässen bei Zufall 41,5 %, echte
  Linien 76 %, Palm-Mute nicht); Urteil etwa 280 ms nach dem Anschlag. Kandidaten aus R2. Umgesetzt (LH-BEW-16): `InharmonicityMeter` und
  `StringModel` (dsp), Prüfung im `NoteTracker` nach jeder Note mit Saite aus der Tab, `StringVerdict` bis zur Session,
  Zeile in der Auswertung; a_Saite im Profil (`Instrument::string_log_b`, nur E1/A1/D2/G2 bundiert).
  `kontra-cli korpus strings [--held-out]`, Kandidaten bundiert 0–20 (Zufall 32,8 %): IDMT-SMT-Bass je Bass mit Werten
  der anderen beiden 97,8 % richtig bei 93,0 % mit Aussage (Bass 1/2/3: 100/95,2/97,8 %), tiefster Bund 48,5 %;
  IDMT-ST-Linien 93,2 % bei 58,6 % nach R1 (< 0,2 s 90 % bei 39 %, 0,2–0,5 s 93 % bei 60 %, > 0,5 s 96 % bei 67 %).
  Palm-Mute: Abklingen über 30 dB/s in den ersten 100 ms und in der Tab markiert ohne Aussage, die übrigen weiter
  beim Zufall. Die Schwelle 14 Teiltöne ist auf IDMT-ST gewählt (mit 6: 84 % bei allen Noten). Notenerkennung
  unverändert (`korpus check`, `measure` IDMT-ST/FiloBass, `techniques` gleich wie vorher). Offen: Messung am Bass
  des Eigentümers (andere Mensur und Saiten verschieben a_Saite; Kalibrierung aus gespielten Noten), Anzeige je Note
  in der Lane, tiefe H-Saite und andere Stimmungen.
  Nacharbeit (30.10., Branch `followups`): Eine Note, deren erste Meldung einen Halbton daneben lag (flacher
  Anfang), bekam keine Saitenprüfung; seit der Tonhöhen-Korrektur wird sie neu geplant. IDMT-ST geprüft 648 → 654
  Noten, 93,2 % richtig bei 58,6 % unverändert; IDMT-SMT-Bass je Bass gleich (Bass 2 466 → 470 Noten, 95,2 %);
  `korpus check` gleich. Die Auswertung nennt Noten ohne Saitenwerte auch neben erkannten (Fünfsaiter).
- [x] R8 Doppelgriffe erkennen, wenn die Tab beide Töne vorgibt (Klapuri 2006): 95 % erkannt, 2,1 % Fehlalarm;
  Oktaven nicht. Teil von M1 (LH-BEW-11). Umgesetzt in #205: `DyadCheck` im Tracker, Urteil `DoubleStopVerdict` je Anschlag,
  Session wertet beide Töne (beide → zwei Treffer, einer → der andere ausgelassen). `korpus check`: synthetisch
  168/168 beide, Oktaven 0 Aussagen, Einzelnoten 0 % (4 Sätze + room-double); `korpus double-stops` IDMT:
  Mischungen 82,1 % (307, davon 44 ohne Urteil), Einzelnoten im ganzen Stück 1,7 % als beide, 9 von 3680 als der
  andere Ton. FiloBass (Kontrabass, Demucs-Stems): 77,8 % / 7,4 % Fehlalarm, offen; über Mikrofon aus (Runde 17,
  nach der alten Zählung). Anschläge ohne Urteil zählen seit der Nacharbeit zu #205 im Nenner mit (vorher IDMT
  95,8 % von 263, 1,8 %; FiloBass 12,3 % Fehlalarm). Unbewiesen: echte Doppelgriffe (Aufnahme mit dem Eigentümer,
  ~15 min).
  Nacharbeit (30.10., Branch `cleanup`): Die 44 Mischungen ohne Urteil waren ein Messfehler. Die Ausschnitte wurden an
  den Annotationen hart geschnitten (Taktraster, meist 20–50 ms vor dem Anschlag, mitten in der Welle); der Knack am
  Schnitt füllt das 40-ms-Gedächtnis des Onset-Detektors, der Anschlag danach hebt sich nicht ab (41 der 44: auch ein
  Ton allein so geschnitten ohne Onset). Ausschnitte blenden jetzt 2 ms ein und aus: IDMT 82,1 → 91,5 % (307, ohne
  Urteil 44 → 13), Einzelnoten unverändert 1,7 %, 9 als der andere Ton; FiloBass unverändert 77,8 % / 7,4 % (dort
  keine Mischung ohne Urteil). Die übrigen 13: Annotation weiter als 80 ms vom Anschlag oder Anschlag vor dem Schnitt
  (Diagnose mit Schnitt am erkannten Anschlag: 0 ohne Urteil, 95,4 %); eine Grenze der Annotation, nicht der Erkennung.
- [x] R9 Time-Stretch: kürzerer Steuer-Vorlauf (win/16) spart 37 ms (Standard) bzw. 75 ms (Fine), Messwerte gleich;
  Doku: Fine kostet 133 ms mehr, nicht 85. Zwei Auflösungen: abgelehnt. Umgesetzt in #200: Standard 133 → 96 ms,
  Fine 267 → 192 ms. Hörtest B0 Standard gegen Fine offen.

## Entscheidungen Harmonicon-Analyse (29.10.2026)

Harmonicon (tcanabrava/harmonicon, MIT, Bevy, Mundharmonika) untersucht; Vorschläge in
https://claude.ai/artifact/XMBQvJcxcUE71NqwDeBdBG, Auswahl in `auswahl/harmonicon`. 39 von 40 gewählt, W10 (Skills
und CLAUDE.md je Crate) abgelehnt. Code übernimmt Kontra kaum (Erkennung grob, 46-ms-Raster), Methoden und Messungen
schon. Bevy: kein Wechsel; Fernsteuerung über AT-SPI, Hot-Reload nur für Kurse und QML.

Fehler (zuerst roter Test):
- [x] F1 Volle Eingangs-Ringe verwerfen Samples stumm, spätere Zeitstempel zu früh (`crates/engine/src/input.rs`, #181).
- [x] F2 Schwierigste Takte nach Fehlerzahl (LH-BEW-03, #182); offen: „Stelle üben“ aus dem dichtesten Fenster.
- [x] F3 Falschnoten im Leerlauf (LH-NF-16, gefunden mit E2, 29.10.): Jeder Knackser, jedes Kratzen und Dämpfen löste
  einen Anschlag aus, die Tonhöhe kam vom Netzbrummen (50 Hz, MIDI 31,35) oder fehlte. `idle-line` 41, `idle-room` 26
  Falschnoten je Minute statt höchstens 1. Behoben: Eine Tonhöhe, die schon 200 ms vor dem Anschlag klar zu hören war
  und bei ihrer Frequenz nicht um 12 dB lauter wird, ist die des Hintergrunds (`over_background`); die Session wertet
  solche Anschläge und tonlose nur, wo eine Note fällig ist. Eine Pegelregel für die noch klingende letzte Note scheiterte
  in beide Richtungen (Brummen durch, dann Wiederanschlag tonlos; Review #193). Jetzt 0 je Minute; IDMT 90,1 % / 80,6 %
  wie vorher, synthetische Sätze gleich. Der saubere Weg ist Brummen vor der Erkennung zu entfernen (E8).

Rückmeldung und Lernen:
- [x] A1 Eine Beobachtung je Durchlauf (LH-BEW-09, #184).
- [x] A2 Hinweis „Neu kalibrieren?“ (LH-EIN-14): in der Auswertung mit Knopf zur Kalibrierung in den Einstellungen;
  nicht, wenn das gewählte Timing-Ziel die Neigung deckt (laid-back).
- [ ] A3 Improvisation bewerten (LH-BEW-13).
- [ ] A4 Adaptive Dichte über Abschnitts-Anteile (LH-MOD-06).
- [ ] L1 Stellen wiederholen (LH-LRN-16).
- [ ] L2 Übungsserie, standardmäßig aus (LH-FOR-05).
- [ ] L3 Übungsleiter je Lektion (LH-LRN-17).
- [ ] L4 Griffbrett-Quiz gewichtet (LH-LRN-18). Teil 1 (`fret-quiz`): „Töne finden“ zieht Plätze nach Gewicht
  (verfehlt und lange nicht geübt öfter), Statistik je Saite und Bund in `quiz.json`, Bünde 0–4, 0–7, 0–12 oder eigene,
  Überspringen kein Fehler, auch nicht in „x von y gefunden“; Bund 12 bei 0–12 kein eigener Platz (gleicher Ton wie
  die Leersaite); `quiz.json` im Datenexport (J-C10 ⚙). Offen: Heatmap mit Zahl (Kante-`KanteHeatmap` hat keine
  Zahlen, Achsen, Tooltips; erst in Kante), Gewichtung der Intervalle im Gehörtraining.
- [x] L5 Intonation als Richtung und Streuung (LH-BEW-12).
- [ ] L6 Verstimmte Saite erkennen (LH-TUN-05).
- [ ] L7 Liedkarte, Kante-Baustein (LH-FOR-06).
- [ ] L8 Ruf und Antwort (LH-LRN-19).
- [ ] L9 Reagierende Band (LH-WIE-05).
- [ ] L10 Ziel je Lektion (LH-LRN-20).
- [x] L11 Shuffle-Klick (LH-WIE-04): Shuffle-Grooves und „Stück Shuffle“ klicken auf 1. und 3. Triole (3. leiser),
  auch im Einzähler. Offen: Shuffle-Angaben importierter Dateien (Guitar Pro „Triplet Feel“, MusicXML `<swing>`)
  werden nicht gelesen; dort stehen die Noten gerade, die Bewertung erwartete sie dann ebenfalls gerade.

Erkennung und Messung:
- [x] A5 `korpus confusion`: Verwechslung nach Intervall und Register. IDMT (29.10., ±80 ms): C1–B1 90,8 % richtig
  (10 „other“, 11 verpasst), C2–B2 92,3 % (13 Halbtöne daneben), C3–B3 68,2 %, C4–B4 12,5 % (11 von 16 zwei Oktaven
  tiefer gehört: Flageoletts, die Bewertung lässt sie in jeder Oktave gelten). Oktavfehler bei tiefen Tönen kaum (3 von
  346 und 3 von 542).
- [x] E1 Meldezeit je Note im Korpus (LH-NF-15): `korpus check` zeigt „p95 rep“. Erste Messung (29.10.): line-4 und
  laptop-jack-4 81 ms, line-5 und fretless-5 100 ms (tiefe H-Saite), sixteenths 82 ms, techniques 271 ms (Legato
  ohne Anschlag). Ziel 70 ms damit verfehlt; bis zur Entscheidung des Eigentümers eine Grenze bei 110 ms gegen
  Verschlechterung. Runde 17: 70 ms je Note, Töne unter E1 gemessen (LH-NF-15).
- [x] E2 Sätze ohne Noten, Falschnoten je Minute (LH-NF-16): `idle-line`, `idle-room` (60 s, Brummen, Rauschen, Knackser, Kratzen, Dämpfen); Ziel seit F3 erreicht.
- [x] E3 Wächter und dauerhafter Eingangszustand (LH-EIN-15): Wächter und Pause der Bewertung (#209); Anzeige
  (`input-state`): verbunden mit Namen unter dem Pegel, „kein Signal“, „gestört“ und „kein Zugriff“ bleiben mit
  „Erneut versuchen“ in der Leiste unter jeder Seite (J-F4 ⚙). „Kein Zugriff“ erkennt Kontra an der Meldung des
  Systems (EACCES, EPERM, „Permission denied“); am Solo mit entzogenem Recht noch nicht gesehen. Hinweise zu
  Aussetzer und Wiedereinsatz kommen jetzt nur noch im Gehörtraining als Meldung.
  Verlust aus Gerätestempeln über eine DLL (R3, erster Teil, #209); offen aus R3: Wiedergabe- und
  Eingangszeitachse an der DLL verankern (Ausgangsstempel), Loopback-Prüfung am Solo.
- [ ] E4 Criterion-Benches für den Echtzeitpfad (Anteil am Blockbudget; CI baut nur).
- [ ] E5 Notenende (LH-BEW-10).
- [x] E6 Rauschboden bei der Kalibrierung (LH-EIN-16, `noise-floor`): Beim Einzählen misst der Onset-Detektor den
  Pegel im Anschlagband (10. Perzentil), die Schwelle liegt 6 dB darüber (−62 bis −40 dB) und gilt je Eingang.
  Messung (synthetisch, weißes Rauschen): Am Kabel gab es schon vorher 0 Falschnoten je Minute bei −70 bis −35 dBFS
  (die Spitzenwahl vergleicht mit dem eigenen Mittel); die Schwelle steigt bei −50 dBFS auf −53, bei −40 auf −43 dB,
  gefunden wie vorher (leise Anschläge verliert das Rauschen selbst, nicht die Schwelle). Gewinn am Mikrofon (P6) im
  leisen Raum: die feste Schwelle −48 dB verlor leise Anschläge, jetzt −62. `room-double` auf −30 dBFS Spitze bei
  Rauschen −80 dBFS: Recall 14,7 % → 100 %, auf −40 dBFS 0 → 57,7 %; `idle-room` weiter 0 Falschnoten. Korpus
  (`korpus check`, IDMT, Techniken, FiloBass) unverändert: er läuft ohne Kalibrierung.
- [ ] E7 Erste echte Aufnahmen als eingefrorene Basis, neue als eigene Gruppen.
- [x] E8 Brummen vor der Erkennung entfernen (Eigentümer, 29.10.): Netzfrequenz verfolgen (49,8–50,2 Hz, 60 Hz),
  Grundton und Obertöne abziehen statt sperren; Noten auf derselben Frequenz (G1, G2) bleiben (`crates/dsp/src/dehum.rs`,
  für jeden Eingang vor Anschlag und Tonhöhe). Gelernt wird nur in ruhigen, stetigen 100-ms-Blöcken, abgezogen erst nach
  2 s stetigem Brummen; ein gehaltener G1 (5 s) wird nicht gelernt. Knackser auf Brummen sind jetzt tonlos statt
  `over_background` mit 31,35. Neuer Satz `hum-line` (E1 bis A2 auf Brummen −40 dBFS): Cent-Mittel 3,68 → 1,76, p95
  12,35 → 6,57, max 17,8 → 9,1 (vor R1; mit der frühen Meldung aus R1 2,05 / 7,58 / 10,76). IDMT, IDMT am Laptop-Eingang, Techniken und FiloBass unverändert (IDMT neu umgewandelt: 91,6 % / 81,9 %),
  übrige Sätze gleich, 0 Falschnoten je Minute. Rechenzeit etwa 2 ms je Sekunde Audio. Hört das Brummen auf oder wird leiser, endet der Abzug nach zwei ruhigen
  Perioden (40 ms), nicht erst nach 1 s (sonst Phantom-Brummen, Review #198). Den Sprung bis dahin fängt die
  F3-Regel ab: Sie prüft das Signal vor dem Abzug. Schnelle Drift (49,8→50,2 Hz in 7 s) kann nach einer Note als
  Legato-Ton auf dem Raster auftauchen; bleibt offen als Kleinigkeit (Eigentümer, Runde 17), Legato wie vorher. Ohne Brummen tauchten im
  Leerlauf Tonhöhen aus Abdämpf-Geräusch auf (idle-room 2 je Minute); tote Noten (`Muted`) gelten deshalb abseits der
  Noten nicht mehr als eigene Note. Die Hintergrund-Regel von F3 bleibt als zweite Linie: für die ersten Sekunden, bevor
  das Brummen gefunden ist, und für die noch klingende Note (ohne sie scheitern zwei F3-Tests). Offen: Bei Spiel ohne
  Pause länger als 8 s wird nicht mehr abgezogen (das Netz driftet vom Modell); weiterlernen unter Noten.

Mehrere Töne (offen für Gitarre und Tasten):
- [x] M1 Doppelgriffe: Korpus-Satz, Merker „zwei Töne“, Fähigkeit je Eingang (LH-BEW-11). Mit R8: synthetischer
  Satz in `korpus check`, IDMT-Mischungen in `korpus double-stops`, `ChordHearing` je Eingang. MIDI-Akkorde als
  ganze Menge bleiben M2.
- [ ] M2 Akkorde im Korpus als ganze Menge werten, zuerst MIDI.
- [ ] M3 Saiten-Plausibilität für Akkordkandidaten im Instrumentprofil.
- [ ] M4 Machbarkeitsstudie NNLS mit Vorlagen je Saite, Log-Frequenz (nach 1.0).

Werkzeuge (W2, W4, W6, W7 ändern `agent.md`, CI oder Release: Bestätigung in Worten je PR):
- [ ] W1 `scripts/qactl.py` über AT-SPI; Socket nur im Cargo-Feature `qa`.
- [x] W2 `scripts/qa-session.sh` für jeden isolierten Testlauf.
- [ ] W3 Layout-Prüfung je Seite bei P4- und P7-Auflösung (LH-ANZ-09).
- [ ] W4 Tabelle Regel → Prüfung, Git-Hooks, CI-Prüfung der Commit-Nachrichten.
- [x] W5 Test für Crate-Kanten und Dateigröße mit Abbauliste (`tests/physical_design.rs`, 1200 Zeilen je Datei; Abbauliste: appcore/lib.rs, bridge.rs, exercise.rs, PracticePage.qml, tracker.rs).
- [ ] W6 `scripts/release.sh`, metainfo `<releases>`/`<screenshots>`, AppStream- und Flatpak-Lint.
- [x] W7 `cargo deny` mit `[advisories]`, eigener Job und wöchentlich (`advisories.yml`); rustls auf 0.23.46, RUSTSEC-2025-0044 (minimp3) mit Begründung ignoriert.
- [ ] W8 JSON-Schemas für Pakete und Kurse aus den serde-Strukturen.
- [ ] W9 Kurs aus Ordner live (LH-LRN-21); QML von der Platte in Entwicklung.
- [x] W11 Glyphen-Test für Sonderzeichen (`tests/glyphs.rs`): jedes Zeichen über ASCII aus i18n (de, en), Lektionen
  und eigenem QML muss in einer mitgelieferten Schrift stehen (Kante: Rajdhani, JetBrains Mono, Bravura; Private-Use
  nur Bravura). Eigener `cmap`-Leser (Formate 4 und 12), keine neue Abhängigkeit. ♯ ♭ ♮ deckt nur Bravura.

## Entscheidungen Runde 17 (30.10.2026)

- #198 Brummen: die Drift-Regel (`Dehum::stale`) entfällt, der Phantom-Fix bleibt (Branch `hum`). Schnelle Drift
  der Netzfrequenz bleibt eine Kleinigkeit (E8).
- #186 Eingangswächter: Gerätezeitstempel mit DLL glätten (R3), die Bewertung pausiert erst nach echtem Datenverlust
  (LH-EIN-15). #209 aus Branch `device-clock` ersetzt #186.
- [x] LH-NF-15 Meldezeit: Ziel 70 ms je Note für alle Instrumente, außer Tönen unter E1 (etwa die tiefe H-Saite eines
      Fünfsaiters); die brauchen allein etwa 65 ms Signal, ihr Ziel gilt gemessen. Das Ziel gilt je Note, nicht je
      Instrument. Stand R1 (#206): `line-4` p95 69 ms, `line-5` p95 80 ms.
      `korpus check` prüft das seit 30.10. (Branch `cleanup`) je Note, ab E1 und unter E1 getrennt: `line-4` 69,3 ms
      erfüllt; verfehlt und als Ausnahme mit gemessenem Wert geprüft: `line-5` ab E1 74,7 ms, `hum-line` 72,0 ms;
      unter E1 94,7 ms. Offen: Entscheidung, ob das Ziel dort nachgearbeitet oder die Ausnahme bestätigt wird.
- [x] R8 Doppelgriffe über Mikrofon aus, bis Aufnahmen da sind (FiloBass 12,2 % Fehlalarm): `ChordHearing::OneTone`,
      ein Doppelgriff zählt dort wie ein Ton; Einstellung „Mikrofon“ und Auswertung sagen, dass Doppelgriffe über
      Mikrofon noch nicht nach beiden Tönen bewertet werden (LH-BEW-11, J-B7).
- [x] Techniken auf anderen Bässen (R6: Slap 18–44 %, Pop 1–14 % auf zwei fremden IDMT-Bässen): Einmessung je Bass
      (LH-BEW-17) zurückgestellt, die Machbarkeit scheiterte an den Merkmalen (Warwick Slap 26 %, Pop 9 %; Linien
      11,6 % Fehlalarm). Bis dahin sind Slap und Pop über Kabel nur Hinweise: gezeigt, nicht gewertet (LH-BEW-06, J-C5,
      Branch `tech-hint`). Einmessung erst mit Aufnahmen mehrerer Bässe (`AUFNAHME.md`); Slaps und Pops des Eigentümers
      kommen in der Hardware-Sitzung dazu.
- [ ] Hardware-Prüfung mit dem Eigentümer in der nächsten Sitzung, geführt (R3-Stempel, R1-Fingertipp, R8 am echten
      Bass, Latenz und Pegel am Scarlett Solo).
- Neue pub-Schnittstellen aus F3, A2, R4, R2, R1, R8, #172, #174, #176 und Kante 1.50 freigegeben.
- Nächster Schwerpunkt offen: Der Eigentümer wird zu R5 (pYIN) erneut gefragt; der Prototyp vom 29.10. brachte
  keinen Gewinn (`FORSCHUNG-AUDIO.md`, „pyin (Punkt 5)“).

## Entscheidungen Runde 16 (28.10.2026)

- [x] Kante #86: `scheme` herausgenommen, Rest nach Review gemerged; Kante 1.49 veröffentlicht, in Kontra #170. Hell/Dunkel als eigener Kante-PR mit der Einstellung in Kontra (M20).
- [x] Probenzeichen: Kante-Baustein, dann Generator und MIDI-Import (mehrere Marker je Takt zusammengefasst,
      LH-BIB-12, J10; Kante 1.50, #174).
- [x] Pop und Ballade in Dreiklängen, Septimen bei Funk und Shuffle, Rock in Powerchords (LH-BIB-12, J10, #169).
- [x] Einstellungen → Daten: Backing-Cache und getrennte Songs mit Größe, je Eintrag löschen, „alles leeren“
      (LH-BIB-09, #172).
- [x] Notenansicht zeigt Pegel und Toleranz immer, die Noten schrumpfen (S12, #171).
- [x] Einstöckiges „a“ (S14): kein Kante-Fehler. Die QA-Läufe erbten die KDE-Systemschrift des Eigentümers
      (Rajdhani); Kante setzt Rajdhani nur in Großbuchstaben (`kanteHeading`). Rajdhani bleibt, nichts zu ändern.
- Neue pub-Schnittstellen aus #152–#164 und Kante 1.49 freigegeben.
- [x] Regel gegen App-Starts auf dem echten Desktop in `agent.md` (bestätigt im Chat, #168).
- Danach J-F1 und J-B5 erneut spielen, dann Hell/Dunkel.

## Entscheidungen Runde 15 (28.10.2026)

Nach der QA-Nacht 27.10.:

- [x] Einrichtung fragt am Ende nach dem Stand (LH-EIN-13): Lektion 1, Übungen der Stufe oder Bibliothek (#161).
- [x] Einstellung „Einfache Ansicht“ (LH-ANZ-11), für „Neu am Bass“ an; Ziel der Übung unter dem Titel (#161).
- [x] „Lage“ als Timing-Ziel heißt in der Oberfläche „Timing“ (LH-BEW-08, #154).
- [x] Instrument jetzt eigens wählbar: E-Bass, Kontrabass, Bass über MIDI (LH-EIN-09, #163).
- [x] `.kontra` mit Loops und Zieltempo, ohne Fortschritt; Pakete können Backing Tracks enthalten (LH-BIB-07, #164).
- [ ] Kante Light ist die abgespeckte Kante (Systemfarben), nicht hell. Hell/dunkel als eigene Einstellung, im
      Präsentationsmodus als Schalter (LH-ALL-02).
- [ ] Fehlende Kante-Bausteine als Kante-PR; Claude merged Kante-PRs nach Review wie Kontra-PRs (`agent.md`).
      Kante #86 (1.49) nach drei Runden mit offenem Wichtig (`scheme` in Kante Light) angehalten; Runde 16.
- Demo-Modus „Studio Weber“ nach 0.1; „Kurs aus Ordner“ bleibt bei der Kurswerkstatt (1.0).
- [x] QA-Läufe mit `--input-file`: kein Portal-Dialog, kein Browser, `kontra-cli` mit stummem Ausgang (#159).
- Neue pub-Schnittstellen aus #137, #138, #141 und #147 freigegeben.
- Danach zuerst die offenen Befunde der QA-Runde 27.10., dann J-F1 und J-B5 erneut spielen. Befunde M4, M5, M6,
  M9, S3, J2, J3 behoben (#152–#162); J-F1 und J-B5 warten auf Kante 1.49 (Fokus, Return, hell/dunkel).

## Entscheidungen Runde 14 (25.10.2026)

Kurspakete (LH-LRN-14):

- Zip über die Crate `zip` statt eigener Lesung, für alle Archive (`.gp`, `.mxl`, `.mscz`, `.kontra`,
  `.kontrakurs`); Größengrenzen bleiben in `content` (#131).
- `.kontrakurs` als MIME-Typ `application/x-kontrakurs` anmelden; Doppelklick zeigt erst die Vorschau.
- Videos in WebM und MP4: das Flatpak bekommt die FFmpeg-Erweiterung.
- Die CI installiert Qt Multimedia und prüft den Videospieler.
- Neue pub-Schnittstellen aus #131, #132 und „Kurse in der App“ freigegeben.
- Danach zuerst Kurse fertig (Aktualisieren, Entfernen, dann Kurswerkstatt), Hardware-Prüfungen danach.

## Entscheidungen Runde 13 (25.10.2026)

Lernmodul ([`LERNEN.md`](LERNEN.md), LH-LRN-07 bis -15, Journeys J-G1 bis J-G5):

- Name „Lernen“; keine Videos in mitgelieferten Kursen; Pfad empfohlen, nichts gesperrt; keine Abzeichen; Claude
  schreibt die Lektionen, der Eigentümer liest je Kapitel Stichproben; Lektionen neben den Übungsstufen.
- Alle 15 Kapitel vor 0.1, Beginn mit Kapitel 1–5 (Eigentümer, 25.10.).
- Kurse je Instrument aus einem gemeinsamen Lektionsvorrat (Eigentümer, 25.10.): `for` (any, Familie, Instrument),
  Varianten, relative Tonhöhen, `needs`, ein Glossar; Bass-Kurs zuerst (LERNEN.md Abschnitt 12).
- Kurse nachladen aus Datei oder Internet, auch mit Videos, auch Bezahlkurse von Anbietern; mitgeliefert nur ohne
  Videos (Eigentümer, 25.10., LERNEN.md Abschnitt 13, LH-LRN-14). Videos bevorzugt über das Internet eingebunden;
  ein Glossar über alle installierten Kurse, doppelte Begriffe mit allen Texten und ihrer Herkunft.
- Kurswerkstatt für 1.0 (Eigentümer, 25.10.): kleines Begleitprogramm zum Anlegen, Prüfen und Packen von Kursen
  (LH-LRN-15, LERNEN.md Abschnitt 13).

## Entscheidungen Runde 12 (14.10.2026)

- Zwei-Bund-Slides: Slide und Hammer-on/Pull-off zählen bis zwei Bünde füreinander (LH-BEW-06, umgesetzt).
- Übungen aus dem Plan öffnen im „Durchlauf“ (Auswertung, „Nächste Übung“); eigene Stücke behalten den zuletzt für
  sie gewählten Modus (umgesetzt, `Settings::piece_mode`).
- Stufenfolge (`level_order = swap`): 4 Groove vor 5 Walking Bass, Walking ist schwerer (umgesetzt: Ordner getauscht,
  Dateinamen und Nummern gleich; Bibliothek und Setlisten ziehen beim Laden auf die neuen Pfade um, Fortschritt bleibt).
- Songsterr: Suche aus Kontra heraus erlaubt, ein Klick öffnet weiterhin die Seite (eigener PR).
- Dauertest ohne Last: in der nächsten Nacht allein, auf dem Solo, nach allen Builds.
- Kirigami folgt Kontras Sprache: Kontra installiert Kirigamis Qt-Übersetzung beim Start (`cpp/translator.cpp`);
  vorher stand „Close Sidebar“ auch auf deutschen Systemen (umgesetzt; Sprachwechsel wirkt bei Kirigami nach Neustart).
- Neue pub-Schnittstellen der Nacht freigegeben.

## Didaktik: wie Kontra Bass vermittelt (laufend, alle Phasen)

Ziel: nicht nur Noten abfragen, sondern herausfinden, welche Wege zum
Bassspielen bei welchen Menschen am besten wirken, und die besten davon in
Kontra umsetzen. Jede Idee wird klein als Prototyp gebaut, mit den Personas
(P1–P7) durchgespielt und, wo möglich, an echten Übungsläufen gemessen
(Trefferquote, Timing-Streuung, Übungszeit, Abbrüche).

- [x] Bestandsaufnahme: Lehrwerke und Methoden (Bassschulen, Suzuki/
      Gehör zuerst, Groove zuerst, Notenlesen zuerst), Spiele und Apps
      (Rocksmith Riff Repeater und Dynamic Difficulty, Synthesia, Yousician,
      Melodics), Forschung zu Üben (deliberate practice, verteiltes Üben,
      Chunking, Fehler-Feedback). Ergebnis (9.10.2026): [`DIDAKTIK.md`](DIDAKTIK.md),
      7 Thesen, 12 Ideen, Vorschlag für drei Prototypen nach 0.1 (Blind-Probe, Stellen-Wiederholung über Tage,
      Groove-Start mit „vor/hinter dem Beat“)
- [ ] Stand der Ideen (24.10.): I7 Zielband umgesetzt (Runde 5); I5 Lage und Streuung stehen in der Auswertung
      („meist 12 ms zu spät“, „Streuung ±9 ms“), die Aufgabe „bewusst laid-back“ fehlt; I4 über Stufe 4 Groove, aber
      mit Tonhöhe bewertet; I8 teilweise über Gehör („Linie nachspielen“). Reihenfolge (Eigentümer, 24.10.): I1 als
      Knopf nach der Werkstatt und auf der Startseite, dann I4 mit Übungen nur auf leerer Saite, nur Timing bewertet.
- [x] I1 Blind-Probe (24.10., LH-LRN-05, J-C6 ⚙): „Ohne Hilfe prüfen“ in der Werkstatt spielt den Abschnitt einmal
      ohne Noten (Session-Modus `Probe`, endet nach einem Durchgang, Weiterspielen zählt nicht), Auswertung neben dem
      letzten Werkstatt-Lauf derselben Takte; Läufe speichern jetzt auch die Streuung. Startseite „Ohne Hilfe“ nach
      einer Nacht. QA-Schalter `--probe`.
- [x] I4 Erst der Puls (24.10., LH-LRN-06, J-C8 ⚙): Bewertungsart `Judging::TimeOnly` im Arrangement (MIDI-Text
      `kontra:timing`); die Session wertet dann nur den Anschlag. Fünf Grooves der Stufe 1 auf leerer Saite nutzen sie.
      Generator-Schalter „Nur Timing“ für Grooves (24.10.). Schwelle für Bünde: Streuung ≤ ±25 ms (25.10.).
- [x] I5 Lage-Ziel (24.10., LH-BEW-08, J-C2 ⚙): „Lage“ auf der Übungsseite (frei, auf den Punkt, laid-back, vorn);
      die Auswertung sagt, ob die mittlere Lage gehalten wurde, sonst um wie viele ms daneben. Offen: Band im
      Histogramm (Kante), Lage-Ziel je Übung.
- [x] I6 Bausteine (24.10., LH-MOD-08, J-C9 ⚙): Werkstatt teilt die Takte in Bausteine zu 2 oder 4 Takten (1 · 2 ·
      1–2 · 3 …, oder von hinten); ein sitzender Schritt (zwei Durchgänge im Zielband) startet den nächsten mit
      Einzähler. Nach Phrasen aus Pausen (25.10.) und vor Takten, an denen der Spieler in zwei der letzten fünf
      Durchläufe gezögert hat (25.10.).
- [ ] Ideen sammeln und je eine Skizze, z. B.: Groove vor Tönen (erst
      Rhythmus auf einer Saite), Hören vor Lesen (nachspielen ohne Anzeige),
      Lieder in Bausteine zerlegen und zusammensetzen, adaptives Tempo und
      Ausdünnen, Griffbrett-Landkarte über Intervalle statt Bundnummern,
      Mitspielen mit Band ab der ersten Stunde, Rückmeldung zu Timing als
      „vor/hinter dem Beat“ statt richtig/falsch, Wiederholung schwacher
      Stellen über Tage
- [ ] Je Idee: Prototyp hinter einem Experiment-Schalter, QA-Journey mit den
      passenden Personas, Messgrößen festlegen
- [ ] Auswertung und Entscheidung (als Artifact mit Vorschlägen): welche Wege
      werden Kern von Kontra, welche Lernpfade entstehen daraus (Phase 2/3)

## Lastenheft-Umsetzung (10.10.2026)

- [x] LH-MOD-07: Pos1/Bild auf springt an den Loop-Anfang (sonst an den Anfang); `[`/`,` und `]`/`.`
      setzen Loop-Anfang und -Ende auf den aktuellen Takt; Bild ab spielt/pausiert (Fußschalter);
      alle Tasten und ein Fußschalter-Hinweis in den Einstellungen.
- [x] LH-ANZ-02: Takte tragen die geschriebenen Wiederholungszeichen und Volta-Nummern (`Bar`:
      `repeat_open`, `repeat_close`, `endings`); die Tabulatur zeigt `|:` und `:|` im ausgerollten
      Verlauf. Volta-Klammern fehlen in Kante (`KanteTabStaff`), die Daten liegen bereit.
- [x] LH-ANZ-05: Lage je Note aus der Tab-Zuordnung (Zeigefinger-Bund, 4-Bund-Fenster, Wechsel mit
      Vorausschau); unter dem Griffbrett „Lage n“ und ein Lagenwechsel-Hinweis ab einem Takt vorher;
      Fingersatz (Bund − Lage + 1) im Marker der nächsten Note.
- [x] LH-BEW-01/-05: Auswertung mit jeder Note (Takt, Ton, Zustand, ms früh/spät, Cent) –
      die schwierigsten fünf sofort, alle auf Knopfdruck; Cent gehen im Modus volle Markierung als
      drittes Argument an `setNoteState` (wirkt, sobald Kante PR #43 gemergt ist).
- [x] LH-FOR-01: Fortschritt je Abschnitt (Ganzes Stück, „Takte 5–8“): beste Trefferquote mit
      Tempo und Datum; die Bibliothek merkt sich die Takte geübter Abschnitte. Läufe speichern jetzt
      das Tempo der Session (die Werkstatt ändert es selbst; vorher stand das Starttempo drin).
- [x] LH-FOR-02: Werkstatt-Tempo und Trefferquote der letzten 30 Tage je Abschnitt als
      Liniendiagramm mit Achse, Zeitraum und Legende.
- [x] LH-BIB-04: Tags je Stück (in der Bibliothek mit Kantes Tag-Auswahl bearbeitbar, Vorschläge
      aus allen Tags), Suche findet Titel, Interpret und Tags; Sortierung nach zuletzt, Titel und
      Fortschritt.
- [x] LH-MOD-06 (Soll, Phase 2): Notendichte 1–3 (wenige Noten, auf den Schlägen, alle) je Abschnitt,
      sichtbar und fixierbar; nach einem Werkstatt-Durchgang oder Song-Lauf ab 90 % Vorschlag „mehr
      Noten“, unter 60 % „weniger Noten“ – nur als Frage, in den Einstellungen abschaltbar. Die
      Ansichten zeigen nur die verlangten Noten; Fortschritt zählt für das ganze Stück.

## QA-Runden

### QA-Runde 28.10.2026 (J-B5 und J-F1 erneut, nach Runde 16)

P4 am Beamer (1280×720, 150 %, Kante Light), P7 (1920×1080, 200 %, Deuteranopie simuliert, Bewegung „Weniger“, nur
Tastatur); `main` 05f033e, Xvfb, Takes aus `etude-1.mid` (+20 ms und −45 ms, falsche und ausgelassene Noten).
Orca nicht genutzt, statt dessen der AT-SPI-Baum.

- Behoben bestätigt: S1 (Return löst Knöpfe aus), S2 (Seite scrollt zum Fokus), S4 (Fokus auf „Schließen“),
  S6 (Treffer, Fehler, Ausgelassen über die Form), S12 (Pegel in jeder Ansicht), M21, M22.
- [x] B1 (hoch, #175): In niedrigen Fenstern (1280×720 bei 150 %) bleiben den Noten etwa 100 px: Tab schneidet die tiefe
  Saite ab, Noten + Tab überlappen, Lane 4 Saiten auf 70 px. Am Ende eines Laufs drückt der Hinweis „Fertig
  gespielt“ Noten und Tab auf null.
- [x] B2 (mittel, #176 mit Kante 1.51): Die Seitenleiste nimmt den Hintergrund aus der Systempalette; Kante dunkel auf hellem System:
  „Seitenleiste schließen“ 1,4:1, gesperrtes „Üben“ 1,7:1 (L1 nur teils behoben).
- [x] B3 (mittel, #176 mit Kante 1.51): Auswertung ohne Fokus-Käfig: Tab geht in die Seitenleiste, der Screenreader liest die Seite
  dahinter; nach Esc bleibt der Fokus auf dem versteckten „Schließen“.
- [x] B4 (mittel, #178): Einrichtung Schritt 3 bei 200 %: Ton, Cent und Drehrichtung unter dem Seitenfuß, per Tastatur
  nicht erreichbar (Bild ab, Strg+Ende, Pfeile).
- [x] B5 (mittel, #176 mit Kante 1.51): Glossar aus der Übungsliste („?“), dann „Zurück“: Fokus weg, die erste Übung per Tab nicht mehr
  erreichbar.
- [ ] B6 (mittel, beim Spielen behoben #178): Werkstatt bei 200 %: die Regler füllen die Seite, Tab oder Lane nicht zu sehen.
- [ ] Extreme Vergrößerung und kleine Auflösungen (Eigentümer, 28.10.): getrennt anschauen, jede Seite und jeder Modus
  bei 200 % und mehr sowie bei kleinen Auflösungen als Screenshot, dazu Ideen je Seite; mit dem Eigentümer
  entscheiden. Dazu gehört B6 vor dem Abspielen (Vorschlag: Klappe „Einstellungen“).
- [x] B7 (mittel, Kante 1.51, übernommen #176): Saitenfarben der Tabulatur in Kante Light auf hellem Grund 1,7–2,6:1, am Beamer kaum lesbar.
- [ ] B8 (niedrig): Nach Seitenwechsel Fokus auf der ScrollView; Trenner der Seitenleiste als unbenannte
  Listeneinträge (gehört nach Kante: `KanteScope.popup`); Tempo-Regler mit zwei Tab-Halten; nach „Klinke direkt“ zeigt der Eingang weiter „Automatisch:
  Scarlett Solo …“; nach Esc aus der Präsentation Seitenleiste 216 statt 260 px; Saitenbuchstaben und Taktzahlen in
  der Präsentation etwa 8 px; frühe Note (orange) unter Deuteranopie wie eine D-Saiten-Note, nur das kleine Abzeichen
  trägt.
- [x] B9 (niedrig, Kante 1.51, übersetzt #176): Timing-Histogramm nennt dem Screenreader keine Balkenwerte (S5 nur teils).
- Geprüft für #176 (Xvfb, helles Breeze als Systemschema, Tastatur über xdotool, `--focus-log`): B2 Seitenleiste in
  Kante dunkel ohne helle Fläche, „Üben“ gesperrt lesbar; B3 Tab kreist in der Auswertung (sechs Halte, Umschalt+Tab
  vom ersten zum letzten), die Seite dahinter ist verborgen, Esc setzt den Fokus auf „Abspielen“; B5 „Zurück“ setzt
  den Fokus auf das „?“, jede Übung per Tab und Umschalt+Tab erreichbar; B7 Ziffern in Kante Light 4,9:1 (Himbeere),
  5,8:1 (Blau), 5,2:1 (Ocker) auf #eff0f1. B9 nur per QML-Test (Balkenname „+10 to +20 ms: 3 notes“), Orca nicht
  genutzt. QML-Tests: `tst_summarysheet`, `tst_glossaryreturn`.
- Vorfall: eine eigene D-Bus-Sitzung für AT-SPI startete `xdg-desktop-portal-kde` mit dem echten Wayland-Display;
  sofort beendet, nichts erschienen. Regel: eigener Session-Bus nur für den Screenreader-Test, ganz vom Desktop getrennt (#179).

### QA-Runde 27.10.2026 (Nacht: alle Personas, alle Funktionen, echtes Fenster)

Echtes X-Fenster (Xvfb, ohne Fenstermanager), stummer Ausgang, Aufnahmen aus `testdata/korpus/` und synthetische
Takes als Eingang. Vier Durchgänge parallel: P1 Lea (selbst, mit Gesamtdesign), P7 Sam mit P5 Aylin und P6 Karl,
P3 Mira mit P4 Thomas, P2 Jens. Entscheidungen dazu: Runde 15 im Entscheidungs-Artefakt.

- **P1 Lea (J-A1, J-A2, J-B2, J-C8, J-G1): bestanden nach Befunden.**
  - L1 (mittel, behoben #137): Seitenleiste in Systemfarben (grüner Akzent), Lektionslinks grün; jetzt Kante.
  - L3 (mittel, behoben #137): Pegel-Hinweise verschoben „Weiter“; die Knöpfe stehen jetzt fest im Seitenfuß.
  - L5 (mittel, behoben #161): Einrichtung endete immer mit „Groove Viertel“, ohne Frage nach dem Stand (auch M17).
  - L6 (mittel, behoben #161): Übungsseite zeigte einer Anfängerin alle Regler; Ziel der Übung fehlte auf der Seite.
  - L8 (niedrig, behoben #137): Auswertung: „Sitzt“-Karte trug den Titel der nächsten Übung ohne Hinweis.
  - L11 (niedrig, behoben #137): Geführtes Stimmen endete ohne Weg weiter.
  - L15, L16 (niedrig, behoben #137): Lektion: Überschrift doppelt, Griffbrett zu hoch für 768 px.
  - L17 (mittel, behoben #138): „Einstieg“ folgte den Dateinummern, Tonleitern vor dem Puls; jetzt Lernreihenfolge.
  - Offen (niedrig): L2 Auswahl der Anschlussliste schwach sichtbar; L4 übersprungene Latenz mit Haken (behoben, Kante 1.49);
    L7 dieselbe Übung in zwei Startkarten; L9 Histogramm ohne y-Achsentitel (behoben, Kante 1.49); L10 „Zeigefinger am 1. Bund“
    bei leeren Saiten; L12 Callout reserviert zwei Zeilen (Kante); L13 Oktavzahlen (E1) unerklärt; L18 Fortschritt
    ohne Lektionen und Gehör.
- **P7 Sam (J-F1, 200 %, Deuteranopie, nur Tastatur): nicht bestanden.**
  - S1 (hoch, behoben #137 und Kante 1.49): Leertaste startete einen Lauf statt den fokussierten Knopf zu drücken; behoben.
    Return/Enter löste `KanteButton` nicht aus; behoben mit Kante 1.49.
  - S2 (hoch, behoben mit Kante 1.49): Lange Seiten scrollten nicht zum Fokus (Einstellungen, Lernen).
  - S3 (mittel, behoben #153): Fokus in der Seitenleiste unsichtbar, Trennlinien sind Tab-Halte.
  - S4 (mittel, behoben #137): Fokus blieb hinter der Auswertung.
  - S5 (mittel, teils behoben #137): Modus, Ansicht, Suche jetzt benannt; `KanteBarChart` ohne Namen (Kante),
    Glossar-Links im Lektionstext nicht per Tab erreichbar.
  - S6 (mittel, behoben mit Kante 1.49): Bei Deuteranopie glich ein verfehlter Ton (rot) einer D-Saiten-Note (ocker).
  - S7 (mittel, behoben #137): Bibliothek zeigte Ergebnisse erst nach Neustart.
  - S8 (mittel, behoben #154): „Lage“ hieß im Übungskopf das Timing-Ziel, sonst die Handposition; jetzt „Timing“.
  - S9 (mittel, behoben #137), S10, S13, S15 (niedrig, behoben #137): springende Knöpfe, MIDI-Hinweis ohne Keyboard,
    englische Formulierungen, Glossar sortierte „Dämpfen“ nach „Durchgangston“.
  - S11 (mittel, behoben #163): Kein Instrument „Kontrabass“ oder „MIDI-Bass“ wählbar (LH-EIN-09).
  - S12 (mittel, behoben #171): Notenansicht ohne Pegel und Toleranz-Hinweis, untere Hälfte leer; Pegel jetzt immer.
  - S14 (niedrig, Kante): Einstöckiges „a“ der Kante-Schrift liest sich bei 100 % wie „o“.
- **P5 Aylin (J-A4, J-E1), P6 Karl (J-A4, J-E2): bestanden.** J-E2 mit Band im Raum (`room-double-backing`) nicht
  prüfbar: der Abzug zieht nur Kontras eigenes Backing ab.
- **P3 Mira (J-A3, J-B4, J-C2, J-C3, J-C5, J-E3): J-E3 nicht bestanden, Rest bestanden nach Befunden.**
  - M1 (hoch, behoben #139): Bundlos und im strengen Fenster blieb ein von unten einschwingender Ton „falsch“.
  - M2 (mittel, behoben #147): Strenges Fenster, alle Noten 40 ms früh: „0 % getroffen, 128 ausgelassen“ ohne „zu früh“.
  - M3 (mittel, behoben #149): Pocket-Satz widersprüchlich („2 ms zu früh · meist 8 ms zu spät“).
  - M4 (mittel, behoben #152): Enharmonik in B♭: Liste A♯1, Noten D♭ aus C♯.
  - M5 (mittel, behoben #156): MusicXML-Akkordsymbole (`<harmony>`) werden nicht gelesen (LH-BIB-03). Offen
    (niedrig): `<degree>` (z. B. C7♯9, C7sus4) und negativer `<offset>` werden noch nicht gelesen.
  - M6 (mittel, behoben #160): „Alle Noten“ ohne Timing je Note (LH-BEW-01).
  - M9 (mittel, behoben #157): Sechzehntel unlesbar, acht Takte in einer Zeile.
  - M10 (mittel, behoben #140): Generator: Schlusston über sieben Achtel, Takt las sich als 3/4.
  - M15 (mittel, behoben #143): Alt+5 (Klaviatur) fehlte, Tastenliste nannte alte Modi.
  - Offen (niedrig): M7 Liste „zweiter Blick“ unsortiert; M8 Live-Etikett überdeckt Marken; M11 Fähnchen, Hälse;
    M12 „in BB“, ♭ im Dateinamen verloren; M13 Generator vergisst Einstellungen; M14 Akkordantworten mit
    Oktavzahl; M16 Gehör-Versuchsliste unbegrenzt; M18 Laid-back-Hilfe ohne ms.
- **P4 Thomas (J-B5, J-D2, J-G4, J-G6): bestanden nach Befunden; J-D2 nicht prüfbar (Portal-Dialog).**
  - M20 (mittel, Entscheidung): „Kante Light“ bleibt bei dunklem System dunkel (LH-ALL-02).
  - M21 (mittel, behoben #145): Bei 853 px fehlte der Präsentations-Knopf; nur F5 half.
  - M22 (mittel, behoben #145): Nach Esc aus der Präsentation fehlte die Seitenleiste.
  - M24 (niedrig, offen): Dieselbe Etüde doppelt in der Bibliothek (zwei Pfade).
  - M25 (mittel, behoben #164): `.kontra` ohne Loops und Zieltempo, trägt den Fortschritt der Lehrkraft.
  - M26 (mittel, Entscheidung): Kurs nur per CLI anlegbar, Platzhalter „Mein Kurs“ besteht die Prüfung.
  - M27 (mittel, behoben #137): `kontra --help` startete die App mit echtem Eingang.
- **P2 Jens (J-B1, J-B3, J-B6, J-C6, J-C7, J-C9, J-D1, J-G2, J-G5): bestanden nach Befunden.**
  - J1 (mittel, behoben #141): Meldungen nach dem Start (Import, kaputte Datei) erschienen nie.
  - J2 (mittel, behoben #162): MP3-Backing wurde bei jedem Start dekodiert; Debug-Build 20 s eingefroren, Release ~1 s.
    Jetzt Cache, Vorbereitung im Hintergrund, Ladeanzeige.
  - J3 (mittel, behoben #155): Takterkennung rastete auf dem Offbeat ein (+310 ms) und meldete trotzdem „8 von 8 Takten“.
  - J4 (niedrig, behoben #150): Bibliothek zeigte den internen GP-Titel („001“) statt des Dateinamens; Umbenennen fehlt weiter.
  - J7 (mittel, behoben #142): Nach Eingabe in „Takte“ blieben K und Fußschalter tot; Return/Esc geben frei.
  - J8 (mittel, behoben #148): „Ohne Hilfe prüfen“ floss in Werkstatt-Durchläufe und Dichte-Vorschlag ein.
  - J9 (mittel, behoben #146): Erzeugtes Stück erbte Loop und Tempo des vorigen.
  - J10 (mittel, teils behoben #158, #169): Erzeugtes Stück ohne Teilnamen; „Rock in E“ mit maj7-Akkorden. Akkorde behoben
    (Rock Powerchords #158; Pop, Ballade und Pop-Grooves Dreiklänge #169, Runde 16); Teilnamen brauchen Probenzeichen in Kante.
  - J11 (mittel, Inhalt): Glossar ohne Abbildungen und Hörbeispiele.
  - Offen (niedrig): J1b englischer Rohfehler im Kurs-Satz; J5 Mithör-Hinweis ohne ms; J6 Lane 45 px bei Mixer,
    Mithören und Callout; J12 „bestes Ergebnis“ aus Frei spielen; J13 Sammelbefund („1 Stücke“, Schlagzeug-Regler
    ohne Schlagzeug, Start zeigt 0:00 trotz Loop, alter Beat-Map nach Entfernen des Backings).
- **Vorfälle (Testumgebung):** `kontra --help` lief zweimal ~2 min mit dem echten Eingang (behoben, M27); ein
  Ordnerdialog öffnete über das Portal auf dem echten Desktop (`main.rs` setzt das Portal-Theme); Songsterr-Treffer
  öffneten Browser-Tabs; `kontra-cli practice --backing` hat keinen stummen Ausgang. Scarlett-Knoten danach geprüft.

### QA-Runde 14.10.2026 (Tag: Personas im echten Fenster, schmal, 200 %, Englisch)

Echtes X-Fenster wie in der Runde davor; dazu 700 px (halber Laptop), 2560×1440 bei 200 % (P7), 853×533 bei
150 % (P4) und Englisch (J-F2). Stummer Ausgang, Aufnahmen als Eingang, echtes Demucs für J-D1.

- **P2 Jens (J-D1): bestanden.** Neu: Regler „Original-Bass“ im Mixer (#76), mit echtem Demucs getrennt und gespielt.
  - T1 (mittel, behoben #77): Offener Mixer mit Mithören drückte die Lane auf 50 px (siehe H13).
  - T2 (niedrig, Testumgebung): Demucs scheiterte aus Kontra heraus mit CUDNN-Fehler; `scripts/qt-env.sh` mit
    `QT_ROOT=/usr` stellt `/usr/lib` in `LD_LIBRARY_PATH` voran. Betrifft nur die Entwicklungsumgebung.
- **P1 Lea, P4 Thomas (J-C4, Quiz): bestanden nach Befunden.**
  - T3 (mittel, behoben #78): Griffbrett in Quiz und Gehörbildung 160 px, darunter halbe Seite leer; am Beamer
    schnitt die Gehörbildung Griffbrett und Punktzahl ab. Jetzt mit der Breite bis 240, auf kurzen Schirmen kleiner.
- **P7 Sam (J-F1, 200 %, 700 px): bestanden nach Befunden.**
  - T4 (mittel, behoben #79): „½ STEP DOWN“ las sich in der Schmalschrift wie „% STEP DOWN“; bei 700 px liefen die
    fünf Stimmungs-Knöpfe aus dem Fenster. Jetzt „Halbton/Ganzton tiefer“, schmal als Liste.
  - T5 (mittel, Kante #64): Die Schritte der Einrichtung schnitten bei 700 px „LET'S GO“ ab; Kante 1.39 nennt schmal
    nur den aktuellen Schritt.
  - T6 (niedrig, behoben #80): Mikrofon-Hinweis endete mit „…“; die Zahlen stehen jetzt nach der Wahl darunter.
  - T7 (niedrig, behoben #81): Song-Suche schmal mit vier Zeilen, Browser-Knöpfe einzeln mittig.
  - Stimmgerät: Nadel und Text wirkten auf Bildschirmfotos widersprüchlich; Ursache ist die Nadel-Animation
    zwischen zwei Messungen, die Werte stimmen. Kein Befund.
- **J-F2 Englisch: bestanden nach Befunden.** Keine rohen Schlüssel, kein Sprachmix; T8 (niedrig, behoben):
  Formulierungen („the frame sits first“, „Once you found“) und weitere aus einem Durchgang über `en.json`.
- **Nachmittag (P2 J-B3, P4 J-D2, P1 J-B2), lokal, während Gitea keine Pushes annahm:**
  - T9 (hoch, behoben): Ab dem zweiten Schleifen-Durchgang trugen die kommenden Noten die Marken des vorigen
    (verpasst, falsch) vor der Spiellinie; jetzt beginnt jeder Durchgang neutral.
  - T10 (mittel, behoben): Ein Fenster ohne Fokus meldete „kein Download-Ordner“; die Liste lud nur im aktiven Fenster.
  - T11 (niedrig, behoben): Fortschritt „1 min an einem Tag“ nach 22 s, darunter „unter 1 min“; jetzt einheitlich.
  - Speicherdialoge: Vorschlag über `selectedFile`; der Qt-Ersatzdialog zeigt ihn nicht, nativ ungeprüft.
  - Warten-Modus: hält am richtigen Ton, nennt gespielten und gesuchten, Griffbrett mit Bund und Finger. Bestanden.
- Offen (niedrig): Stücke mit zwei Takten stehen in Tab und Noten klein oben links (keine Streckung auf die Breite);
  `--page`/`--mode` mit unbekanntem Wert zeigen eine leere Seite bzw. speichern den Wert (nur QA-Schalter).

### QA-Runde 14.10.2026 (GUI, ganze Abläufe im echten Fenster)

Erstmals im echten X-Fenster gespielt (Xvfb, `xdotool` klickt und tippt, `import` nimmt den ganzen Bildschirm auf),
stummer Ausgang, Aufnahmen als Eingang; Fokuswege mit `--focus-log`. Befunde des Eigentümers vorab: der Latenz-Hinweis.

- **P1 Lea (J-A1, J-A3, J-C1): bestanden nach Befunden.**
  - H1 (hoch, behoben): Hinweise schwebten über der Seite, im Systemschema durchsichtig, Knöpfe dahinter blieben
    klickbar; der Latenz-Hinweis kam eine Sekunde nach Songbeginn, bei jedem Lauf. Jetzt Leiste unter der Seite (neben
    der Seitenleiste), Prüfung einmal je Start, sobald ein Eingang läuft (Stimmgerät, Einrichtung, Üben).
  - H2 (hoch, behoben): „Erste Übung starten“ öffnete die Demo-Etüde statt der ersten Übung des Plans; der Text nannte
    „ein Riff in e-Moll“, „Später, zur Bibliothek“ führte zur Startseite.
  - H3 (mittel, behoben, Test zuerst): Pegel „−7 dB · gut“ über dem Rat „etwas zu laut“; die Anzeige hält jetzt die Spitze
    der letzten 3 s wie der Rat.
  - H4 (mittel, behoben): Üben ohne Stück zeigte eine leere Lane mit bedienbaren Knöpfen; jetzt Leerzustand mit Wegen.
  - H5 (mittel, behoben): Pegel vor dem Spielen „−∞ dB · zu leise“, obwohl Kontra nicht zuhörte.
  - H6 (mittel, behoben): Latenzmessung ohne Anzeige; jetzt „Vorzähler 3“, „Schlag 4 von 8“ mit Balken.
  - H7 (niedrig, behoben): „Durchlauf beendet“ nach „Frei spielen“ (seit Runde 11 ein Modusname).
- **P2 Jens (J-B1, J-B3, J-D1): bestanden nach Befunden.**
  - H8 (hoch, behoben #65): Seit #58 startete die Wiedergabe mit dem Scarlett Solo nicht (ALSA meldet falsche Puffer-
    grenzen); Probe-Stream, sonst Standardblock.
  - H9 (mittel, behoben): Werkstatt zeigte „Takte 1 bis 1“ ohne Schleife, Abspielen lief das ganze Stück; jetzt zwei
    Takte ab der Position.
  - H10 (niedrig, behoben): Dateidialoge öffneten im Arbeitsordner des Programms; jetzt letzter Ordner bzw. Downloads,
    Begleitspuren in Musik.
- **P4 Thomas (J-B5, Beamer 853×533 bei 150 %): bestanden nach Befunden.**
  - H11 (hoch, behoben): In der Präsentation blieb die Seitenleiste, ihr Griff lag über der Zeit; die Noten zeigten eine
    Zeile mit zwei Takten und ein Drittel leer. Jetzt ohne Seitenleiste, zwei Zeilen.
  - H12 (mittel, behoben): Titel wurde neben Modi und Ansichten abgeschnitten; leere Kirigami-Titelleiste kostete 45 px.
- **P7 Sam (Tastatur): bestanden.** Fokusweg der Übungsseite logisch (Kopf, Modi, Ansicht, Transponieren, Dichte,
  Transport), Fokusrahmen überall sichtbar. Offen (niedrig): Kirigamis „Close Sidebar“ folgt der Systemsprache.
  - H13 (mittel, behoben): Offener Mixer mit „Bass über Kontra hören“ drückte die Tab-Lane auf einen Streifen
    (1280×800: 50 px). Jetzt höchstens 40 % der Seite, der Rest scrollt; Tab bringt die ganze Zeile ins Bild.
- Weiter behoben: Qt-Optionen mit Wert (`-qwindowgeometry`) wurden als Datei geöffnet (Test zuerst); Tests spielten auf
  den Standardausgang (jetzt Timer-Ausgang); Stimmungs-Presets als Text, Latenz mit „ms“, Fortschritt „20 s“ statt
  „0.4min“; Metronom/Einzähler als Kästchen statt gelber Knöpfe (Kante 1.38, PR in Kante).

### QA-Runde 14.10.2026 (Nacht: Stem-Trennung mit echtem Demucs)

Gespielt als P2 Jens (J-D1 Schritt 5 ⚙) mit Demucs 4.1.0 (pipx) und einem erzeugten Song (Etüde 1 gerendert,
synthetisches Schlagzeug und Akkorde, MP3, 20 s und als Schleife 4:00); headless, DE/EN, Beamer 1280×720 bei 150 %.
Die Trennung entfernt den Bass: der Unterschied zum Original korreliert mit 0,94 mit der Bassspur.

- **LH-BIB-09 gemessen** (Song 4:00, Ryzen 9 3900X, `nice 19`, Last 8–22 durch parallele Builds): Grafikkarte
  (RTX 3090) 14 s; Prozessor mit 24 Threads 128 s (bei mehr Last 187 s, 12.10. ohne Last 92 s); auf 4 Kerne
  beschränkt 212 s. Erfüllt auf Empfehlungs-Hardware; mit 4 Kernen über 3 min, läuft dort im Hintergrund.
- S1 (mittel, behoben, Test zuerst): Abbrechen und Beenden ließen halbe Dateien von Demucs im Cache.
- S2 (mittel, behoben, Test zuerst): Ein Fehler bei einem Song stand an jedem Stück.
- S3 (mittel, behoben, Test zuerst): Lief die Trennung eines anderen Songs, tat „Ohne Bass“ nichts und sagte nichts;
  jetzt „Kontra entfernt gerade den Bass aus …“.
- S4 (niedrig, behoben, Test zuerst): Ein neuer Backing-Track ließ Demucs minutenlang am alten weiterrechnen.
- S5 (mittel, behoben): Getrennte Dateien (rund 10 MB je Minute) ließen sich nicht löschen, Ort und Größe waren
  unbekannt. Jetzt im Cache-Ordner (`~/.cache/kontra/stems`, der alte Ort im Datenordner zieht einmal um), „?“ nennt
  Ort, Anzahl und Größe, „Getrennte Datei löschen“ je Song.
- S6 (mittel, behoben): Demucs belegt alle Kerne; es läuft jetzt mit `nice -n 10`, Mitspielen und Oberfläche gehen vor.
- S7 (niedrig, behoben): Fortschritt ohne Restzeit, „0 %“ solange Demucs sein Modell lädt, „100 %“ beim Speichern.
  Jetzt „Demucs startet …“ (Streifen), „31 %, noch etwa 79 s“ (Demucs' eigene Schätzung), „Datei wird gespeichert …“.
- S8 (niedrig, behoben): Fehlermeldung zweizeilig mit Python-Ausnahme und „..“; jetzt kurz mit Rat, Demucs' Worte
  unter „?“.
- S9 (niedrig, behoben): Der Schalter „Bass der Aufnahme entfernen“ zeigte nicht, was spielt; jetzt Auswahl
  „Original | Ohne Bass“ (ohne Demucs „Ohne Bass“ durchgestrichen und gesperrt, der Tooltip sagt warum).
- Erster Lauf: Demucs lädt sein Modell (81 MB) ohne Fortschrittsbalken; „?“ sagt das vorher (LH-NF-08).
- **P1 Lea:** die Zeile erscheint erst mit Backing-Track, ein Klick, jederzeit abbrechbar. **P7 Sam:** Auswahl mit
  Pfeiltasten, gesperrtes Segment durchgestrichen, Fortschritt mit Namen, Fehler als Text. Screenreader am Gerät
  nicht geprüft.
- Offen: Regler „Original-Bass leise mithören“ im Mixer (Demucs liefert `bass.wav` mit; braucht einen zweiten
  gestreckten Strom im Mixer, Rechenzeit auf Mindest-Hardware); Flatpak weiter ausgegraut; Übersicht aller
  getrennten Dateien in den Einstellungen.

### Hardware-Prüfung 13.10.2026 (Eigentümer: E-Bass, Scarlett Solo, PipeWire)

`kontra-cli check`, geführt; Aufnahmen in `testdata/korpus/local/own/check-2026-10-13/` (lokal). Betrifft P1, P2, P3.

- H1 (hoch, behoben): Rundlauf 41–45 ms, über den 30 ms von LH-NF-01. Kontra forderte keinen Puffer an, PipeWire
  blieb bei 1024 Frames; erzwungenes Quantum 256/128/64: 28/20/16 ms, keine Xruns. Live-Streams fordern 128 an.
  Nachtrag 14.10. (nativer PipeWire-Host): 128 Frames kommen als `node.latency` an, das Quantum folgt. Software-Schleife
  (temporärer Null-Knoten `Audio/Duplex`, danach entfernt; nicht am echten Ausgang), Release-Build, Rechner unter Last
  (Load 9–14), je Lauf 12 Töne:

  | Weg | Block | Rundlauf (Median) | Streuung max. | Xruns je Lauf |
  |---|---|---|---|---|
  | ALSA-Plugin, Standard | 512 | 21,5 ms | 0 | 0 |
  | PipeWire, Standard | 1024 (Quantum) | 42,8 ms | 0 | 0 |
  | ALSA-Plugin, 128 ohne Echtzeit | 128 | 5,3–13,4 ms | bis 2,7 ms | 0–17 |
  | PipeWire, 128 ohne Echtzeit | 128 | 5,4–21,4 ms | bis 62 ms, Töne fehlen | 0–16 |
  | beide, 128 mit Echtzeit-Priorität | 128 | 5,4 ms | 0 | 0 (6 von 6 Läufen) |

  Ohne Echtzeit-Priorität verliert der Audio-Thread unter Last Blöcke (cpals PipeWire-Host verarbeitet im Thread seiner
  Hauptschleife). Kontra fordert jetzt im ersten Callback `SCHED_FIFO` an (Priorität 80, unter PipeWires 88); das geht
  nur mit RLIMIT_RTPRIO (z. B. Gruppe `realtime`), sonst bleibt es beim Alten; `kontra-cli check` meldet, ob es
  gewährt wurde. rtkit (D-Bus) für alle anderen bietet cpal nur über audio_thread_priority (MPL-2.0): offen. Die Software-Schleife misst nur
  Puffer und Planung, nicht den Wandler: am Solo gilt weiter die Messung oben (128: 20 ms).
- H2 (hoch, behoben): Bass auf Eingang 2, Eingang 1 leer: der Mittelwert halbierte den Pegel, Übersteuern (0,2 s je
  Pop) blieb unter der Clip-Marke (LH-EIN-03). Der Eingang folgt jetzt dem lautesten Kanal.
- H3 (mittel, behoben, Test zuerst): Kalibrierung ohne Vorzähler; der Spieler wartete vier Klicks (4 von 8), die App
  riet „lauter spielen“. Jetzt vier Klicks Vorzähler, Hinweis nennt ihn.
- H4 (mittel, behoben): Mit Vorzähler spielte der Spieler 75–95 ms früh; der Korpus maß den Spieler (3 von 8). `check`
  schätzt den Versatz je Aufnahme; Bericht markiert Übersteuern.
- H5 (mittel, teilweise behoben 14.10.): Fehlalarme am echten Bass, etwa einer je Ton (Precision 50 %), meist 0,2–0,5 s
  nach dem Anschlag und 20–35 dB leiser (Finger heben: leere Saite klingt kurz an; Saitenwechsel). Der „echte leise Ton
  26 dB unter dem davor“ war keiner: Das Greifen tippt die Saite 30–70 ms vor dem Zupfen an, Kontra meldete diesen
  Vorläufer mit dem Ton des Zupfens (50–70 ms zu früh, 20 dB zu leise). Jetzt beginnt die Note am Zupfen, wenn es
  ≥ 12 dB lauter ist; ein späterer Anschlag mit besserer Tonhöhe im Zeitfenster übernimmt die Note, der frühere wird
  Extra. Aufnahme `fretted` bewertet: 75 → 100 % (normal und locker). Die Trefferquote je Durchlauf (Werkstatt-Tempo, „sauber“)
  zählte Extras mit: Bei etwa einem Fehlalarm je Ton lag ein fehlerfreier Durchlauf bei 50 %, die Werkstatt wurde
  langsamer. Jetzt zählt sie wie die Auswertung nur Noten der Partitur. Offen: Nachklang-Fehlalarme bleiben (Extra);
  eine Pegelregel relativ zur Note davor kostet echte Töne (IDMT: echte leere Saite 21 dB leiser, 0,4 s danach).
  Zahlen: `testdata/korpus/QUELLEN.md`, „Fehlalarme am E-Bass“.
- H6 (hoch, offen): Slap 8 von 8 als Dead Note, Pop 6 von 8 als normaler Ton (ohne Übersteuern). Hammer-on 4/4.
  Messung (`dsp::strike`): die Slaps sind hell (Höhenanteil 1,5–3 %), aber nach 60 ms fast still (Rest 4–15 %; IDMT-Slaps
  ≥ 28 %); die Pops nicht heller als Zupfen (0,3–0,9 %). „Hell und kurz = Slap“ bringt 7 von 8 Slaps, kostet aber IDMT-
  Dead-Notes (27 → 21 von 28, auch sie sind hell); verworfen. Braucht Aufnahmen weiterer Spieler, geübt und ungeübt.
- H7 (mittel, offen): Slide über zwei Bünde als Hammer-on gehört; Runde 11 „legato“ deckt nur einen Bund.
- H8 (mittel, behoben): Ein bestimmtes Interface neben anderem Standardausgang ging nur über `PIPEWIRE_NODE`; die
  Geräteliste zeigte das Solo nur als `hw:`/`sysdefault:`, die PipeWire belegt hält. Kontra spricht jetzt PipeWire
  direkt an (cpal-Host `pipewire`, ALSA bleibt Rückfall ohne PipeWire, `KONTRA_AUDIO_HOST=alsa` erzwingt ihn): Listen
  zeigen die Knoten mit ihrem Namen („Scarlett Solo USB Analoges Stereo“), Eingänge nur Quellen (keine Monitore,
  keine Streams anderer Programme), Interfaces zuerst; die ID ist der Knotenname und bleibt über Neustarts gleich.
  Eine gespeicherte Wahl, die nicht mehr in der Liste steht (ALSA-ID, abgezogenes Gerät), gilt als „Automatisch“
  (Interface zuerst). Entfernter Knoten meldet einen Fehler wie bisher (LH-ALL-04).
  Review #73: Nach „device busy“ (ein ALSA-Programm hielt das Solo, 13.10. abends) legte WirePlumber den Eingang des
  Solo nicht neu an; „Automatisch“ nahm still den Line-in der internen Karte. Jetzt: Sieht ALSA eine USB-Karte mit
  Aufnahme, für die PipeWire keinen Eingang hat (`/proc/asound/cards`, Name im Knoten), listet Kontra sie über ALSA
  (`plughw`, 128 Frames, 0 Xruns am Solo gemessen) und sagt beim Start des Eingangs, wie der Knoten zurückkommt:
  Interface ab- und anstecken oder `systemctl --user restart wireplumber`. `kontra-cli devices` markiert sie.
- Tonhöhe: 100 % der gefundenen Noten. Kalibrierung 43,7 ms (±19,8 ms). Pegelring „grün“ trotz Übersteuern.

### QA-Runde 13.10.2026 (GUI und Bedienung; Demo-Stück, gerenderte Aufnahmen)

Alle Seiten headless aufgenommen (DE/EN, Kante, Kante Light, System) und im Beamer-Format 1280×720 bei 150 %
(`QT_SCALE_FACTOR=1.5 kontra -qwindowgeometry 853x480`). Klicks und Tasten am QML-Code geprüft, keine Geräte.

- **P1 Lea (J-A1, J-C1): bestanden nach Befunden.**
  - G1 (hoch, behoben #48): Kontra öffnete in der Bibliothek; ohne Stück stand dort nur eine leere Liste, und der
    Weg zum nächsten Schritt fehlte. Neu: Startseite (letztes Stück, nächste Übung, die noch nicht sitzt, Woche).
  - G2 (mittel, behoben #48): Neun Seiten ungeordnet, Einrichtung zwischen Einstellungen und Fortschritt.
    Navigation nach Tätigkeit gruppiert, Einrichtung unter Einstellungen und auf der Startseite.
  - G3 (mittel, behoben #49): Übungen begannen mit dem Generator; jetzt der Plan zuerst.
  - G4 (mittel, behoben #49, Test zuerst): Quiz und Gehörbildung zeigten vor der ersten Antwort „0 von 1 gefunden“.
  - G5 (mittel, behoben #49): Fortschritt sagte nach dem ersten Lauf „noch nicht geübt“ und zeigte keinen Balken
    (Minuten vor dem Summieren gerundet); „1 Läufe“, „an 1 Tagen“.
  - G6 (mittel, behoben #49, Test zuerst): Auswertung nannte einen falschen Ton „+100 Cent“; jetzt „C2 gespielt“.
  - Runde 11: „Spielen“ und „Song“ heißen jetzt „Frei spielen“ und „Durchlauf“.
- **P2 Jens (J-B1, J-B3): bestanden nach Befunden.**
  - G7 (niedrig, behoben #49): Setlisten-Feld seitenbreit vor den Stücken; ungespielte Stücke zeigten den Dateinamen.
  - G8 (mittel, behoben #49): Gerätenamen in Eingang/Ausgang auf „Automatisch…“ gekürzt (Layout-Hinweise wirken
    in `KanteSettingRow` nicht).
  - G9 (mittel, behoben mit Kante 1.35): Tabulatur und Noten zeigten den Werkstatt-Abschnitt nicht, nur die Tab-Lane.
    Jetzt in allen Ansichten getönt mit Klammern [ ].
- **P4 Thomas (J-B5, Beamer): bestanden nach Befunden.**
  - G10 (hoch, behoben #49): Tabulatur und Noten zeichneten über die Transportleiste, die Kopfzeile lief rechts aus
    dem Fenster. Notenbereich beschnitten, Ansichten als Liste, Transponieren in der Backing-Zeile.
  - G11 (mittel, behoben #49): Im Stimmgerät lief der Cent-Wert in die Pegelanzeige; Pegel steht jetzt daneben.
- **P7 Sam (J-F1):** Startseite in allen drei Stilen lesbar; Zustände der Karten mit Zeichen und Wort. Tabulatur nach
  einem Lauf unter simulierter Deuteranopie und Protanopie (Machado 2009): Treffer und Fehler über ✓ und ✗ unterscheidbar,
  die Farbe der Ziffer allein nicht (bestanden, die Form trägt).
  - G12 (mittel, behoben #49): Saitenwahl im Stimmgerät nur per Maus; jetzt Tab, Leertaste, Eingabe.
  - G13 (niedrig, behoben #49): Auswertung schloss nicht mit Esc.
  - G14 (niedrig, behoben #49): „Nächste Übung“ trug Kantes Häkchen (grüne Stufe) und las sich wie erledigt.
  - G15 (niedrig, behoben #49): Kreuz und b als „#“ und „b“ (Quiz, Stimmgerät, Übungstitel); jetzt ♯ und ♭.
  - G16 (mittel, behoben #49, P1): Leere Bibliothek und leerer Fortschritt endeten ohne Weg; jetzt „Mit einer Übung
    anfangen“.
  - G17 (mittel, behoben #49, P4): Auswertung im Beamer-Format: Bildlaufleiste über „Schließen“, Histogramm schob die
    schwierigsten Takte aus dem Bild.
  - G19 (mittel, behoben #49, P2/P3): Notenansicht zeigte immer eine Zeile, der Rest blieb leer; jetzt so viele
    Zeilen, wie passen (wie die Tabulatur).
  - G18 (niedrig, behoben #49, P4): Im Beamer-Format war der Notenbereich niedriger als eine Notenzeile; sie wurde
    abgeschnitten, und ein Balken lag über der Transportleiste (Qt 6.12: `clip` schneidet `Shape`-Kinder nicht ab,
    nachgewiesen mit `grabToImage`). Jetzt wird der Linienabstand kleiner, bis eine Zeile passt.
  - G20 (niedrig, behoben #49, P4): „Noten + Tab“ im Beamer-Format: Notenzeile ragte in die Tabulatur; Noten haben
    jetzt die halbe Höhe und so viele Takte je Zeile wie die Tabulatur darunter.
  - G21 (mittel, behoben #49, P2/P3): Tabulatur gab jedem Takt dieselbe Mindestbreite; Sechzehntel (Ghost-Groove #56)
    liefen ineinander. Jetzt richtet sich die Breite nach dem dichtesten Takt (Achtel: weiter fünf Takte je Zeile).
  - G22 (hoch, behoben #49, Test zuerst, P1): Eine Übung, aus dem Dateimanager oder per Pfad geöffnet, blieb in der
    Übungsliste „neu“ und auf der Startseite „nächste Übung“: Der Plan nannte sie über „../..“ (auch installiert:
    `bin/../share`), der Fortschritt hängt am Pfad. Jetzt ohne „..“; die Bibliothek speichert und sucht ohne „..“, alte
    Einträge wandern beim Laden.
  - G23 (mittel, behoben #49, P1): Die Übungsliste zeigte nicht, was sitzt; jetzt Häkchen ab 90 % und „3 von 8 sitzen“
    je Stufe.
  - G24 (mittel, behoben #49, Test zuerst, P1): Nach einer Übung, die jetzt sitzt, endete die Auswertung ohne Weg nach
    vorn; jetzt „Sitzt“ mit der nächsten Übung im Plan, ein Klick.
  - G25 (niedrig, behoben #49): Der Mixer der Übungsseite zeigte die Lautstärke fürs Vorspielen der Gehörbildung, die
    dort nie klingt; nur noch in Einstellungen und auf der Gehör-Seite.
  - G26 (niedrig, behoben #49, P1): Startseite zeigte dieselbe Übung zweimal mit zwei Knöpfen, wenn die nächste Übung
    auch das zuletzt gespielte Stück war; jetzt ein Knopf und ein Satz dazu.
  - G27 (niedrig, behoben #49, P7): „Über Kontra“ listete rund 70 Bausteine ausgeklappt; die Tastenkürzel darunter
    lagen weit außer Sicht. Jetzt Kontra, Qt und KDE Frameworks, der Rest auf Knopfdruck.
- **P2 Jens (J-B3, Setliste):** G28 (niedrig, behoben #49, Test zuerst): Mitgelieferte Übungen standen in Setlisten und
  bei „Nächstes Stück“ mit Dateinamen („09 Groove Achtel …“); jetzt mit dem Titel aus dem Plan in der Oberflächensprache.
  Zweiter Leser über #48/#49: sieben Befunde, alle behoben (u. a. gewählte Spur bleibt, alte „..“-Pfade wandern).
- **P5 Aylin (J-E1):** G29 (hoch, behoben #49): In der Klaviatur-Ansicht schrumpften Spur und Tasten auf einen Streifen;
  die Zeile mit Griffbrett und Pegel nahm die Höhe (eine verschachtelte Qt-Layout füllt von sich aus). In allen
  Ansichten bekommen die Noten jetzt die freie Höhe.
- **Halbes Fenster (≈ 500 px, neben Video oder Tab):** G30 (mittel, behoben #49): Kopfzeile, Pegel und Transportleiste
  liefen rechts hinaus. Unter 700 px: Kopf einspaltig, Pegel über die ganze Breite, ohne Griffbrett und
  Präsentationsknopf (F5 bleibt); in der Bibliothek rückt das Ergebnis unter den Titel.
- **Sprache (J-F2):** G31 (mittel, behoben #49, Test zuerst): Fortschritt, Bericht und Übungsseite nannten mitgelieferte Übungen
  auf Deutsch, auch in der englischen Oberfläche (der gespeicherte Titel ist der deutsche Schlüssel).
- G32 (niedrig, behoben #49, Test zuerst): Die Spurwahl bot bei Übungen und anderen Dateien mit Schlagzeug die
  Schlagzeugspur an (MIDI-Kanal 10, MusicXML-Percussion); jetzt nur Spuren, die ein Bass spielen kann.
  - Offen (Kante #61): `KanteKpi` hat keinen Namen für Screenreader (Zahl und Beschriftung getrennt).
- Offen: Ghost Note doppelt geklammert (F8, Kante, Runde 11).

### QA-Runde 12.10.2026 (Nacht: Techniken, MIDI, Stems, Transponieren; synthetisch und IDMT)

Gespielt headless mit erzeugten Aufnahmen (`kontra-cli render --techniques`) und den echten IDMT-Aufnahmen; ohne
echten Bass, ohne zwei MIDI-Keyboards, ohne installiertes Demucs.

- **P3 Mira, P4 Thomas (J-C5 ⚙, neu): bestanden nach Befunden.**
  - F1 (hoch, behoben #35): Eine richtig gespielte Ghost Note galt als falscher Ton (−3300 Cent, keine Tonhöhe).
  - F2 (mittel, behoben #35): Hammer-on- und Pull-off-Töne zählten als zusätzliche Noten.
  - F3 (mittel, behoben #35): Slide-in/-out wurde als Slide importiert, MusicXML markierte beide Noten.
  - F4 (hoch, behoben #41): An echten Aufnahmen erkannte Kontra 0 % der Slaps, 2 % der Pops, 14 % der Dead Notes;
    jetzt 79/70/96 %.
  - F5 (hoch, behoben #42): Flageoletts wurden an der gegriffenen statt der klingenden Tonhöhe bewertet.
  - F6 (mittel, behoben #42): Eine gedämpft gespielte Dead Note (x) galt als falscher Ton.
  - F7 (niedrig, Runde 10): „6 von 6“ bei 8 Noten (Hammer-on-Töne nur in der Technik-Auswertung).
  - F8 (niedrig): Ghost Note in Tab-Lane und Tabulatur doppelt geklammert (Bund „(0)“ und Zeichen „( )“).
  - F9 (Runde 10): x- und ( )-Noten meinen musikalisch etwas anderes, als Kontra bewertet.
  - Offen: Slides an echten Aufnahmen 0 von 20 (Onsets beim Gleiten; ein Versuch mit strengerem Anstieg brachte
    +0,6 Punkte Präzision, verworfen). 13.10.: 3 von 20 (Slides über zwei Bünde); Ein-Bund-Slides klingen wie
    Hammer-ons (siehe LH-BEW-06, Slides an echten Aufnahmen).
- **P1 Lea (Warten-Modus): F10 (mittel, behoben #38):** Ein Hammer-on vor der erwarteten Note zählte als falscher Ton.
- **P2 Jens (J-D1 ⚙):** Transponieren mitten im Lied hält das Lied nicht mehr an (#39). Stem-Trennung ohne Demucs
  ausgegraut mit Anleitung (Screenshot); F11 (mittel, behoben #37): Beim Öffnen per Start fehlte der Stem-Zustand;
  F12 (niedrig, behoben #37): Eine laufende Trennung erschien an jedem Stück; F13 (mittel, behoben #37): Fehlermeldung
  des Programms konnte fehlen (Rennen beim Lesen). Mit echtem Demucs nicht geprüft (Runde 10).
- **P5 Aylin (J-E1 ⚙):** bestanden. F14 (mittel, behoben #40): Die Klaviatur beschriftete die nächste Taste mit
  „Nächstes Stück der Setliste“ (doppelter Text-Schlüssel). MIDI-Auswahl nur per Unit-Test.
- **P7 Sam:** Technik-Ergebnisse unterscheiden sich durch Form (Rahmen, Strich, gestrichelt); Namen für Screenreader
  aus i18n.
- **Zweiter Leser** (PR-Agent fällt aus; Review-Agent über #36–#42): 12 Befunde, 11 behoben in #43, darunter Dead
  Notes mit hörbarer Tonhöhe (Warten-Modus hing), eine fertige Trennung beendete den Lauf eines anderen Stücks,
  Abbrechen traf im Flatpak nur den Wrapper, Marken verschwanden beim Transponieren in der Pause. Der zwölfte (Neu-Layout
  bei anderer Stimmung legt ein Flageolett als gegriffene Note) zeigt kein falsches Zeichen (Kante hat keins für
  Flageoletts); die Note wird weiter oktavtolerant bewertet.

### QA-Runde 9.10.2026 (Personas, headless, erzeugte und echte Aufnahmen)

Drei Durchläufe: P1 Lea + P7 Sam, P2 Jens + P4 Thomas, P3 Mira + P5 Aylin + P6 Karl. Berichte mit Belegen lagen im
Scratchpad der Sitzung; die Befunde stehen hier vollständig. Klicks und Tasten ließen sich headless nicht bedienen
(am QML-Code geprüft), kein echter Screenreader, keine Geräte. Ergebnis: keine Journey voll bestanden außer J-C2.

| Journey | Status | Hauptgrund |
|---|---|---|
| J-A1 Anstecken und los | teilweise | Assistent-Schritte nur am Code; Pegeltexte (behoben) |
| J-A2 Stimmen | teilweise | Laptop-Buchse zeigte falsche Richtung (behoben); kein geführter Modus (offen) |
| J-A3 Latenz | teilweise | 40 ms gemessen, danach pünktlich; PipeWire-Quantum-Hinweis fehlt (offen) |
| J-A4 MIDI/Mikrofon | nicht bestanden | kein Hinweis „kommt später“ (offen) |
| J-B1 Lied, Setlist | teilweise | Ordner-Import, Setlisten, Mixer fehlen (offen); Spielen zeigt Punkte (Runde 6) |
| J-B2 Warten | teilweise | gespielter Ton fehlte, Ansage fehlte (behoben) |
| J-B3 Werkstatt | teilweise | früher erster Ton blockierte Steigerung (behoben); Bedienung (umgesetzt) |
| J-B4 Bundlos, tief | teilweise | B0 +8 ct (behoben), Notensatz (behoben), Cent-Werte und „streng“ in der App fehlen (offen) |
| J-B5 Beamer | nicht erfüllt | Überlappung bei 1280×720/150 %, Auswertung durchscheinend und zu groß (offen) |
| J-C1 Erste Woche | nicht erfüllt | keine Fortschrittsansicht; „0 min“ nach 22 s (offen) |
| J-C2 Timing-Tendenz | bestanden | Histogramm ohne Achsentitel (offen) |
| J-C3 MusicXML | teilweise | Notensatz (behoben); Groove und Generator Phase 2 |
| J-D1 Backing | nicht prüfbar | Beat-Erkennung Phase 3 |

Behoben (Branch `runde-5`): `--style` startete die UI nicht (jetzt `--theme`); hängender Prozess bei QML-Fehler;
Stimmgerät an der Laptop-Buchse (50-Hz-Brumm, Notch-Filter); B0 +8 ct im Tracker (`korpus check` prüft jetzt auch
den größten Cent-Fehler); früher erster Ton im Loop; Warten-Modus (gespielter Ton, Saitenname, H statt B, Ansage,
Info statt Warnband); Profil 5 Saiten wurde überschrieben; Tonart, Pausen, Haltebögen, Oktavlage in der
Notenansicht; fehlender Backing-Track stoppte das Lied; Importfehler englisch; MIDI-Titel „Tempo“; Pegel-Assistent;
Tastatur global; Namen für Screenreader; feste englische Texte.

Offen (Phase 1): Setlisten und Ordner-Import (LH-BIB-04), Mixer (LH-WIE-03, LH-MOD-05), geführtes Stimmen
(LH-TUN-01/03), Fortschrittsansicht (LH-FOR-01), Hinweis MIDI/Mikrofon (LH-EIN-10), PipeWire-Quantum (LH-EIN-07),
Zeitfenster und Cent-Grenze einstellbar (LH-BEW-02/05), Cent-Werte für Bundlos, Ansicht pro Lied (LH-ANZ-04),
Layout bei 1280×720/150 % und Auswertungskarte, Tabulatur nur eine Zeile, Spurwechsel nach Autowahl,
Desktop-Datei `%f`/MimeType, Hinweis bei automatischem Fünfsaiter, Histogramm-Achsen, Werkstatt mit Backing bei
60 % (Ursache unklar), Brumm in der Notenerkennung über die Laptop-Buchse, Journey-Tests in `tests/journeys/` und
fehlendes Testmaterial. Kante-Wünsche: Schalter und Auswahllisten im Kante-Stil (dunkel), Hauptaktion in Kante
Light, Akkordsymbole, Tonart je Takt, Schreibweise aus der Datei, Assistent-Schritt nicht nur per Farbe.

### QA-Runde 9.10.2026 (Entwicklung, synthetische Eingaben)

Durchgespielt mit gerenderten Aufnahmen (`kontra-cli render`) und Screenshots aller Seiten (DE/EN):

- J-B1 (P2) Lied durchspielen: Fehler gefunden und behoben – Aufnahmen wurden zum Zeitpunkt des Dekodierens statt zum ersten Sample ausgerichtet (alle Noten verschoben); Noten kurz vor dem ersten Schlag wurden auf 0 ms gesetzt statt als „zu früh“ gewertet.
- J-B2 (P1) Warten-Modus: hält vor jeder Note, Text „Spiel E1 – Saite 1, Bund 0“; ohne Eingabegerät Fehlermeldung (Container).
- J-A2 (P1) Stimmen: E1 auf 0 ct, „gestimmt“ in Worten und Symbol.
- J-F2 Sprache: Kante-Standardtexte waren deutsch in der englischen App (Transport, Pegel) – übersetzt.
- Reibung: Pegeltexte überlappten im Kante-Pegel; jetzt kurze Zonen-Texte plus Hinweis darunter.
- Fehler: Einstellungen und Bibliothek hell auf hell (Kirigami-Hintergrund) – behoben.
- Fehler: Notenansicht-Seite hing (QML-Eigenschaft `data` überdeckte Kind-Liste) – behoben.
- J-A3 (P3) Latenz kalibrieren: mit Aufnahme 40 ms gemessen, übernommen. Fehler gefunden: eine fehlerhafte Demo-MIDI (falsche Meta-Länge) – Testdatei korrigiert, Importer war richtig.
- J-A1 (P1) Erster Start: Assistent erscheint, alle Schritte überspringbar, endet in der ersten Übung. Fehler: Hinweistext dunkel auf dunkel – behoben.
- J-F1 (P7) 200 % Skalierung: Üben und Einstellungen ohne Abschneiden. Kosmetisch: in Kantes Einstellungszeile ist der Hinweis größer gesetzt als der Titel (an Kante melden).
- Nicht getestet: echte Geräte, Tastaturdurchlauf, Screenreader, Farbfilter.

## Lastenheft-Umsetzung (10.10.2026)

- [x] LH-LRN-04: Übungs-Generator (Tonleitern, Arpeggien, Walking Bass, Grooves) in jeder Tonart und jedem Tempo, Seite „Übungen“ (Strg+7).
- [x] LH-BIB-08: 34 mitgelieferte Übungen in vier Stufen (`testdata/exercises`, von `kontra-cli exercises` erzeugt, CI prüft den Stand).
- [x] LH-BIB-08, LH-LRN-04, Ausbau (14.10.): 144 Übungen in acht Stufen (neu: 6 Taktarten, 7 Lesen, 8 Technik; 99 neue, an die Stufen angehängt, alte Dateinamen unverändert), je Übung ein Ziel. Neue Familien: Fingerübungen (Bünde fest in der Datei, sonst legt die Tab sie auf leere Saiten), Modi, harmonisch Moll, Terzen und Vierergruppen, Dreiklänge, I-IV-V, I-V-VI-IV, I-VI-II-V, Moll-II-V-I, Grundton-Quinte(-Oktave), Dead Notes, Offbeat, Tresillo, Triolen, Funk, Slap, 3/4, 6/8, 5/4, 7/8, Tempo-Treppen. Fehler behoben (Test zuerst): Blues und Moll-Kadenz endeten auf der Dominante (jetzt Schlusstakt auf der Tonika), Walking-Annäherung an E1 von unten fiel auf den Grundton, Oktav-Groove auf hohen Grundtönen (A♭2) verlor die Oktave. Review #62 (Test zuerst): Der Fortschritt hängt an der Song-ID (Noten), nicht am Pfad; ändert sich die Datei am selben Pfad (neu erzeugte Übung), wandern die Läufe zur neuen ID (`ProgressStore::adopt`), sonst galt eine sitzende Übung als neu. `pos`-Ereignisse fremder Dateien gelten nur, wenn Saite und Bund den Ton ergeben; Dreiklänge heißen als Dreiklänge (G, Am, Bdim); Terzen enden mit einem Grundton. Generator auf der Übungsseite mit allen Familien.
- [x] LH-LRN-02: Griffbrett-Quiz mit dem echten Bass (Töne auf einer Saite finden, Intervalle), Seite „Griffbrett“ (Strg+8); nach dem richtigen Ton zeigt das Griffbrett alle Lagen.
- [x] LH-BIB-06: Backing-Track abgleichen: Kontra erkennt die Schläge der Aufnahme (Spektralfluss, Tempo per Autokorrelation, Beat-Tracking nach Ellis) und legt die Takte darauf (Tempo-Karte je Takt); Takt 1 um einen Schlag, alle Takte oder ein einzelner Takt in 10-ms-Schritten korrigierbar. Abnahme (24.10., synthetisch): 4 min Band-Mix, Tempo 116–124 BPM, ±5 ms locker: schlimmster Schlag 7,4 ms, Mittel −1,2 ms (`stays_within_20_ms_over_four_minutes`). Dabei behoben: alle Schläge lagen 12,8 ms zu früh (Versatz des Spektralflusses), der Spieler wurde mit der Band als zu spät gewertet. Offen: Gegenprobe mit einer echten Aufnahme.
- [x] LH-BIB-09 (Kann, Runde 9): Bass aus dem Backing-Track entfernen über Demucs, wenn installiert (`drivers::separator`: `KONTRA_DEMUCS`, `PATH`, `~/.local/bin`; im Flatpak `flatpak-spawn --host`, braucht eine Erlaubnis, die das Manifest noch nicht hat). `demucs --two-stems bass -n htdemucs`, Fortschritt aus dem Fortschrittsbalken auf stderr, Abbrechen beendet den Prozess. Ergebnis im Cache je Datei (Pfad, Größe, Änderungszeit); der Abgleich bleibt. Übungsseite „Ohne Bass“: ohne Demucs ausgegraut mit Erklärung, Anleitung („?“) und „Erneut prüfen“. Getestet mit einem Ersatz-Programm; mit echtem Demucs 4.1.0 gemessen (12.10.): Song von 4:11 in 17 s (RTX 3090) bzw. 92 s nur auf dem Prozessor (24 Threads); `pipx install demucs` bringt numpy nicht mit (Anleitung ergänzt). Messung auf Mindest-Hardware (Laptop) offen. 14.10.: Auswahl „Original | Ohne Bass“, Restzeit, Abbrechen ohne Reste, Cache-Ordner mit Größe und Löschen je Song, `nice -n 10`; 4:00 in 128 s (Prozessor, 24 Threads, unter Last), 212 s auf 4 Kernen (QA-Runde 14.10.).
- [x] LH-LRN-01: Grooves auf dem Grundton (Viertel bis Sechzehntel, Pausen, Shuffle, Oktaven) mit Schlagzeug (#15); seit dem Ausbau (14.10.) auch mit Ghost Notes, in Stufe 4 Groove.
- [x] LH-BIB-12 (24.10.): Stück-Generator, Art „Stück“ mit den Stilen Ballade, Pop, Rock, Funk, Shuffle: Form aus
      Intro, Strophe, Refrain, Bridge, Outro mit eigenen Akkordfolgen und Grooves, Fill vor jedem Teil, Schluss auf der
      Tonika (`exercise/song.rs`). Sechs mitgeliefert als Stufe „9 Stücke“ (145–150). Die Art-Auswahl des Generators ist
      jetzt eine Liste (lief bei 700 px aus dem Fenster). Offen: Teilnamen (Strophe, Refrain) über den Takten brauchen
      Probenzeichen in Kante; Basslinien mit Annäherungstönen vor Akkordwechseln;
      Bestand: die Blues-Tonleiter im Generator bekommt nur einen Durchgang (`variant == "blues"`, Review #100).
- [x] LH-TUN-02 / LH-EIN-01: Stimmung im Profil (Standard, Drop D bzw. Drop A bei 5/6 Saiten, ½ und 1 Ton tiefer, eigene mit Notenwahl je Saite) in Einstellungen und Assistent; Stimmgerät (geführt) und Tab-Zuordnung von Dateien ohne Stimmung (MIDI, MusicXML) nutzen sie, ein Wechsel ordnet das geladene Stück sofort neu zu.
- [x] LH-TUN-04: Kammerton A 430–450 Hz (Standard 440) für Stimmgerät, Erkennung und Bewertung.
- [x] LH-BEW-02: Zeitfenster „Eigene“ mit ±20–150 ms.
- [x] LH-ANZ-06: Saitenfarben Kante / Einfarbig / Kräftig (nur Kante-Farben) mit Vorschau; seit Kante 1.28 aus `KanteStyle.stringPalette` für Tab-Lane, Tabulatur und Griffbrett (Runde 8).
- [x] LH-ANZ-08: Linkshänder-Griffbrett (Sattel rechts).
- [x] LH-EIN-02: ohne gewählten Eingang nimmt Kontra ein USB-Interface vor der internen Karte; „Automatisch: <Gerät>“ zeigt, welches.
- [x] LH-EIN-10: MIDI und Mikrofon stehen als deaktivierte Einträge „Kommt später“ in der Anschluss-Auswahl.
- [x] LH-ALL-05: Info-Knöpfe für Hi-Z, Latenz und das Werkstatt-Zielband; Zeitfenster, Kammerton, Stimmung und Quantum-Hinweis erklären sich im Text. „Lage“ kommt in der Oberfläche nicht vor.
- [x] LH-NF-13: Journey-Tests in `tests/journeys/` (J-A2, J-B1, J-B2, J-B3, J-C2, Neustart) als Test `journeys` von `kontra-appcore`, laufen im CI-Job `core`; Aufnahmen, MIDI-Dateien und `stimmen-e-30c.wav` erzeugt der Test.
- [x] LH-FOR-03: Einstellungen → „Deine Daten“: alles als eine JSON-Datei sichern (`kontra-data`, Version 1) und laden; beim Laden „Ergänzen“ (eigene Einstellungen bleiben, neue Stücke, Setlisten, Durchläufe dazu) oder „Ersetzen“; ungültige Dateien ändern nichts; Bericht als Meldung.
- [x] LH-FOR-04: Fortschritt → „Bericht …“ je Stück: eine HTML-Datei mit Kante-Stil eingebettet (offline lesbar), Kennzahlen, Verlauf Trefferquote/Tempo, Tabelle aller Durchläufe mit schwierigsten Takten (neu im Durchlauf gespeichert).
- [x] LH-BIB-07: Paketformat `.kontra` (Zip mit `manifest.json`, Original-Notendatei, Fortschritt, Begleitspur nur auf Wunsch eingebettet, sonst als Pfad), beschrieben in `FORMAT.md`; Bibliothek → „Teilen …“ je Stück; Öffnen über die Bibliothek, `kontra datei.kontra` und Dateimanager (MIME `application/x-kontra-package`); eigener Zip-Schreiber in `content`, keine neue Abhängigkeit.
- [x] LH-ALL-04: Gerät weg → Lauf endet mit Meldung, Kontra bietet den nächsten Eingang an (Standard, sonst der erste andere; Hinweis mit Knopf „Diesen Eingang nehmen“, nur vorübergehend, nicht gespeichert); alle 2 s wird die Geräteliste geprüft, ist das eigene Gerät zurück, nimmt Kontra es wieder. Auswahl mit Geräteliste-Attrappe getestet. Mit echtem Interface nicht geprüft.
- [x] LH-NF-04: Xrun-Zähler: Treiber zählt Xrun-Meldungen des Audio-Backends und verspätete Callbacks (> 2 Blocklängen + 2 ms) für Ein- und Ausgang; Xruns gelten nicht mehr als „Gerät weg“. Summe seit dem Start in den Einstellungen (Debug-Zeile). Dauertest `kontra-cli soak <stück> --minutes 60 [--speed 0.8] [--output null|<id>] [--max-xruns 0]`: Schleife mit gestreckter Begleitspur, Metronom und Referenztönen auf einem zeitgesteuerten Null-Ausgang oder Gerät. Der 60-Minuten-Lauf auf Mindest-Hardware steht aus; nicht in CI (Dauer).
- [x] LH-NF-10: `scripts/check-design.sh` (in `make check` und CI) bricht ab bei Hex-Farben, px-Angaben, Zahlen für Größen (außer 0 und 1 für Haarlinien) und Schriftfamilien in `crates/app/qml/*.qml`; die Kante-Kopie wird nicht geprüft. Stand: sauber.
- [x] LH-NF-11: Einstellungen → „Über Kontra“: Kontra (MIT), Qt und KDE Frameworks (LGPL, dynamisch) und alle 64 eingebundenen Crates mit Version und Lizenz aus `crates/appcore/licences.json`; erzeugt von `scripts/licences.py` (`make licences`), CI prüft mit `--check`, dass die Liste zum `Cargo.lock` passt.
- [x] LH-EIN-08: Klang fürs Mithören: Tiefen (Shelf 100 Hz), Mitten (Glocke 800 Hz), Höhen (Shelf 4 kHz) je ±12 dB, ein Kompressor (0–100 %, automatische Aufholverstärkung) und „Wärme“ (weiche tanh-Sättigung bei doppelter Abtastrate, Aliasing unter −50 dB statt −34 dB ohne) im Mixer, solange Mithören an ist; „Neutral“ setzt zurück. Änderungen gleiten in 32-Sample-Schritten (kein Knacken), die Erkennung hört das trockene Signal. Keine Vorausschau-Puffer; die Filter gegen Aliasing verzögern um etwa 0,1 ms. Rechenzeit 144 ns je Sample (0,7 % eines 256er-Blocks). Die Bandfrequenzen kommen aus dem Instrument-Profil (`Instrument::tone_bands`), ein weiteres Instrument bringt eigene mit.
- [x] LH-LRN-03: Gehörbildung, Seite „Gehör“ (Strg+9): Ton oder Linie nachspielen (die Linie wächst nach drei fehlerfreien Antworten bis vier Töne und schrumpft nach einer mit Fehler), Intervall (Grundton genannt, zweiten Ton spielen), Grundton eines Dur- oder Moll-Dreiklangs in zufälliger Umkehrung (jede Oktave zählt). Jeder gespielte Ton bekommt Haken oder Kreuz (Kante-Notenzeichen), falsche Töne „zu hoch“ oder „zu tief“; Töne, die während der Vorgabe beginnen, zählen nicht (Anschlagzeit gegen das Ende der Vorgabe, J-C4 ⚙). Nur Tonhöhen zählen, keine Saiten: offen für weitere Instrumente.
- [x] LH-WIE-02: Transponieren je Stück ±6 Halbtöne (Übungsseite): Backing-Track über den eigenen Pitch-Shifter `dsp::PitchShifter` (Runde 8; Phasen-Vocoder mit Peak-Verschiebung, 4096er-Fenster, läuft im Mixer ohne Allokation, Backing wird um die Latenz von 85 ms vorausgelesen, Tonhöhe auf 5 Cent, Einsätze ±5 ms; 82 ns je Frame, Wechsel der Halbtöne ohne Lücke; ersetzt das Vorab-Umrechnen aus #21), Noten und Tonart mit; Bünde aus der Datei bleiben, wo sie auf derselben Saite passen, sonst neue Zuordnung; Töne außerhalb des Instruments rücken eine Oktave. Fortschritt bleibt beim Stück. Wechsel mitten im Spiel (12.10.): der Lauf geht weiter, Bewertetes bleibt, die weiteren Noten zählen in der neuen Tonart (`Session::retarget`, `Practice::retarget`); nur wenn sich mehr als die Tonhöhe ändert, endet er. Mit echter Aufnahme nicht gehört.
- [x] LH-NF-13: Journey-Tests für alle ⚙-Journeys der Phasen 1–3: neu J-A1 (Laptop-Buchse, zu leise, erster Ton < 60 s), J-A3 (Latenz 40 ms gemessen, danach pünktlich ±5 ms), J-B4 (Fünfsaiter bundlos bis B0, Cent je Note auf 3 ct, ab 30 ct „unsauber“), J-C4, J-D1. Manuell bleiben: Pegel-Hinweis als Text (QML), PipeWire-Quantum, hörbarer Versatz über 4 min, Bassschlüssel. `Mistakes::detune_cycle_cents` für Cent-Muster je Note.
- [x] LH-NF-14 / LH-EIN-09 (Saiteninstrumente): Dummy-Gitarre (6 Saiten E2–E4, 22 Bünde) läuft ohne Änderung durch Tab-Zuordnung, Erkennung und Bewertung (`crates/session/tests/profiles.rs`). Offen: Tasteninstrumente passen nicht ins Modell (`Instrument` verlangt 4–6 Saiten, Art heißt `BassKind`); Frage in Runde 8.
- [x] LH-NF-14 / LH-EIN-09 (Tasteninstrumente, Runde 8): `BassKind` heißt `InstrumentKind`; ein Instrument hat Saiten (1–12, vorher 4–6) oder einen Tastenumfang (`Instrument::keyboard`, `KeyRange`, `has_strings`). Dummy-Keyboard läuft ohne Änderung an Session und Bewertung durch Tab (keine), Transponieren und Bewertung. Gespeicherte Profile bleiben gültig (`keys` mit serde-Default). Klaviatur-Ansicht da (LH-EIN-12). Offen: Profilwahl „Keyboard“ in der Oberfläche (Tasteninstrumente nach 1.0); bis dahin nutzt Bass über MIDI das Bass-Profil.
- [x] LH-ANZ-09 (Soll, Phase 2): Präsentationsmodus auf der Übungsseite (F5 oder Knopf, Esc beendet): Vollbild ohne Seitenleiste, nur Titel, Ansicht und Transport; Notenlinien 1,5-fach (Kante-Eigenschaften `lineSpacing`/`staffSpace`), die Lane wächst mit der Fläche. QA-Schalter `--present`. Tastenliste nennt jetzt Strg+1–9 und F5. Bei 1280×720 und 150 % Schrift am echten Bildschirm nicht geprüft.
- [x] LH-EIN-12 (Phase 4, vorgezogen): MIDI-Eingang. Treiber `drivers::midi` (midir 0.11 über ALSA-Seq, dieselbe alsa-Bindung wie cpal; MIDI-Dateien mit midly), Engine-Eingänge `Midi` und `MidiFile` (Note-on wird erkannte Note: exakte Tonhöhe, Lautstärke aus der Anschlagstärke, Zeit aus den MIDI-Zeitstempeln auf der Engine-Uhr). Anschluss „MIDI-Instrument“ wählbar, nennt das erkannte Keyboard; Instrument „Bass über MIDI“ (`BassKind::Keys`). Eine `.mid`-Datei als `--input-file` spielt wie ein Keyboard (QA). J-E1 ⚙: 10 ms späte Noten werden auf 2 ms genau so bewertet. Am echten Gerät über ALSA „Midi Through“ geprüft (Zeitstempel in µs). Klaviatur-Ansicht (Kante 1.30, `KanteKeyLane` über `KanteKeyboard`, Alt+5) über den Umfang des Instruments. Port-Auswahl (Runde 9): unter Anschluss „MIDI-Gerät“, sobald mehr als ein Gerät angeschlossen ist; gespeichert wird der Name (ALSA-Port-Nummern wechseln beim Umstecken). Fehlt das gewählte Gerät, nimmt Kontra das erste und sagt es dort.
- [x] LH-EIN-11 (Phase 4, Korpus erfüllt): Mikrofon als Anschluss wählbar (Hinweis: Zeitfenster ±90 ms, Tonhöhe ±50 Cent), Rauschsperre −48 dBFS. Korpus „room-double“ (Kontrabass pizzicato im Wohnzimmer: frühe Reflexionen, Schroeder-Hall 0,45 s, Mikrofon-Tiefenschnitt 50 Hz, Raumrauschen): Treffer 100 %, Tonhöhe 98 %, in der CI geprüft. Mit Band aus den Lautsprechern („room-double-backing“) ohne Abzug Tonhöhe 28 %; mit `dsp::EchoCanceller` (PBFDAF, 16 × 512, Schritt je Bin gegen Doppelsprechen, Schrittweite 0,2) Treffer 100 %, Präzision 98,7 %, Tonhöhe 92,9 % (mit Schrittweite 1,0 nur 85 %: der Filter folgt dann dem Bass); in der CI geprüft. Live: der Mixer schreibt, was er spielt (ohne Mithören), in einen Referenz-Ring; ein Mikrofon-Eingang übernimmt ihn, richtet ihn über die Startzeiten von Aus- und Eingang aus und zieht ihn vor der Erkennung ab (nur bei gleicher Abtastrate). Offen: Prüfung mit echtem Mikrofon und Lautsprechern (J-E2), Oktavfehler am Kontrabass bei echten Aufnahmen (FiloBass).
- [x] LH-BEW-06 (Soll, Phase 4): Techniken erkennen. Ghost Note (keine Tonhöhe oder nach 60 ms unter 10 %), Slap/Pop (Helligkeit der ersten 8 ms > 0,12; Pop ab D2, nur am Kabel: am Mikrofon macht Raumrauschen leise Anschläge hell), Hammer-on/Pull-off/Slide über Tonhöhenverfolgung im klingenden Ton (≥ 80 Cent, 80 ms stabil, Slide ab 6 Zwischenwerten, gemeldet erst, wenn 60 ms kein Anschlag folgt). Korpus „techniques“ (eigene Artikulations-Synthese): 91,7 % richtig erkannt (Ziel ≥ 85 %), alle anderen Sätze 100 % ohne erfundene Technik. Echte IDMT-Aufnahmen: Recall 82,5 → 87,9 % (gebundene Töne), Präzision 78,9 → 78,2 %; CI-Untergrenze Recall auf 86 % angehoben.
- [x] Flageoletts (12.10., LH-BIB-01, LH-BEW-01): Guitar Pro speichert Saite und berührten Bund; Kontra erwartete die gegriffene Tonhöhe (Flageolett am 5. Bund der E-Saite: A1 statt E3), ein richtig gespieltes Flageolett galt als falscher Ton. Natürliche Flageoletts klingen jetzt am Teilton (GP4/5 binär, GPIF mit `HarmonicFret`; 12., 7./19., 5./24., 4./9./16., 3,2., 2,7., 2,4. Bund); die Tab behält den Bund. Der Tracker hört Flageoletts oft eine oder zwei Oktaven zu tief (IDMT: 43 statt 67): ihr Ton zählt in jeder Oktave (Session und Korpus). IDMT mit Flageolett-Labels: Recall 87,9 → 89,6 %, Präzision 78,2 → 79,7 %; CI-Untergrenzen auf 88/78 %. Künstliche und getappte Flageoletts (13.10.) klingen über der gegriffenen Note: GP4 (AH5/7/12), GP5 (Tonklasse und Oktave, Tapping-Abstand), GPIF (`Artificial`/`Tap` mit `HarmonicFret` als Abstand). MusicXML (13.10.): `<harmonic>` (natural/artificial, `base-pitch`/`touching-pitch`/`sounding-pitch`) und Rautenkopf; ein Akkord aus Grund-, Berühr- und Klangnote wird eine Note am Klangton, Tab mit Bund der Grundnote; natürliche Flageoletts mit Saite und Bund klingen am Teilton. Bleibt gegriffen (oktavtolerant): Pinch, Semi, Feedback, GP4-Tapping, GP5-Werte ohne passenden Teilton, MusicXML-Flageoletts ohne Tab, künstliche nur mit Grundnote. Die Oktave der GP5-Daten ist gedeutet (Teilton mit dieser Tonklasse ab Oktave × 12 über der gegriffenen Note; alphaTab rechnet anders), ohne Einfluss auf die Bewertung. Dead Notes (x, `Muted`) treffen wie Ghost Notes ohne Tonhöhe, wenn sie gedämpft gespielt werden (vorher „falscher Ton“).
- [x] LH-BEW-06, echte Aufnahmen (12.10.): Die IDMT-Labels (Slap-Daumen, Slap-Pop, Dead Notes, Slide) gehen jetzt mit in den Korpus; `kontra-cli korpus techniques` zeigt je Technik, was erkannt wurde (CI: Bericht). Erster Stand dort: Slap 0 %, Pop 2 %, Ghost 14 %; die Helligkeit der ersten 8 ms ab dem gemeldeten Anschlag trennt echte Slaps nicht (Median 0,002, Schwelle 0,12; die Synthese war zu hell), und bei 80 % der Noten lag der Pegel nach 60 ms beim Melden noch nicht vor. Neu (`dsp::strike`): am Spitzenwert des Anschlags ausgerichtet, Energieanteil über 1,5 kHz in 10 ms und Pegel 60–80 ms danach; das Urteil fällt, sobald 110 ms Audio da sind, sonst später als Korrektur (`TechniqueRevision`, Engine-Event, Session bewertet die Note neu). IDMT: Slap 79 %, Pop 70 %, Ghost 96 %, normale Anschläge 1,7 % mit erfundener Technik (vorher 0); Recall/Präzision unverändert 87,9/78,2 %. Die Schwellen sind an denselben Aufnahmen gewählt (ein Bass, ein Spieler): eigene Aufnahmen bleiben nötig. Synthese an IDMT angeglichen (Slap und Pop fallen nach dem Anschlag schnell ab, Daumen-Schlag tiefer): Technik-Korpus 95,8 % (vorher 91,7 %). Offen: Slides an echten Aufnahmen (0 von 20, IDMT markiert vermutlich die Ausgangsnote).
- [x] LH-BEW-06, Teil 2 (Runde 9): Bewertung der Techniken in der Session (`technique.rs`), getrennt von der Trefferquote. Soll-Technik aus der Partitur (Slide, Hammer-on, Pull-off, Slap, Pop, Ghost Note; ein Slide zählt dort, wo er ankommt), je Note und Durchlauf „gespielt“, „andere Technik“ oder „nicht gehört“; Auswertung je Technik („Hammer-on: 1 von 1“). Ziele von Hammer-on und Pull-off haben keinen Anschlag: ein Ton dort zählt nur für die Technik, nicht mehr als zusätzliche Note. Am Mikrofon werden Slap und Pop nicht bewertet, über MIDI keine Technik; die Auswertung sagt das. Eine als Ghost Note gespielte Ghost Note trifft unabhängig von der Tonhöhe (vorher „falscher Ton“, −3300 Cent). Import: Slide nur, wenn er zur nächsten Note führt (Guitar Pro Shift/Legato, MusicXML `type="start"`; vorher auch Slide-in/-out und in MusicXML beide Noten), Ghost Note aus MusicXML (Notenkopf in Klammern). J-C5 ⚙. Anzeige (Kante 1.31, `KanteTechniqueMark`): Zeichen sl, h, p, S, P, ( ) an der Note in Tab-Lane, Tabulatur und Noten, nach dem Spielen mit Rahmen (gespielt), durchgestrichen (andere Technik) oder gestrichelt (nicht gehört); Ghost Notes in der Tabulatur mit Bund in Klammern; Auswertung mit einer Zeile je Technik und dem Hinweis, dass sie nicht zur Trefferquote zählt. `kontra-cli render --techniques [--plain-every n]` spielt die Techniken einer Partitur.
- [x] LH-BEW-06, Slides an echten Aufnahmen (13.10.): Befund an den 20 IDMT-Slides: IDMT markiert die Ausgangsnote; sie wird normal angeschlagen und gleitet 150–350 ms später um 1 oder 2 Bünde (`modulationFrequencyRange` 100/200 Cent, Zielton nicht annotiert), immer aufwärts, meist auf eine andere Saite weiter. Die alte Auswertung verglich das Slide-Label mit dem Anschlag der Ausgangsnote, also 0 von 20 per Definition. Neu: Import als `slide_cents` an der Note (Technik bleibt der Anschlag, bei SP Pop), `korpus techniques` sucht den Slide dort, wo Kontra ihn meldet (Ankunft vor der nächsten Note). Damit ehrlich vorher: 0 von 20 (10 als Hammer-on, 4 als neuer Anschlag, 6 nicht gehört). Bundierte Slides gleiten nicht, sie springen von Bund zu Bund: über einen Bund ist der Übergang 0–22 ms lang, ein synthetischer Hammer-on 2–24 ms; über die Tonhöhe allein nicht trennbar. Über zwei Bünde ruht die Tonhöhe 20–40 ms auf dem Bund dazwischen: neue Regel (`Change`, ≥ 2 übereinstimmende Schätzungen und dann ≥ 50 Cent weiter = Bund passiert → Slide). Nachher: Slide 3 von 20 (alle erkannten Zwei-Bund-Slides; der vierte kommt als Anschlag an), Recall/Präzision 89,6/79,7 % unverändert, Slap 78,8 %, Ghost 96,4 % unverändert; Pop 32 von 46 unverändert (Zeile jetzt 50 mit den vier Pop-Slides aus Datei 013, die bisher in der Slide-Zeile standen), normale Anschläge 90,3 % richtig (663 von 734, vorher 647 von 718; 2 bisherige Hammer-on-Fehlalarme heißen jetzt Slide); Technik-Korpus 95,8 % unverändert. Verworfen: Anschläge mit schwachem Anstieg (< 1,4) bei 1–2 Halbtönen Abstand als Slide (5 Slides mehr, aber 4–9 echte Anschläge verloren). Offen: Ein-Bund-Slides vom Hammer-on trennen (Pegel/Klick beim Aufschlagen; braucht echte Hammer-on-Aufnahmen, IDMT hat keine); 4 Slides in 013 nicht gehört (Ausgangston nach dem Pop 30–40 Cent zu hoch, das Ziel liegt dann unter 80 Cent).
- [x] LH-EIN-03/04: Pegel-Hinweis entscheidet Core (`appcore::level`, Grenzen −60/−36/−6 dB über die letzten 3 s, Laptop-Vorschlag nach 4 s leise am Line-Eingang, nicht bei MIDI); `LevelAdvice.qml` zeigt nur noch an. J-A1 prüft jetzt den Hinweis selbst. „Laptop-Buchse verwenden“ wählt den Anschluss statt die Instrument-Einstellung.
