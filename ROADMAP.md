# Kontra: Roadmap

Phasen und Inhalte: [`KONZEPT.md`](KONZEPT.md), Abschnitt 7. Anforderungen je Phase: [`LASTENHEFT.md`](LASTENHEFT.md).

| Phase | Ziel | Stand |
|---|---|---|
| 0 Machbarkeit | Qt-Brücke, Audio-Engine, Erkennung gegen Testkorpus, eigener Time-Stretch, Latenz, Xruns | weitgehend erledigt (synthetisch); echte Aufnahmen und Latenzmessung am Gerät fehlen |
| 1 MVP (0.1) | E-Bass, Import GP/MIDI, Tab-Lane, Tab, Noten, Spielen/Song/Warten/Werkstatt | große Teile umgesetzt, siehe unten |
| 2 Lernen (0.2–0.4) | Groove, Griffbrett, adaptive Schwierigkeit, MusicXML | MusicXML-Import vorgezogen |
| 3 Inhalte (0.5–0.7) | eigene Audiodatei, `.kontra`-Pakete, Etüden, Gehör | Backing-Track-Wiedergabe mit Time-Stretch vorhanden |
| 4 Bass komplett (0.8–1.0) | Kontrabass am Mikrofon, Bass über MIDI, Techniken, Stem-Trennung | offen |
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
- [ ] Latenz Anschlag → Anzeige am echten Gerät messen
- [x] Eigener Time-Stretch (Phasen-Vocoder), Tests synthetisch; Hörvergleich mit Signalsmith offen
- [x] Kante-Bauteile als PR in Kante (#37): Tab-Lane, Tabulatur, Notenzeile, Griffbrett, Stimmgerät, Pegel, Histogramm, Transport

## Phase 1 (Stand)

| Anforderung | Stand |
|---|---|
| LH-BIB-01/02 Import GP3–8, MIDI | umgesetzt, gegen Fremddateien geprüft |
| LH-BIB-03 MusicXML | umgesetzt (vorgezogen) |
| LH-BIB-04 Bibliothek | „Zuletzt gespielt“ mit Suche; Setlisten offen |
| LH-ANZ-01/02/03 Tab-Lane, Tab, Noten | umgesetzt (Kante) |
| LH-MOD-01..04 Spielen, Song, Warten, Werkstatt | Logik umgesetzt und getestet; Werkstatt-Bedienung in der App minimal |
| LH-MOD-05 Metronom, Einzähler | umgesetzt |
| LH-WIE-01 Time-Stretch | umgesetzt; Backing-Track je Stück wählbar |
| LH-WIE-03 Referenzton | umgesetzt |
| LH-BEW-01..05 Bewertung | umgesetzt |
| LH-TUN-01/03 Stimmgerät | umgesetzt |
| LH-EIN-01 Einrichtungsassistent | umgesetzt (5 Schritte, überspringbar, startet die erste Übung) |
| LH-EIN-03/04 Pegel, Laptop-Hinweis | Pegelanzeige mit Hinweisen; Assistent offen |
| LH-EIN-05 Latenz-Kalibrierung | „Spiel auf den Klick“ umgesetzt, mit Aufnahme getestet (40 ms ± 3) |
| LH-EIN-06 Bluetooth-Hinweis | umgesetzt |
| LH-FOR-01/03 Fortschritt | Läufe werden gespeichert; Bibliothek zeigt bestes Ergebnis und Übungszeit |
| LH-ALL-01/02 Sprache, Stil | umgesetzt |
| LH-NF-07 Barrierefreiheit | Kante-Bauteile mit Namen und Formen; Tastaturdurchlauf der App offen |

## Entscheidungen Runde 3 (9.10.2026)

- 3D-Highway (nach 1.0): Qt bleibt. Zuerst eine perspektivische Lane (2,5D) in Qt Quick (LGPL, gleiches Fenster,
  Kante-Farben, Screenreader); echtes 3D später neu bewerten. Qt Quick 3D ist in Qt 6.10 nur GPL-3 oder kommerziell.
- Veröffentlichung: Flatpak erst lokal bauen und testen, Flathub nach 0.1.
- Landing Page jetzt im Kontra-Repo (`docs/`), kein Eintrag in der öffentlichen Übersicht bis 1.0.
- Logo: Claude entwirft 10 Vorschläge zur Auswahl.
- Didaktik: Bestandsaufnahme jetzt (`DIDAKTIK.md`), Prototypen nach 0.1.

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

## QA-Runden

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
