# Songs aus dem Netz: Songsterr und MuseScore

Was Kontra mit Songsterr und MuseScore.com tun darf. Stand: 14.10.2026, geprüft an den Nutzungsbedingungen,
`robots.txt` und `ai.txt` der Seiten. Keine Rechtsberatung; bei Änderungen der Bedingungen neu prüfen.

## Ergebnis

| | Songsterr | MuseScore.com |
|---|---|---|
| Öffentliche, dokumentierte API | keine | keine (alte REST-API mit OAuth-Schlüssel antwortet 401, Entwicklerseite nicht erreichbar) |
| Suche aus Kontra heraus | nur undokumentiert über `/api/` (in `robots.txt` gesperrt) | Suchseiten `/sheetmusic?` und `/search/` in `robots.txt` gesperrt; Bot-Schutz |
| Herunterladen | nur mit Songsterr Plus, im Browser | nur mit PRO (20 am Tag, 300 im Monat), im Browser, nur privat |
| Automatischer Abruf | „Plus“-Funktionen nicht automatisieren; unabhängige Werkzeuge, die die Seite nicht automatisieren, sind erlaubt | „automated systems … for accessing or downloading Content“ verboten |

**Kontra baut deshalb:**

1. **Songsterr-Suche in Kontra** (Entscheidung des Eigentümers, Runde 12: „Die Suche selbst aus Kontra heraus ist
   okay. Ein Klick darauf öffnet weiterhin die Seite.“): siehe unten.
   **Suche im Browser:** Kontra öffnet außerdem die Suchseite von Songsterr oder MuseScore mit dem eingegebenen
   Titel im Browser des Nutzers (Ausweg ohne Netz in Kontra, und für MuseScore der einzige Weg).
2. **Neu in Downloads:** Was der Nutzer dort mit seinem Konto selbst herunterlädt (`.gp`, `.gp5`, `.mid`, `.mscz`,
   `.mxl`, …), zeigt die Bibliothek als „Neu in Downloads“ an; ein Klick öffnet es. Dazu Ziehen und Ablegen auf die
   Bibliothek.
3. **MuseScore-Format:** `.mscz`/`.mscx` direkt importieren (LH-BIB-10), ohne Export nach MusicXML. Das ist ein
   Dateiformat, keine Website; hilft P3 und P4, die in MuseScore schreiben.

**Nicht gebaut:** Scraping, Download von Tabs durch Kontra, andere Endpunkte als die Suche, MuseScore-Suche in
Kontra, Umgehen von Anmeldung oder Bezahlschranke.

## Songsterr-Suche in Kontra (Runde 12)

`/api/songs?pattern=…&size=20` ist undokumentiert und per `robots.txt` (`/api/`) für Crawler gesperrt; Kontra ist
kein Crawler, sondern fragt im Auftrag des Nutzers (wie `ai.txt` es erlaubt: Titel, Interpret, Stimmung ansehen).
Der Eigentümer hat das Risiko entschieden. Grenzen, die Kontra einhält:

- Nur auf Wunsch des Nutzers (Enter oder „Suchen“), eine Anfrage zur Zeit, kein Wiederholen, kein Vorladen, kein
  Abfragen im Hintergrund; 8 s Zeitlimit, Antwort höchstens 2 MB.
- User-Agent `Kontra/<Version> (bass trainer; +<Repository>)`, damit Songsterr weiß, wer fragt.
- Gezeigt werden Titel, Interpret, ob es eine Bassspur gibt und deren Stimmung; nichts wird gespeichert.
- Ein Treffer öffnet die Songsterr-Seite (auf der Bassspur) im Browser; Noten holt Kontra nie.
- Antwortet Songsterr mit 429 oder ändert das Format, sagt Kontra das und verweist auf die Browser-Suche. Verbietet
  Songsterr es ausdrücklich, fliegt die Suche raus.
- Tests, CI und Screenshots nutzen eine Antwort aus einer Datei (`--songsterr-answer`), nie das Netz.

## Songsterr

- Nutzungsbedingungen (Stand 10.6.2026): <https://www.songsterr.com/a/wa/terms>
  - Inhalte: kein Verbreiten, Ändern, Wiederverwenden, Herunterladen oder Kopieren „for commercial purposes or for
    personal gain, without express advance written permission“.
  - Plus: Download (Guitar Pro, MIDI, MP3, WAV) ist eine Plus-Funktion. Kein „automated tool, script, browser
    extension, or AI agent“, um Plus-Funktionen ohne Abo zu bekommen. „Independent tools that do not modify,
    automate, or interfere with the Service or its tab player“ sind davon nicht betroffen.
  - Apps nicht nachbauen oder zurückentwickeln; Aussehen der Seite nicht kopieren.
- Regeln für Agenten: <https://www.songsterr.com/ai.txt>. Im Auftrag eines Nutzers erlaubt: Tab öffnen und ansehen
  (Titel, Interpret, Stimmung, Noten), abspielen; unabhängige Werkzeuge bauen.
- `robots.txt`: <https://www.songsterr.com/robots.txt> sperrt `/api/`; die Suche der Startseite (`/?pattern=`)
  ist erlaubt (außer mit bestimmten Klammerzeichen).
- Rechte: Tabs bekannter Songs sind Bearbeitungen geschützter Werke; Songsterr zahlt nach eigener Aussage
  Lizenzen an Verlage (früher über Musicnotes,
  <https://m.musicnotes.com/PressMaterials/Musicnotes%20Nears%20$60%20Million%20in%20Royalties%20Paid;%20Celebrates%205%20Millionth%20Customer.pdf>).
  Ein Download über Plus ist für den Nutzer lizenziert, nicht zum Weitergeben.

## MuseScore.com

- Nutzungsbedingungen (Stand 27.9.2025): <https://musescore.com/legal/terms>
  - Abschnitt 21: Downloads und Ausdrucke nur „for personal, non-commercial, use“; nicht an Dritte weitergeben;
    keine „automated systems (except for those that may be provided by MuseScore) for accessing or downloading
    Content“. PRO/PRO+: höchstens 20 Downloads am Tag, 300 im Monat; manche Noten gar nicht.
  - Abschnitt 7: Hochgeladene Noten bleiben beim Urheber; MuseScore bekommt eine Lizenz zur Nutzung im Dienst, nicht
    zur Weitergabe außerhalb.
- `robots.txt`: <https://musescore.com/robots.txt> sperrt `/sheetmusic?`, `/search/`, `/score/*/download/*`,
  `/api/jmuse` (Audio, MIDI, Notenbilder).
- Download-Werkzeuge Dritter (z. B. LibreScore) umgehen die PRO-Sperre; MuseScore ließ eines 2021 löschen:
  <https://mjtsai.com/blog/2021/08/02/musescore-requests-downloader-takedown/>. Kontra baut so etwas nicht.
- Das Dateiformat `.mscz` (Zip mit `.mscx`-XML) ist offen dokumentiert durch den Quelltext von MuseScore (GPL);
  Kontra liest es mit eigenem Code, ohne MuseScore-Code zu übernehmen.

## Für den Nutzer

Kontra nennt in der Suche, dass sie im Browser öffnet und dass Downloads dort ein Konto oder Abo brauchen. Was er
herunterlädt, ist zum eigenen Üben lizenziert, nicht zum Weitergeben (auch nicht als `.kontra`-Paket).
