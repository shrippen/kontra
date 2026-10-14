# Kontra: Roadmap

Phasen und Inhalte: [`KONZEPT.md`](KONZEPT.md), Abschnitt 7. Anforderungen je Phase: [`LASTENHEFT.md`](LASTENHEFT.md).

| Phase | Ziel | Stand |
|---|---|---|
| 0 Machbarkeit | Qt-Brücke, Audio-Engine, Erkennung gegen Testkorpus, eigener Time-Stretch, Latenz, Xruns | weitgehend erledigt (synthetisch); echte Aufnahmen und Latenzmessung am Gerät fehlen |
| 1 MVP (0.1) | E-Bass, Import GP/MIDI, Tab-Lane, Tab, Noten, Spielen/Song/Warten/Werkstatt | große Teile umgesetzt, siehe unten |
| 2 Lernen (0.2–0.4) | Groove, Griffbrett, adaptive Schwierigkeit, MusicXML | MusicXML-Import vorgezogen |
| 3 Inhalte (0.5–0.7) | eigene Audiodatei, `.kontra`-Pakete, Etüden, Gehör | Backing-Track-Wiedergabe mit Time-Stretch vorhanden |
| 4 Bass komplett (0.8–1.0) | Kontrabass am Mikrofon, Bass über MIDI, Techniken, Stem-Trennung | MIDI-Eingang mit Klaviatur-Ansicht, Mikrofon mit Echo-Kompensation, Techniken erkannt; Rest offen |
| Didaktik (laufend) | Wie vermittelt Kontra Bass am besten? Ideen entwickeln, als Prototypen umsetzen, mit Personas vergleichen | offen, siehe unten |

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
| LH-BIB-11 Songsterr, MuseScore.com | Suche im Browser, „Neu in Downloads“, Ziehen und Ablegen (14.10., J-B6 ⚙); Ergebnisliste in Kontra nur mit Erlaubnis von Songsterr (offen) |
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

### Hardware-Prüfung 13.10.2026 (Eigentümer: E-Bass, Scarlett Solo, PipeWire)

`kontra-cli check`, geführt; Aufnahmen in `testdata/korpus/local/own/check-2026-10-13/` (lokal). Betrifft P1, P2, P3.

- H1 (hoch, behoben): Rundlauf 41–45 ms, über den 30 ms von LH-NF-01. Kontra forderte keinen Puffer an, PipeWire
  blieb bei 1024 Frames; erzwungenes Quantum 256/128/64: 28/20/16 ms, keine Xruns. Live-Streams fordern 128 an.
- H2 (hoch, behoben): Bass auf Eingang 2, Eingang 1 leer: der Mittelwert halbierte den Pegel, Übersteuern (0,2 s je
  Pop) blieb unter der Clip-Marke (LH-EIN-03). Der Eingang folgt jetzt dem lautesten Kanal.
- H3 (mittel, behoben, Test zuerst): Kalibrierung ohne Vorzähler; der Spieler wartete vier Klicks (4 von 8), die App
  riet „lauter spielen“. Jetzt vier Klicks Vorzähler, Hinweis nennt ihn.
- H4 (mittel, behoben): Mit Vorzähler spielte der Spieler 75–95 ms früh; der Korpus maß den Spieler (3 von 8). `check`
  schätzt den Versatz je Aufnahme; Bericht markiert Übersteuern.
- H5 (mittel, offen): Fehlalarme am echten Bass, etwa einer je Ton (Precision 50 %), meist 0,2–0,5 s nach dem Anschlag
  und 20–35 dB leiser (Finger heben: leere Saite klingt kurz an; Saitenwechsel). IDMT hat nur einzeln abklingende Töne.
  Angezeigt werden sie nicht (Extra); schaden nur, wenn einer vor dem nächsten Ton in dessen Zeitfenster fällt und ihn
  mit falscher Tonhöhe belegt (Achtel, Sechzehntel). Ein reiner Pegelfilter scheidet aus: ein echter leiser Ton lag
  26 dB unter dem davor.
- H6 (hoch, offen): Slap 8 von 8 als Dead Note, Pop 6 von 8 als normaler Ton (ohne Übersteuern). Hammer-on 4/4.
  Messung (`dsp::strike`): die Slaps sind hell (Höhenanteil 1,5–3 %), aber nach 60 ms fast still (Rest 4–15 %; IDMT-Slaps
  ≥ 28 %); die Pops nicht heller als Zupfen (0,3–0,9 %). „Hell und kurz = Slap“ bringt 7 von 8 Slaps, kostet aber IDMT-
  Dead-Notes (27 → 21 von 28, auch sie sind hell); verworfen. Braucht Aufnahmen weiterer Spieler, geübt und ungeübt.
- H7 (mittel, offen): Slide über zwei Bünde als Hammer-on gehört; Runde 11 „legato“ deckt nur einen Bund.
- H8 (mittel, offen): Ein bestimmtes Interface neben anderem Standardausgang geht nur über `PIPEWIRE_NODE`; die
  Geräteliste zeigt das Solo nur als `hw:`/`sysdefault:`, die PipeWire belegt hält.
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
  - G9 (mittel, Kante-PR): Tabulatur und Noten zeigen den Werkstatt-Abschnitt nicht, nur die Tab-Lane.
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
- [x] LH-LRN-02: Griffbrett-Quiz mit dem echten Bass (Töne auf einer Saite finden, Intervalle), Seite „Griffbrett“ (Strg+8); nach dem richtigen Ton zeigt das Griffbrett alle Lagen.
- [x] LH-BIB-06: Backing-Track abgleichen: Kontra erkennt die Schläge der Aufnahme (Spektralfluss, Tempo per Autokorrelation, Beat-Tracking nach Ellis) und legt die Takte darauf (Tempo-Karte je Takt); Takt 1 um einen Schlag, alle Takte oder ein einzelner Takt in 10-ms-Schritten korrigierbar.
- [x] LH-BIB-09 (Kann, Runde 9): Bass aus dem Backing-Track entfernen über Demucs, wenn installiert (`drivers::separator`: `KONTRA_DEMUCS`, `PATH`, `~/.local/bin`; im Flatpak `flatpak-spawn --host`, braucht eine Erlaubnis, die das Manifest noch nicht hat). `demucs --two-stems bass -n htdemucs`, Fortschritt aus dem Fortschrittsbalken auf stderr, Abbrechen beendet den Prozess. Ergebnis im Cache je Datei (Pfad, Größe, Änderungszeit); der Abgleich bleibt. Übungsseite „Ohne Bass“: ohne Demucs ausgegraut mit Erklärung, Anleitung („?“) und „Erneut prüfen“. Getestet mit einem Ersatz-Programm; mit echtem Demucs 4.1.0 gemessen (12.10.): Song von 4:11 in 17 s (RTX 3090) bzw. 92 s nur auf dem Prozessor (24 Threads); `pipx install demucs` bringt numpy nicht mit (Anleitung ergänzt). Messung auf Mindest-Hardware (Laptop) offen.
- [ ] LH-LRN-01 (Teil): Grooves auf dem Grundton (Viertel bis Sechzehntel, Pausen, Shuffle, Oktaven) mit Schlagzeug (#15), 21 mitgeliefert; Ghost Notes im Groove fehlen noch.
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
