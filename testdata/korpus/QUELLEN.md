# Quellen für echte Bass-Aufnahmen

Stand 2026-10-09. Gesucht: echte Aufnahmen von E-Bass (zweitrangig Kontrabass, bundlos) mit Annotation je Note
(Onset, Tonhöhe, wenn möglich Saite/Bund) unter einer Lizenz, die Tests erlaubt. Ergänzt [`AUFNAHME.md`](AUFNAHME.md)
(eigene Aufnahmen bleiben nötig, siehe „Bewertung“).

**Regel:** Fremde Audiodaten liegen nur in `testdata/korpus/local/` (gitignored) und werden nie committet, auch nicht
bei CC BY: Weitergabe wäre zwar erlaubt, aber das Repo soll keine großen Binärdateien tragen. NC/ND-Daten nur lokal.

## Übersicht

| Quelle | Lizenz | Größe | Inhalt / Kette | Annotation | Nutzbar für |
|---|---|---|---|---|---|
| [IDMT-SMT-Bass-Single-Track](https://zenodo.org/records/7544099) | CC BY-NC-ND 4.0 laut Zenodo; die PDF im Archiv nennt CC BY-SA 4.0. Vom Eigentümer am 9.10.2026 für Kontras Tests und CI freigegeben; Audio wird weiter nicht committet, die CI lädt es von Zenodo | 20 MB | 17 Basslinien, 4-Saiter (Fame Baphomet 4), sauberes Line-Signal, 44,1 kHz/16 Bit, 5 Anschlags- und 6 Ausdrucksarten | XML/CSV: Onset, Offset, MIDI, Saite, Bund, Technik. **Onsets sind Partitur-Raster (Guitar Pro), nicht von Hand gesetzt**, teils auf 0,1 s gerundet | Line4 (Tonhöhe, Fehlalarme, Recall ±50 ms), Laptop-Simulation, Techniken (Phase 4). **Nicht** für Onset-Genauigkeit. Regressionsprüfung in der CI (`korpus measure --pitch --window-ms 80`, Untergrenzen 81 % / 77 %); Techniken (ST → Slap, SP → Pop, DN → Ghost; SL → `slide_cents`, der Slide zählt an seiner Ankunft) mit `korpus techniques` |
| [IDMT-SMT-Bass](https://zenodo.org/records/7188892) | CC BY-NC-ND 4.0 | 1,6 GB | ca. 4300 Einzelnoten, gleicher Bass, 10 Techniken | Tonhöhe/Saite/Bund/Technik je Datei | Tonhöhe je Saite/Bund/Technik. Nicht geladen (Größe), Konverter fehlt noch. Nur lokal |
| [NSynth](https://magenta.tensorflow.org/datasets/nsynth) (Test-Split, `bass_electronic_*`) | CC BY 4.0 | 350 MB Archiv, davon 160 E-Bass-Noten | Einzelnoten aus Sample-Bibliotheken, 16 kHz, 4 s, Onset exakt bei 0 | JSON: MIDI, Velocity | Onset-Genauigkeit und Tonhöhe über den ganzen Bereich (B0–G4), Laptop-Simulation. Weitergabe mit Namensnennung erlaubt |
| [FiloBass](https://zenodo.org/records/10069709) | Annotationen CC BY 4.0; Audio sind Demucs-Stems aus kommerziellen Aebersold-Play-Alongs (Rechte ungeklärt) | 434 MB | 48 Jazz-Walking-Lines, Kontrabass, Studio, als MP3-Stem aus dem Mix getrennt | MIDI (Melodyne-Transkription, je Note zweimal von Hand geprüft), MusicXML, Beats | Kontrabass grob (Onsets ±20 ms), Tonhöhe nur mit Stimmton-Ausgleich. Nur lokal |
| [JazzSAMBA](https://zenodo.org/records/22943963) | MIT-artig (frei, Hinweis beibehalten) | 22 GB | Jazz-Band, Bass als Nahmikrofon-Stem | Bass-MIDI nur automatisch (AMT), keine Ground Truth | Nicht als Referenz; später evtl. für Fehlalarme mit Übersprechen |
| [Slakh2100](https://zenodo.org/records/4599666) / [BabySlakh](https://zenodo.org/records/4603870) | CC BY 4.0 | 104 GB / 0,9 GB | Bass aus MIDI gerendert (Sample-Instrumente) | MIDI exakt | Nur synthetisch, kein Ersatz für echte Aufnahmen |
| [MedleyDB Pitch](https://zenodo.org/records/2620624) | CC BY-NC-SA 4.0, Zugang auf Anfrage | – | einzelne Bass-Stems echter Produktionen | f0-Kurven, keine Noten | Nur mit eigener Notensegmentierung; nicht geladen |
| [PHENICX-Anechoic](https://zenodo.org/records/840025) | Annotationen CC BY 4.0; Audio Rechte bei Aalto Univ. | 760 MB | Orchester, schalltot, enthält Kontrabass-Stimmen | Note-Onset/Offset (aus Partitur ausgerichtet) | Kontrabass am Mikrofon, klassisch; nicht geladen |
| [URMP](https://labsites.rochester.edu/air/projects/URMP.html) | keine Lizenz angegeben (Formular) | 12,5 GB | Kammermusik, einzeln aufgenommene Spuren, teils Kontrabass | Noten je Spur | Kontrabass; Lizenz vorher klären |
| [Univ. Iowa MIS Kontrabass](https://theremin.music.uiowa.edu/MISdoublebass.html) | „used for any projects, without restrictions“ | ca. 100 MB | Kontrabass arco/pizz., 3 Dynamiken, alle Saiten, schalltot, Neumann KM 84 | nur Tonbereich im Dateinamen (chromatische Läufe) | Kontrabass-Tonhöhe; Onsets müssten selbst gesetzt werden (`csv`-Import) |
| GuitarSet, Guitar-TECHS, EGFxSet, MAESTRO, ENST-Drums | – | – | Gitarre, Klavier, Schlagzeug | – | ausgeschlossen |
| Freesound | je Datei verschieden | – | Einzelnoten, kein geprüftes Bass-Paket gefunden | Tonhöhe nur im Titel | nur einzeln nach Prüfung (CC0/CC BY) |

Für bundlosen Bass gibt es keine annotierten freien Aufnahmen: der Satz `fretless-5` muss selbst aufgenommen werden.

## Gewählt und geladen

`testdata/korpus/local/dl/` (Rohdaten, 520 MB) und `testdata/korpus/local/conv/` (umgewandelt, 115 MB):

- `dl/idmt-st/`, `conv/idmt-st/` (17 Linien, 948 Noten), `conv/idmt-st-norm/` (dieselben auf −6 dBFS Spitze)
- `dl/nsynth/nsynth-test/` (nur `bass_electronic_*` und `examples.json`), `conv/nsynth/` (3 Instrumente, 157 Noten)
- `dl/filobass/`, `conv/filobass/` (6 Stücke, je die ersten 60 s, 842 Noten)

Laden:

```sh
cd testdata/korpus/local && mkdir -p dl/idmt-st dl/filobass dl/nsynth
curl -L -o dl/idmt-st/st.zip https://zenodo.org/api/records/7544099/files/IDMT-SMT-BASS-SINGLE-TRACKS.zip/content
(cd dl/idmt-st && unzip -q st.zip)
curl -L -o dl/filobass/fb.zip https://zenodo.org/api/records/10069709/files/FiloBass_v1.0.0.zip/content
(cd dl/filobass && unzip -q fb.zip -x '__MACOSX/*')
curl -L http://download.magenta.tensorflow.org/datasets/nsynth/nsynth-test.jsonwav.tar.gz \
  | tar -xz -C dl/nsynth --wildcards '*examples.json' '*bass_electronic*'
```

## Umwandeln

`scripts/korpus-import.py` (nur Standardbibliothek, MP3 über `ffmpeg`) schreibt je Stück `<name>.wav` (Mono, 16 Bit) und
`<name>.json` im Format von `KorpusItem` (`set`, `sample_rate`, `notes[{onset_s, midi}]`). Zusätzlich steht `meta`
darin (Quelle, Lizenz, Kette, je Note Saite/Bund/Technik); serde ignoriert das Feld. Vor jedes Stück kommen 0,5 s Stille
(`--pad`), weil IDMT-Linien bei 0 s beginnen.

```sh
S=scripts/korpus-import.py; L=testdata/korpus/local
python3 $S idmt-st  $L/dl/idmt-st $L/conv/idmt-st [--normalize -6] [--exclude-expr HA,DN]
python3 $S nsynth   $L/dl/nsynth/nsynth-test $L/conv/nsynth
python3 $S filobass "$L/dl/filobass/FiloBass ISMIR Publication" $L/conv/filobass --max-seconds 60 --limit 6
python3 $S csv aufnahme.wav onsets.csv $L/conv/eigen --name riff-1 --set Line5   # CSV: onset_s,midi
```

`kontra-cli korpus measure DIR… [--window-ms 50] [--laptop-jack] [--verbose]` wertet jedes `*.json` mit seiner
`set` aus (Instrument und Eingang), mit wählbarem Fenster (Raster-Annotationen wie IDMT brauchen ±50 ms) und der
Laptop-Simulation; `--verbose` listet Fehler, falsche Tonhöhen und zusätzliche Erkennungen je Datei. Es misst nur und
bricht nicht ab; `korpus check` bleibt die harte Prüfung des synthetischen Korpus.

## Messung (Erkennung Stand 2026-10-09, `kontra-cli analyze --json`, Abgleich wie `korpus.rs::compare`)

| Material | Fenster | Noten | erkannt | Recall | Präzision | Tonhöhe | Onset mean / p95 | Cent (richtige) |
|---|---|---|---|---|---|---|---|---|
| NSynth E-Bass, Line | ±30 ms | 157 | 195 | 100 % | 80,5 % | 63,7 % | 2,8 / 6,5 ms | 15,0 |
| NSynth E-Bass, Laptop-Simulation | ±30 ms | 157 | 172 | 89,2 % | 81,4 % | 75,7 % | 10,0 / 18,6 ms | 15,8 |
| IDMT Line (Originalpegel −15…−26 dBFS) | ±30 ms | 948 | 1495 | 78,7 % | 49,9 % | 82,8 % | (Raster) | 11,4 |
| IDMT Line | ±50 ms | 948 | 1495 | 94,3 % | 59,8 % | 82,3 % | (Raster) | 11,8 |
| IDMT Line ohne Flageolett/Dead Notes | ±50 ms | 888 | 1495 | 94,3 % | ≤ 59 % | 87,0 % | (Raster) | 11,8 |
| IDMT auf −6 dBFS, ohne HA/DN | ±50 ms | 888 | 1722 | 95,0 % | 49,0 % | 84,8 % | (Raster) | 11,7 |
| IDMT Laptop-Simulation, Originalpegel, ohne HA/DN | ±50 ms | 888 | 353 | 36,0 % | 90,7 % | 80,3 % | (Raster) | 13,9 |
| IDMT −6 dBFS + Laptop-Simulation, ohne HA/DN | ±50 ms | 888 | 921 | 78,9 % | 76,1 % | 90,2 % | (Raster) | 12,7 |
| FiloBass Kontrabass (Stem) | ±50 ms | 842 | 704 | 59,9 % | 71,6 % | 42,5 % | 8,5 ms | 23,7 |

Ziele (LH-NF-02/03): Recall und Präzision ≥ 99 %, Tonhöhe ≥ 98 % (Laptop ≥ 95 %), Onset p95 < 10 ms. Auf echten
Aufnahmen wird keins davon erreicht; nur die Onset-Genauigkeit bei sauberen Einsätzen (NSynth Line) liegt im Ziel.

Unabhängig von der Annotationsqualität: auf IDMT liefert die Erkennung 1495 Noten für 948 gespielte (+58 %), auf
NSynth 195 für 157 Einzelnoten.

### Nach den Korrekturen (Branch `erkennung-echt`)

Bewertung mit Tonhöhe: eine Note gilt als gefunden, wenn eine Erkennung ±80 ms daneben liegt **und** die gerundete
Tonhöhe stimmt (das ±50-ms-Fenster ohne Tonhöhe zählt bei IDMT tonlose Vorläufer als Treffer und wertet richtige
Töne 52–60 ms neben dem Partitur-Raster als Fehler). F1 = harmonisches Mittel aus Recall und Präzision.

| Material | Noten | vorher erkannt | vorher R / P / F1 | nachher erkannt | nachher R / P / F1 |
|---|---|---|---|---|---|
| IDMT Line | 948 | 1495 | 85,8 / 54,4 / 66,6 | 989 | 82,5 / 79,1 / **80,8** |
| NSynth Line | 157 | 195 | 86,0 / 69,2 / 76,5 | 158 | 86,0 / 85,4 / **85,7** |
| FiloBass | 842 | 704 | 28,1 / 33,7 / 30,6 | 595 | 28,0 / 39,7 / **32,9** |

- Befund 1 behoben: ein zweiter Einsatz binnen 80 ms, bevor der erste eine Tonhöhe hat, gehört zum selben Anschlag
  (Onset des ersten, Tonhöhe nach dem zweiten). Keine `midi 0`-Noten mehr in NSynth.
- Befund 2 größtenteils behoben: ein Einsatz zählt nur, wenn der Spitzenpegel 0–40 ms danach den von −20…−8 ms davor
  um 10 % übersteigt. Gewählt an den Erkennungen der alten Version (946 richtige, 608 falsche auf IDMT und NSynth):
  behält 90 % der richtigen, verwirft 62 % der falschen. Strengere Faktoren (1,25/1,5) verwerfen kaum mehr falsche,
  aber mehr leise Töne nach lauten; reine Pegelmerkmale kommen nicht über 90 % der richtigen hinaus.
- Befund 3 nur teilweise: YIN prüft ohne klaren Einbruch unter der Schwelle jetzt die halbe Periode (NSynth +2,5
  Punkte Tonhöhe); FiloBass hat weiter rund 100 Noten eine Oktave zu tief. Siehe „Oktavfehler am Kontrabass“.
- Synthetischer Korpus unverändert 100 %.

### Oktavfehler am Kontrabass (Branch `oktavfehler`, 13.10.)

FiloBass ±50 ms, Erkennung je annotierter Note gegen die Annotation, Stimmton je Stück abgezogen (Median, +4 bis
−21 Cent). Vorher: 81 Noten eine Oktave zu tief, 6 zu hoch, 15 eine Oktave plus Quinte zu tief.

- Betroffen sind D2–A#2 (38–46), nicht die tiefen Töne; 57 der 81 in „All the Things You Are“. Nicht nach lauten
  Tönen (10 % der Fehler, 26 % der richtigen folgen einem um 3 dB lauteren Ton): „leise nach laut“ ist eine andere
  Baustelle.
- Ursache: In den ersten 55 ms nach dem Anschlag liegt ein Anteil eine Oktave unter dem Ton (G2 bei 96,7 Hz: 48 Hz
  mit −6…−11 dB), der binnen rund 50 ms verklingt; über 200 ms gemessen liegt er bei −25 dB. Vermutlich die Bassdrum
  im Demucs-Stem (Jazz spielt sie auf jedem Viertel) oder der Korpus. Das Signal wiederholt sich dann nach der
  doppelten Periode besser (YIN-Differenz 0,10) als nach der einfachen (0,19–0,47); die Schwelle 0,15 nimmt die
  doppelte, zwei Schätzungen im Abstand von 5 ms stimmen überein, die Note geht eine Oktave zu tief hinaus.
- Korrektur (`Yin::estimate_attack`, nur für die Schätzungen nach einem Anschlag): die halbe Periode gilt, wenn
  sie sich ordentlich wiederholt (Differenz ≤ 0,5), die ungeraden Harmonischen 3, 5, 7 der langen Periode unter 15 %
  der geraden liegen und der tiefste Teilton im Fenster gegen den 2. verklingt (Leistung erste/letzte Periode
  < 0,5). Das Verklingen trennt: gespielte A1 (leere A-Saite) haben auf diesen Stems ebenfalls schwache ungerade
  Harmonische, ihr Grundton klingt aber weiter. Am klingenden Ton (Legato-Verfolgung) und im Stimmgerät bleibt YIN
  unverändert, sonst folgte die Prüfung dem Bass der Band aus den Lautsprechern (`room-double-backing`).

| Material | vorher | nachher |
|---|---|---|
| FiloBass Oktave zu tief / zu hoch | 81 / 6 | 30 / 8 |
| FiloBass ±50 ms R / P / Tonhöhe | 68,8 / 70,2 / 57,2 % | 68,9 / 70,1 / 64,3 % |
| IDMT wie CI (±80 ms, mit Tonhöhe) R / P | 89,6 / 79,7 % | 90,3 / 80,4 % |
| IDMT Laptop-Simulation (±80 ms, mit Tonhöhe) R / P | 37,3 / 62,8 % | 37,9 / 63,8 % |
| NSynth ±30 ms R / P / Tonhöhe | 100 / 95,7 / 90,4 % | 100 / 95,7 / 92,4 % |
| NSynth Laptop-Simulation R / P / Tonhöhe | 87,3 / 47,1 / 88,3 % | 87,3 / 46,8 / 90,5 % |
| Synthetischer Korpus (`korpus check`), Techniken (`korpus techniques`) | | unverändert |

NSynth Laptop: zwei zusätzliche Erkennungen im Netzbrummen nach einem ausklingenden Ton (`bass_electronic_027`,
200,8 s), drei Oktavfehler weniger (`018`). Rechenzeit der Analyse unverändert (±1 %), Wartezeit bis zur Note nur
bei Noten mit Wummern eine Schätzung (5 ms) länger.

Offen: 30 Noten bleiben eine Oktave zu tief. Dort wiederholt sich die halbe Periode schlechter als 0,5 oder der
tiefe Anteil klingt im ersten Fenster nicht hörbar ab; 0,6 und mehr kosteten an `room-double` Tonhöhe (98,1 →
94,2 %). Im ersten Fenster gleichen sie gespielten A1; trennen ließe sich das erst mit mehr Audio (Latenz) oder einer
späteren Korrektur der Tonhöhe.

### Laptop-Pegel (Branch `laptop-pegel`, 13.10.)

Simulation (`synth::laptop_jack`): −15 dB, Tiefpass 2,5 kHz (Impedanz), Brummen 50 Hz −60 dBFS und 150 Hz −66 dBFS
Spitze, Rauschen −71 dBFS RMS. IDMT (Spitzen −15…−26 dBFS) liegt danach bei −30…−40 dBFS, 20–30 dB über dem
Brummen: ein ungünstiger, aber üblicher Fall (Eingang leise gestellt, Netzteil steckt). Mehr Verstärkung hebt Brummen
und Rauschen mit; entscheidend ist der Abstand, nicht der Pegel.

Zerlegt (IDMT, ±80 ms mit Tonhöhe, vorher 37,9 / 63,8 %):

| Simulation nur mit | R / P |
|---|---|
| −15 dB | 86,8 / 86,4 % |
| −15 dB + Tiefpass | 84,7 / 86,3 % |
| −15 dB + Rauschen, ohne Brummen | 56,2 / 81,6 % |
| −15 dB + Brummen 50 Hz, ohne Rauschen | 68,8 / 66,2 % |

Ursachen:

- **Messung:** `korpus measure --laptop-jack` wertete mit dem Profil `Line` aus; P1 spielt mit `laptop-jack`. Das
  Profil änderte an der Erkennung ohnehin nichts.
- **Brummen:** Von 204 Fehlerkennungen lagen 94 bei 49–52 Hz (MIDI 31/32), 61 eine Oktave und 10 eine Oktave plus
  Quinte unter dem Ton: YIN rastet auf 50 Hz ein (G2, D3 sind fast ganzzahlige Vielfache).
- **Einsatzschwelle:** Der Fluss-Abstand 8 ist absolut. Mit Rauschboden und ohne Höhen steigt ein leiser Anschlag nur in
  wenigen Bändern (IDMT 004, A1 bei 0,5 s: Fluss 3,2 statt 10,7); in 004, 009, 010 und 012 keine einzige Note.

Korrektur, nur im Profil `laptop-jack`:

- Brumm-Kerben (50/60 Hz und Harmonische, wie im Stimmgerät) vor Einsatz und YIN, 0,5 Hz breit. 3 Hz (Stimmgerät)
  und 1 Hz klingen nach dem Anschlag bei 50 Hz nach und zogen G2/C2 eine Oktave tiefer. Die Feinbestimmung
  (`refine_hz`) liest das ungefilterte Signal: mit Kerben stiegen die Cent-Fehler am synthetischen Laptop-Satz
  (p95 4,6 → 18,7 ct).
- Fluss-Abstand 2 statt 8. Am Line-Eingang bliebe 8: dort kostete 2 fünf Punkte Präzision.

| Material (Profil laptop-jack) | vorher | nachher |
|---|---|---|
| IDMT Laptop-Simulation ±80 ms mit Tonhöhe, R / P | 37,9 / 63,8 % | 87,7 / 89,5 % |
| NSynth Laptop-Simulation ±30 ms, R / P / Tonhöhe | 87,3 / 46,8 / 90,5 % | 100 / 94,0 / 93,6 % |
| NSynth Laptop-Simulation ±30 ms mit Tonhöhe, R / P | 79,0 / 42,3 % | 93,6 / 88,0 % |
| FiloBass Laptop-Simulation ±50 ms mit Tonhöhe, R / P | 36,3 / 46,8 % | 48,5 / 44,0 % |
| Synthetischer Laptop-Satz, Cent mittel / p95 / max | 1,34 / 4,6 / 10,1 | 1,31 / 4,7 / 9,6 |
| IDMT wie CI, NSynth, FiloBass (Line), `korpus check`, `korpus techniques` | | unverändert |

Robustheit (IDMT, nachher): ohne Absenkung 89,6 / 80,5 %, −6 dB 89,8 / 84,3 %, doppeltes Brummen 82,2 / 86,7 %,
dreifaches Rauschen 78,2 / 85,2 %. Ein sauberes Line-Signal mit Profil laptop-jack: 91,1 / 74,7 % (Line-Profil
89,8 / 79,7 %). Rechenzeit der Laptop-Messung +25 % (mehr erkannte Noten, sechs Kerben), Line unverändert;
keine zusätzliche Wartezeit.

Offen: echte Aufnahme aus einer Laptop-Buchse (`AUFNAHME.md`); die Simulation hat genau 50 Hz, das Netz schwankt
um ±0,05 Hz (Kerbe dort noch −14 dB).

### Halbton zu tief am Kontrabass (Branch `halbton`, 13.10.)

Stand `oktavfehler`, FiloBass ±50 ms, nächste Erkennung je Note, Stimmton je Stück abgezogen (Median der auf ±50 Cent
gefalteten Abweichungen: +3 bis −12 Cent). 88 von 571 zugeordneten Noten einen Halbton zu tief (376 richtig).

- Weder Stimmung noch Annotation: Die Fehler verteilen sich auf vier Stücke (19–23 je Stück, Apple Jump 3, All the
  Things You Are 4). Abweichung gleitend, keine Häufung bei −100: 65 zwischen −60 und −99 Cent, 23 bei −100…−150.
  D2 (38) gehäuft: 29 der 88, unter allen zugeordneten 15 %.
- Ursache: Der Ton beginnt zu tief und steigt in rund 100 ms auf die Annotation (eigenes YIN, 120-ms-Fenster ab
  Anschlag, Median gegen Annotation): 0–60 ms −52 Cent, 40–100 ms −18, 80–140 ms −5, ab 150 ms −3. Bei richtigen
  Noten −15, −4, −2, −1. Die Erkennung entscheidet im ersten Fenster (12–66 ms nach dem Einsatz, D1 als tiefster Ton)
  auf zwei Schätzungen im Abstand von 5 ms; die stimmen auch mitten im Anstieg überein. Herkunft offen (Hineingleiten,
  Saite, Quellentrennung); ein tiefer Anteil unter dem Grundton ist es nicht (gleich stark wie bei richtigen Noten).
- Folgen: 25 der 88 bekommen zusätzlich eine Legato-Note (Hammer-on/Slide) mit dem richtigen Ton, sobald der Anstieg
  80 Cent erreicht; die übrigen bleiben einen Halbton zu tief.
- Korrektur nur nach der Ausgabe, sonst wartet jede Note länger: Die Tonhöhe des klingenden Tons 90–160 ms nach dem
  Einsatz (Median der Verfolgungs-Schätzungen) rundet bei 75 von 83 auf die Annotation und kippt 11 richtige. IDMT
  (±80 ms) ebenso: 17 Halbton- und 13 Oktavfehler richtig, 9 richtige falsch. Braucht eine Tonhöhen-Revision wie
  `TechniqueRevision` bis in die Bewertung (Note neu werten); offen als Entscheidung.
- Stimmton in `korpus measure`: bringt bei höchstens 12 Cent wenig, nicht umgesetzt.

## Befunde

1. **Einsatz ohne Tonhöhe plus zweite Note.** Bei tiefen Tönen meldet die Erkennung am Anschlag eine Note mit
   `midi = 0` und 35–55 ms später eine zweite mit Tonhöhe. NSynth `bass_electronic_018` bei 27,500 s (D1, 26):
   27,504 s `midi 0`, 27,542 s `midi 26,03`; ebenso 63,500 s (39) und `bass_electronic_027` 9,500 s (25). Betrifft
   bei NSynth 32 von 157 Noten, gehäuft B0–C2 (17), aber bis D#4. Werden beide zusammengelegt (Onset der ersten, Tonhöhe der zweiten),
   steigt NSynth auf Präzision 95,7 % und Tonhöhe 83,4 %.
2. **Mehrfachauslösung im Ausklang.** IDMT 008 (Fingerstyle, nur normale Noten): Note bei 1,423 s, weitere
   Erkennungen bei 1,456 s (42,7), 1,658 s (42,8), 1,827 s (`midi 0`). NSynth: neue Note genau beim Loslassen 3 s nach
   dem Anschlag (`bass_electronic_018` 48,498 s, `027` 66,505 s). Auf Normalpegel (−6 dBFS) wird es schlimmer (1722
   statt 1495 Erkennungen); die Tiefpass-Laptop-Simulation dämpft es.
3. **Oktave zu tief und unter dem Tonumfang.** IDMT 010: erwartet D2 (38), erkannt 25,90; A2 (45) → 32,94. Erkannt
   werden Töne unter dem tiefsten Ton des Instruments (E1 = 28 beim 4-Saiter). NSynth 018: 33 → 20,98, 35 → 22,99.
   FiloBass: 118 von 506 zugeordneten Noten eine Oktave zu tief.
4. **Leises Signal über die Laptop-Buchse fällt aus.** IDMT im Originalpegel mit Simulation (−15 dB): Recall 36 %.
   Ein passiver Bass an einer Kombibuchse ist ähnlich leise. Behoben am 13.10. (Brumm-Kerben, niedrigere Einsatzschwelle im
   Profil laptop-jack): 87,7 % bei 89,5 % Präzision, siehe „Laptop-Pegel“.
5. **Techniken.** Flageolett (IDMT 017) wird als Grundton erkannt (erwartet 67, erkannt 32,88), Dead Notes bekommen
   eine Tonhöhe; Slap-Pluck nur 63 % richtige Tonhöhe. Gehört zu Phase 4.
6. **Annotationen.** IDMT-Onsets sind Partiturzeiten (IDMT 008: Abstände genau 0,2308 s, Linien 002/016/017 auf 0,1 s
   gerundet); nur 28 % liegen innerhalb 10 ms des Energie-Einsatzes. FiloBass-Aufnahmen sind gegen A = 440 Hz
   verstimmt (Median −18 Cent in „All the Things You Are“) und durch die Quellentrennung verfärbt.

## Bewertung

- Fremde Datensätze ersetzen die eigenen Aufnahmen nach `AUFNAHME.md` nicht: kein Datensatz hat exakte Onsets an
  echtem E-Bass, keiner hat 5-Saiter, Laptop-Buchse oder bundlosen Bass.
- Brauchbar sind sie als Regressionsprobe: NSynth für Onset und Tonhöhe, IDMT (±50 ms) für Fehlalarme, Tonhöhe und
  Techniken, FiloBass nur grob für Kontrabass.
