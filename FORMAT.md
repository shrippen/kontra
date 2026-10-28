# Kontra-Paket `.kontra` (Format 2)

Offenes Paketformat zum Weitergeben eines Stücks zum Üben (LH-BIB-07, Journey J-D2): Noten, Loops, Zieltempo und
auf Wunsch die abgeglichene Begleitspur. Der Fortschritt des Absenders kommt nicht mit (Runde 15).
Ein `.kontra` ist ein gewöhnliches Zip-Archiv (Deflate oder unkomprimiert, kein Zip64, Namen in UTF-8);
jedes Zip-Programm kann es öffnen. MIME-Typ: `application/x-kontra-package`.

```text
lied.kontra
├── manifest.json      Pflicht: Format, Version, Titel, Notendatei, Spur, Loops, Zieltempo, Begleitspur
├── score/<datei>      Pflicht: die Notendatei, wie sie importiert wurde (Guitar Pro, MIDI, MusicXML)
├── alignment.json     optional: wo die Takte in der Begleitspur liegen (Beat-Map)
└── audio/<datei>      optional: Begleitspur, nur wenn beim Export ausdrücklich gewählt (höchstens 256 MiB)
```

Die Notendatei liegt unverändert im Paket: Kontra liest sie beim Öffnen genauso wie beim ersten Import,
deshalb sieht das Stück auf dem zweiten Rechner gleich aus.

## `manifest.json`

```json
{
  "format": "kontra-package",
  "version": 2,
  "title": "Etude 1",
  "artist": "",
  "score": "etude-1.mid",
  "track": 0,
  "loops": [{ "from": 0, "to": 1 }, { "from": 4, "to": 7 }],
  "target_speed": 0.8,
  "backing": { "kind": "embedded", "file": "song.mp3", "offset_s": 0.5 }
}
```

| Feld | Pflicht | Bedeutung |
|---|---|---|
| `format` | ja | immer `kontra-package` |
| `version` | ja | Formatversion; Kontra lehnt höhere Versionen als die eigene ab |
| `title`, `artist` | `title` ja | Anzeige |
| `score` | ja | Dateiname unter `score/` |
| `track` | nein (0) | gespielte Spur der Notendatei, ab 0 |
| `loops` | nein (keine) | Loops zum Üben: Takte `from` bis `to`, Indizes ab 0 nach dem Ausrollen der Wiederholungen |
| `target_speed` | nein (1.0) | Zieltempo der Werkstatt, 1.0 = wie notiert, 0.25 bis 1.5 in 5-%-Schritten |
| `backing` | nein | Begleitspur, siehe unten |

`backing` ist eines von:

- `{ "kind": "embedded", "file": "<name>", "offset_s": <s> }`: Datei liegt unter `audio/<name>` (WAV, FLAC, Ogg Vorbis, MP3).
- `{ "kind": "linked", "path": "<pfad>", "offset_s": <s> }`: nur der Pfad auf dem Rechner des Absenders; fehlt die Datei, spielt Kontra ohne Begleitspur und sagt das.

`offset_s`: Sekunden in der Audiodatei, an denen Takt 1 beginnt.

Dateinamen (`score`, `file`) sind reine Namen: ohne `/`, `\`, `..` und nicht leer. Pakete mit anderen Namen
lehnt Kontra ab.

## `alignment.json`

Die Beat-Map der Begleitspur wie in Kontras Bibliothek, nur zusammen mit `backing`. Damit liegen die Takte beim
Empfänger so auf der Aufnahme wie beim Absender, ohne neuen Abgleich:

```json
{ "beats": [0.25, 0.75, 1.25], "bpm": 120.0, "first": 0, "shift_ms": 15.0, "nudges": { "3": -20.0 } }
```

`beats`: Schlagzeiten in der Audiodatei (s); `first`: Index des ersten Schlags von Takt 1; `shift_ms`: Verschiebung
aller Takte; `nudges`: Verschiebung einzelner Takte (Takt-Index → ms).

## Öffnen

Kontra entpackt ein Paket nach `<Datenordner>/packages/<Paketname>/` (Notendatei und eingebettete Begleitspur) und
nimmt das Stück mit Loops, Zieltempo und Abgleich in die Bibliothek auf. Der eigene Fortschritt bleibt unberührt;
für ein neues Stück beginnt er bei null. Öffnen über die Bibliothek („Öffnen …“), per Dateimanager oder `kontra lied.kontra`.

## Versionen

- **1**: erste Fassung, mit `progress.json` (Fortschritt des Absenders).
- **2** (Runde 15): Loops, Zieltempo und Abgleich der Begleitspur; kein Fortschritt mehr.

Neue Felder bekommen einen Standardwert, damit ältere Pakete lesbar bleiben; eine inkompatible Änderung erhöht `version`.

# Datensicherung (`kontra-data`, Format 1)

Einstellungen → „Deine Daten“ sichert alles in eine JSON-Datei (LH-FOR-03):

```json
{ "format": "kontra-data", "version": 1, "exported_unix": 1791504000,
  "settings": { … wie settings.json … },
  "library":  { "recent": [ … ], "setlists": [ … ] },
  "progress": [ { … wie progress.json oben … } ] }
```

Laden ergänzt (eigene Einstellungen bleiben; unbekannte Stücke nach Pfad, Setlisten nach Name und
Durchläufe kommen dazu) oder ersetzt alles. Ungültige Dateien ändern nichts.

# Kurspaket `.kontrakurs` (Format 1)

Ein Kurs eines Anbieters zum Hinzufügen (LH-LRN-14): ein Zip, entpackt der Ordner, den der Autor bearbeitet.
MIME-Typ: `application/x-kontrakurs`; Kontra öffnet ihn mit der Vorschau, nichts wird ohne Bestätigung hinzugefügt.
Lektionsformat wie die mitgelieferten Kurse (`lessons/README.md`).

```text
kurs.kontrakurs
├── manifest.json            Format, Version, Präfix, Titel, Anbieter, Lizenz, Kurs, Stream-Hosts
├── courses/<kurs>.json
├── units/<id>/de.md, en.md
├── glossary/de.md, en.md    optional: weitere Begriffe
└── media/                   optional: Videos (.webm, .mp4), Untertitel (.vtt)
```

```json
{ "format": "kontra-course", "version": 1, "revision": 3, "id": "studio-weber",
  "title": { "de": "Groove-Kurs", "en": "Groove course" },
  "provider": "Studio Weber", "license": "CC-BY-4.0",
  "course": "studio-weber.kurs", "hosts": ["video.example.org"] }
```

- `version` ist die Version dieses Formats, `revision` die Fassung des Kurses: Der Autor zählt sie mit jeder
  Veröffentlichung hoch, und ein Update sagt damit, ob es neuer ist als der vorhandene Kurs.
- `id` (a–z, `-`) ist das Präfix des Kurses und jeder eigenen Lektion (`studio-weber.anschlag`): Lektionen aus dem
  mitgelieferten Vorrat dürfen genannt (Kapitel, `needs`), aber nicht ersetzt werden. Begriffe im Glossar dürfen
  mitgelieferte Ids nutzen; das Glossar zeigt dann alle Texte nacheinander, jeden mit seiner Herkunft.
- `::: video src=… subtitles=…`: `src` ist `media/<datei>` im Paket oder `https://<host>/…` auf einem Host aus
  `hosts`; andere Adressen sind ein Fehler. `subtitles` je Sprachdatei (WebVTT) immer unter `media/`, damit sie
  ohne Netz gehen. Ein Video startet erst auf Klick; eins aus dem Paket entpackt Kontra dann in seinen Cache.
- Namen bleiben im Paket (kein `/` am Anfang, kein `..`, kein `\`), höchstens 1 GiB und 10 000 Einträge,
  Textdateien je höchstens 4 MiB, zusammen höchstens 32 MiB. Versteckte Dateien (`.…`) und `__MACOSX/` zählen
  nicht.
- Ein Kurs mit höherer `version` sagt, dass er ein neueres Kontra braucht.

Anlegen, prüfen und packen: `kontra-cli course new <ordner> --id <präfix> --provider … --license …`,
`kontra-cli course check <ordner|datei>` (jeder Befund mit Datei, Fehler im Format mit Zeile),
`kontra-cli course pack <ordner> <datei>.kontrakurs` (prüft vorher).

Hinzufügen: Lernen → „Kurs hinzufügen …“, `kontra kurs.kontrakurs` oder „Kurs von Adresse …“ (nur `https://`;
Kontra lädt nur die eingegebene Adresse). Kontra zeigt Titel, Anbieter, Lizenz und
Stream-Hosts, sagt, wenn der Kurs für ein anderes Instrument ist, und nimmt nur Pakete ohne Befund. Es legt die Datei
unverändert unter `<Datenordner>/courses/<id>.kontrakurs` ab; ein Paket mit derselben `id` ersetzt das alte, der
Lernstand bleibt. Mehrere Kurse für das Instrument: Auswahl „Kurs“ auf der Lernseite.
