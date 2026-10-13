# Echte Aufnahmen für den Testkorpus

Die Erkennung ist bisher nur mit synthetischen Bässen geprüft (`kontra-cli korpus check`). Für LH-NF-02/03 braucht es
echte Aufnahmen. Sie liegen in `testdata/korpus/local/` (nicht in Git) und werden mit derselben Prüfung ausgewertet.

## Prüfmodus (Runde 10)

Am einfachsten mit dem Prüfmodus: Er spielt den Klick, nimmt den Bass direkt in Kontra auf und schreibt jede Aufnahme
mit Annotation als Paar nach `testdata/korpus/local/own/check-<datum>/takes/`, dazu `report.md` und `report.json`.

```sh
kontra-cli devices                                   # Ein- und Ausgang wählen (id in eckigen Klammern)
kontra-cli check --input <id> --output <id>          # Loopback, „auf den Klick“, Aufnahmen
kontra-cli check --steps soak --soak-minutes 60      # Dauertest (LH-NF-04)
kontra-cli check --steps stems --song lied.mp3       # Demucs-Zeit (LH-BIB-09)
kontra-cli check --laptop-jack --takes open-strings,fretted   # dasselbe an der Laptop-Buchse
```

1. **Loopback:** Ausgang des Interfaces per Kabel in den Eingang, Mithören aus. Kontra spielt zwölf Töne und misst
   die Rundlaufzeit (Ausgang + Eingang + Erkennung).
2. **Auf den Klick:** Bass einstecken, auf jeden der acht Klicks eine leere Saite. Die gemessene Latenz legt die
   Annotationen der Aufnahmen auf den Anschlag.
3. **Aufnahmen** (80 BPM, vier Klicks vorweg, 4-Saiter in Standardstimmung): leere Saiten, gegriffene Töne, Slap, Pop,
   Dead Notes, Hammer-on, Pull-off, Slide. Kontra zeigt, was zu spielen ist, als Tab (`hA2` = Hammer-on auf A-Saite
   Bund 2, `xA0` = Dead Note) und misst danach gleich, wie viele Noten, Töne und Techniken es gefunden hat.
4. Am Ende listet der Bericht, was nur von Hand geht (Klang, Beamer, zwei MIDI-Keyboards, Mikrofon mit Lautsprechern).

Auswerten wie die IDMT-Aufnahmen: `kontra-cli korpus measure <ordner>/takes --window-ms 80 --pitch` und
`kontra-cli korpus techniques <ordner>/takes`.

## Aufnehmen (von Hand)

Format: WAV, 48 kHz, Mono, 24 Bit oder Float, Pegel so, dass die lautesten Töne bei etwa −6 dBFS liegen.

1. Die Vorlage als Klick-Spur erzeugen: `kontra-cli korpus write /tmp/korpus` schreibt für jeden Satz eine WAV
   (synthetischer Bass) und die Annotation (`<satz>.json` mit Zeit und Tonhöhe jeder Note).
2. Mit Kopfhörern zur synthetischen Spur mitspielen und nur den eigenen Bass aufnehmen, ab Sample 0 der Vorlage
   (gleicher Start, z. B. in einer DAW auf zwei Spuren). Die Annotation gilt dann für die eigene Aufnahme.
3. Datei als `testdata/korpus/local/<satz>.wav` speichern, die JSON-Datei daneben kopieren.

Runde 6 (9.10.2026): der Eigentümer nimmt einen 4-Saiter über das Interface und über die Laptop-Buchse auf. Diese
drei Sätze werden echt; die übrigen bleiben synthetisch (und IDMT/NSynth aus `QUELLEN.md`).

| Satz | Instrument | Eingang | Persona | Aufnahme |
|---|---|---|---|---|
| `line-4` | E-Bass, 4 Saiten | Interface (Hi-Z) | P2, P4 | Eigentümer |
| `laptop-jack-4` | E-Bass, 4 Saiten | Klinke direkt in die Laptop-Buchse | P1 | Eigentümer |
| `sixteenths` | E-Bass, 4 Saiten | Interface | P2 | Eigentümer |
| `line-5` | E-Bass, 5 Saiten | Interface (Hi-Z) | P3 | synthetisch |
| `fretless-5` | Bundloser Bass, absichtlich leicht verstimmt gespielt | Interface | P3 | synthetisch |

Beim bundlosen Satz die tatsächliche Intonation mit einem Stimmgerät-Mitschnitt prüfen und in der JSON-Datei
nachtragen (`midi` mit Nachkommastellen), sonst misst die Prüfung die Verstimmung als Fehler.

## Prüfen

```sh
kontra-cli korpus check testdata/korpus/local
```

Ergebnis und Abweichungen gehören in `ROADMAP.md` (QA-Runde). Unterschreitet ein Satz die Ziele, zuerst einen Test
mit der Aufnahme anlegen (nur lokal), dann die Erkennung verbessern.
