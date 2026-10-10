# Kontra-Paket `.kontra` (Format 1)

Offenes Paketformat zum Weitergeben eines Stücks mit Übungsstand (LH-BIB-07, Journey J-D2).
Ein `.kontra` ist ein gewöhnliches Zip-Archiv (Deflate oder unkomprimiert, kein Zip64, Namen in UTF-8);
jedes Zip-Programm kann es öffnen. MIME-Typ: `application/x-kontra-package`.

```text
lied.kontra
├── manifest.json      Pflicht: Format, Version, Titel, Notendatei, Spur, Begleitspur
├── score/<datei>      Pflicht: die Notendatei, wie sie importiert wurde (Guitar Pro, MIDI, MusicXML)
├── progress.json      optional: Übungsstand
└── audio/<datei>      optional: Begleitspur, nur wenn beim Export ausdrücklich gewählt
```

Die Notendatei liegt unverändert im Paket: Kontra liest sie beim Öffnen genauso wie beim ersten Import,
deshalb sieht das Stück auf dem zweiten Rechner gleich aus.

## `manifest.json`

```json
{
  "format": "kontra-package",
  "version": 1,
  "title": "Etude 1",
  "artist": "",
  "score": "etude-1.mid",
  "track": 0,
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
| `backing` | nein | Begleitspur, siehe unten |

`backing` ist eines von:

- `{ "kind": "embedded", "file": "<name>", "offset_s": <s> }`: Datei liegt unter `audio/<name>` (WAV, FLAC, Ogg Vorbis, MP3).
- `{ "kind": "linked", "path": "<pfad>", "offset_s": <s> }`: nur der Pfad auf dem Rechner des Absenders; fehlt die Datei, spielt Kontra ohne Begleitspur und sagt das.

`offset_s`: Sekunden in der Audiodatei, an denen Takt 1 beginnt.

Dateinamen (`score`, `file`) sind reine Namen: ohne `/`, `\`, `..` und nicht leer. Pakete mit anderen Namen
lehnt Kontra ab.

## `progress.json`

Der Fortschritt des Stücks wie in Kontras Datenordner (`progress/<id>.json`):

```json
{
  "song_id": "1f3c…",
  "title": "Etude 1",
  "artist": "",
  "runs": [
    { "finished_unix": 1791504000, "practice_s": 600.0, "speed": 0.8, "hit_rate": 0.92,
      "mode": "workshop", "section": [2.0, 4.0], "hardest_bars": [3, 7] }
  ]
}
```

Beim Öffnen werden die Durchläufe zum eigenen Fortschritt des Stücks hinzugefügt; ein Durchlauf mit
gleicher Endzeit und Dauer wird nicht doppelt gezählt. `song_id` wird beim Öffnen neu berechnet.

## Öffnen

Kontra entpackt ein Paket nach `<Datenordner>/packages/<Paketname>/` und nimmt das Stück in die Bibliothek
auf. Öffnen über die Bibliothek („Öffnen …“), per Dateimanager oder `kontra lied.kontra`.

## Versionen

- **1** (Kontra 0.1): erste Fassung.

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
