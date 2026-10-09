# Echte Aufnahmen für den Testkorpus

Die Erkennung ist bisher nur mit synthetischen Bässen geprüft (`kontra-cli korpus check`). Für LH-NF-02/03 braucht es
echte Aufnahmen. Sie liegen in `testdata/korpus/local/` (nicht in Git) und werden mit derselben Prüfung ausgewertet.

## Aufnehmen

Format: WAV, 48 kHz, Mono, 24 Bit oder Float, Pegel so, dass die lautesten Töne bei etwa −6 dBFS liegen.

1. Die Vorlage als Klick-Spur erzeugen: `kontra-cli korpus write /tmp/korpus` schreibt für jeden Satz eine WAV
   (synthetischer Bass) und die Annotation (`<satz>.json` mit Zeit und Tonhöhe jeder Note).
2. Mit Kopfhörern zur synthetischen Spur mitspielen und nur den eigenen Bass aufnehmen, ab Sample 0 der Vorlage
   (gleicher Start, z. B. in einer DAW auf zwei Spuren). Die Annotation gilt dann für die eigene Aufnahme.
3. Datei als `testdata/korpus/local/<satz>.wav` speichern, die JSON-Datei daneben kopieren.

| Satz | Instrument | Eingang | Persona |
|---|---|---|---|
| `line-4` | E-Bass, 4 Saiten | Interface (Hi-Z) | P2, P4 |
| `line-5` | E-Bass, 5 Saiten | Interface (Hi-Z) | P3 |
| `laptop-jack-4` | E-Bass, 4 Saiten | Klinke direkt in die Laptop-Buchse | P1 |
| `fretless-5` | Bundloser Bass, absichtlich leicht verstimmt gespielt | Interface | P3 |
| `sixteenths` | E-Bass | Interface | P2 |

Beim bundlosen Satz die tatsächliche Intonation mit einem Stimmgerät-Mitschnitt prüfen und in der JSON-Datei
nachtragen (`midi` mit Nachkommastellen), sonst misst die Prüfung die Verstimmung als Fehler.

## Prüfen

```sh
kontra-cli korpus check testdata/korpus/local
```

Ergebnis und Abweichungen gehören in `ROADMAP.md` (QA-Runde). Unterschreitet ein Satz die Ziele, zuerst einen Test
mit der Aufnahme anlegen (nur lokal), dann die Erkennung verbessern.
