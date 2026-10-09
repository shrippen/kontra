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
| LH-MOD-05 Metronom, Einzähler | umgesetzt; Lautstärke im Mixer |
| LH-WIE-01 Time-Stretch | umgesetzt; Backing-Track je Stück wählbar |
| LH-WIE-03 Referenzton, Mixer | umgesetzt: Backing, Metronom, Referenzton, Mithören (Standard aus; bei verschiedenen Abtastraten von Ein- und Ausgang kein Mithören, mit Hinweis) |
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

## Entscheidungen Runde 5 (9.10.2026)

- Rückmeldung: während des Spiels dosiert (Zeichen), ausführlich danach (KONZEPT 5.1 angepasst).
- Adaptive Schwierigkeit (Phase 2): nur zwischen Durchläufen, sichtbar, als Vorschlag (LH-MOD-06).
- Werkstatt: Zielband 80–90 % Treffer statt „3 fehlerfrei“, umgesetzt.
- Didaktik-Prototypen nach 0.1: Blind-Probe, Stellen-Wiederholung über Tage, Groove-Start.
- Testspieler: nur der Eigentümer.
- Eigene Aufnahmen: teilweise, was vorhanden ist (welche Bässe und Eingänge: Runde 6).
- 0.1 ist fertig, wenn Phase 1 vollständig ist; der Erkennungsstand wird im Release-Log offen genannt.
- Zuerst: Werkstatt-Bedienung, Tastatur und Screenreader, Pegel-Assistent (alle drei umgesetzt).
- Flatpak-Bau in der Gitea-CI.
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

- [x] LH-TUN-02 / LH-EIN-01: Stimmung im Profil (Standard, Drop D bzw. Drop A bei 5/6 Saiten, ½ und 1 Ton tiefer, eigene mit Notenwahl je Saite) in Einstellungen und Assistent; Stimmgerät (geführt) und Tab-Zuordnung von Dateien ohne Stimmung (MIDI, MusicXML) nutzen sie, ein Wechsel ordnet das geladene Stück sofort neu zu.
- [x] LH-TUN-04: Kammerton A 430–450 Hz (Standard 440) für Stimmgerät, Erkennung und Bewertung.
- [x] LH-BEW-02: Zeitfenster „Eigene“ mit ±20–150 ms.
- [x] LH-ANZ-06: Saitenfarben Kante / Einfarbig / Kräftig (nur Kante-Farben) mit Vorschau; wirkt auf die Tab-Lane (Tabulatur und Griffbrett haben in Kante keine Saitenfarben).
- [x] LH-ANZ-08: Linkshänder-Griffbrett (Sattel rechts).
- [x] LH-EIN-02: ohne gewählten Eingang nimmt Kontra ein USB-Interface vor der internen Karte; „Automatisch: <Gerät>“ zeigt, welches.
- [x] LH-EIN-10: MIDI und Mikrofon stehen als deaktivierte Einträge „Kommt später“ in der Anschluss-Auswahl.
- [x] LH-ALL-05: Info-Knöpfe für Hi-Z, Latenz und das Werkstatt-Zielband; Zeitfenster, Kammerton, Stimmung und Quantum-Hinweis erklären sich im Text. „Lage“ kommt in der Oberfläche nicht vor.
