# Kontra: QA-Journeys

Feste User Journeys für explorative QA und für automatisierte Journey-Tests. Jede Journey gehört zu einer oder mehreren Personas aus [`PERSONAS.md`](PERSONAS.md)
und nennt die Anforderungen aus dem [Lastenheft](LASTENHEFT.md), die sie prüft. Wer etwas an Kontra ändert, spielt die betroffenen Journeys durch (siehe [`agent.md`](../agent.md)).

## Ablauf einer Sitzung

1. **Aufbau.** Qt-Umgebung einmal einrichten (`sudo scripts/setup-qt-env.sh`), dann `. scripts/qt-env.sh` und
   `cargo build -p kontra-app -p kontra-cli`. Frisches Profil: `KONTRA_DATA_DIR=/tmp/qa-<journey>`.
   Nach Änderungen an `crates/app/src/bridge.rs` vorher `cargo clean -p kontra-app` (siehe `agent.md`, Gelernte Fehler).
2. **Eingang.** Mit echtem Instrument oder simuliert:
   - Aufnahme erzeugen: `kontra-cli render testdata/demo/etude-1.mid take.wav --lead-in 2.4 [--offset-ms 20] [--wrong-every 7] [--miss-every 13] [--detune-cents 15]`
     (`--lead-in` = Länge des Einzählers, bei 100 BPM ein 4/4-Takt = 2,4 s).
   - App mit Aufnahme statt Audio-Eingang: `kontra --open <datei> --input-file take.wav [--input-profile laptop-jack] [--autoplay] [--mode song|wait|workshop] [--view lane|tab|notation]`.
   - Ohne GUI: `kontra-cli practice <datei> --input-file take.wav --mode song` und `kontra-cli score <datei> take.wav`.
   - Ohne Audioausgang (Container, CI) läuft Kontra stumm weiter und sagt das.
3. **Als Persona spielen.** Vor dem Start die Persona lesen und ihre Ausrüstung nachstellen (Auflösung, Skalierung
   `QT_SCALE_FACTOR=2`, Sprache `--language de|en`, Stil `--theme kante|kante-light|system` (nicht `--style`, das nimmt Qt), Eingang).
   Entscheidungen so treffen, wie sie es täte, nicht wie ein Entwickler.
4. **Belege:** `--screenshot out.png --screenshot-after <ms>` speichert die aktuelle Seite; `scripts/screenshots.sh` macht alle Seiten auf Deutsch und Englisch.
   Bedienen ohne Koordinaten: `scripts/qa-session.sh --a11y … --run "scripts/qactl.py press --role 'list item' Einstellungen"`
   (`tree`, `find`, `press`, `text`, `wait`, `layout`); `scripts/check-layout.sh` prüft jede Seite bei P4 und P7.
   Nebenher beobachten: Konsole (QML-Warnungen, `audio problem`), rohe Übersetzungsschlüssel (`level.quiet`), Sprachmix, abgeschnittene Texte,
   Farbe als einziges Signal, Fokus nicht sichtbar.
5. **Fund notieren** mit: Stufe (Fehler, Reibung, unlogisch, kosmetisch), Persona, Journey und Schritt, was passierte gegen was erwartet war,
   Beleg (Screenshot, Log-Zeile, Aufnahme). Nur Beobachtetes; Ungetestetes ausdrücklich nennen.
6. **Ergebnis** der Runde in `ROADMAP.md` unter „QA-Runde <Datum>“; gefundene Fehler: zuerst ein Test, der fehlschlägt, dann die Korrektur.
7. **Automatisierung.** Journeys mit messbaren Schritten (markiert mit ⚙) haben einen Test in `tests/journeys/` (ein Modul je Journey,
   `cargo test -p kontra-appcore --test journeys`, läuft in CI): Aufnahmen und Testmaterial (etwa `stimmen-e-30c.wav`) erzeugt der
   Test selbst, die App-Logik spielt sie in Echtzeit ab, ohne Soundkarte. Screenshots für Release-Logs kommen aus `scripts/screenshots.sh`.

Markierungen: ⚙ automatisiert · 🔍 auch mit 200 % Skalierung und Farbfilter Deuteranopie (P7) · 📽 auch bei 1280×720 (P4).

---

## A. Erster Start und Einrichtung

**J-A1 Anstecken und los** ⚙ 🔍 · Persona: P1 Lea · Phase 1 · LH-EIN-01, -02, -03, -04, LH-NF-01
1. Kontra zum ersten Mal starten, Bass per Klinke in die Laptop-Buchse (`--input-profile laptop-jack`).
2. Assistent: Instrument „E-Bass, 4 Saiten“, Stimmung „Standard“; Anschluss „Klinke direkt in den Laptop“ (daneben Interface und Mikrofon; MIDI kommt mit dem Instrument „Bass über MIDI“); Eingang wählen (vorgewählt ist die eingebaute Karte; beim Anschluss „Interface“ ein Interface). Pegel ist zu leise.
3. Den Hinweis lesen und befolgen (Lautstärke am Bass auf, Gain im Assistenten).
4. Kalibrierung überspringen.
5. Am Ende „Wie spielst du bisher?“: „Neu am Bass“ öffnet die erste Lektion und schaltet die einfache Ansicht ein; „Ich spiele schon“ (P3 Mira) öffnet die Übungen mit den Grundlagen; „Eigene Songs“ (P2 Jens) die Bibliothek. Ersten Ton spielen.
Erwartet: ≤ 60 s bis zum ersten bewerteten Ton; die Übungsseite zeigt Lea nur Modus, Ansicht, Tempo und das Ziel der Übung; der Hinweis zur Laptop-Buchse erklärt in einem Satz, warum es dumpf klingt, und verweigert nichts; kein Fachbegriff ohne Erklärung.

**J-A2 Stimmen ohne Vorwissen** ⚙ 🔍 · Persona: P1 Lea · Phase 1 · LH-TUN-01, -02, -03, -04
1. Stimmgerät öffnen, „Geführt“.
2. E-Saite 30 Cent zu tief anschlagen (`stimmen-e-30c.wav`, erzeugt der Journey-Test), Anweisung befolgen.
3. Alle vier Saiten durchgehen.
4. Einstellungen: Stimmung „Drop D“, dann „Eigene“ (oberste Saite F), Kammerton 442 Hz; Stimmgerät erneut öffnen, eine MIDI-Datei öffnen.
Erwartet: Anweisung in Worten („etwas höher drehen“) und Symbol; Erfolg pro Saite eindeutig, auch ohne Farbe. Nach Schritt 4 führt das Stimmgerät durch D A D F, Cent beziehen sich auf 442 Hz, die Tab liegt auf den neuen Saiten; alles bleibt nach einem Neustart.

**J-A3 Latenz kalibrieren** ⚙ · Personas: P3 Mira, P2 Jens, P1 Lea, P7 Sam · Phase 1 · LH-EIN-05, -07, -14, LH-BEW-08
1. Einstellungen → Eingang → Latenz, „Auf den Klick spielen“.
2. Nach vier Klicks Vorzähler acht Anschläge mit +40 ms Versatz einspeisen.
3. Modus „Durchlauf“, pünktlich gespielte Aufnahme einspeisen.
4. PipeWire-Quantum auf 2048 setzen, Kontra neu starten.
Erwartet: gemessene Latenz ≈ 40 ms angezeigt, daneben „auf den Klick gemessen, enthält deine Neigung“; danach Bewertung pünktlich (±5 ms); bei Quantum 2048 Hinweis mit Wert und Anleitung. Während der Messung zählt die Seite mit („Vorzähler 3“, „Schlag 4 von 8“).
5. (P2 Jens, Interface) „Mit Kabel messen“: Kabel vom Kopfhörer-Ausgang in den Eingang; die Töne kommen 20 ms später zurück.
6. Bass wieder einstecken, „Auf den Klick spielen“, jeder Anschlag 15 ms vor dem Klick.
7. Ziel „Auf den Punkt“, ein Lied mit derselben Neigung; dann Ziel „Frei“.
8. Ohne Kabel „Mit Kabel messen“ (es kommt nichts zurück); Anschluss „Klinke direkt in den Laptop“ (P1 Lea).
Erwartet: nach 5 „20 ms“ und „mit Kabel gemessen“; nach 6 bleibt die Latenz 20 ms, der Text sagt „Deine Neigung: −15 ms, du spielst vor dem Klick“, neben dem Wert „deine Neigung: −15 ms“. In 7 „Ziel auf den Punkt: verfehlt“ (nicht „gehalten“), nie „Neu kalibrieren“; mit „Frei“ die Neigung als Beobachtung. In 8 „Nur 0 von 12 Tönen kamen zurück … Ohne Kabel: auf den Klick spielen“, die Latenz bleibt; an der Laptop-Buchse und bei MIDI gibt es den Kabel-Knopf nicht, ein Satz sagt warum. Ein anderer Ein- oder Ausgang verwirft Kabel- und Klick-Wert (die Latenz bleibt als Startwert, ohne Herkunft), auch wenn ihn die automatische Wahl ändert (Interface abgesteckt, Kontra neu gestartet). Screenreader (P7) liest Wert, Herkunft und Neigung als Text, nichts nur über Farbe.

**J-A4 Eingang, den es noch nicht gibt** · Personas: P5 Aylin, P6 Karl · Phase 1 bis 3 · LH-EIN-09, -10, -12
1. MIDI-Keyboard anstecken, Kontra starten, Einrichtung bzw. Einstellungen → Instrument „Bass über MIDI“ (P5). Danach Instrument „Kontrabass“ mit USB-Mikrofon (P6).
Erwartet: Das Instrument ist eine eigene Wahl (E-Bass, Kontrabass, Bass über MIDI), unabhängig vom Anschluss gespeichert. „Bass über MIDI“ setzt den Anschluss „MIDI-Instrument“, der das erkannte Keyboard nennt (ohne Keyboard: „gerade ist keins angeschlossen“); Saiten, Stimmung, Bundlos und Kammerton sind ausgeblendet. „Kontrabass“ schlägt das Mikrofon vor (Tonabnehmer über das Interface bleibt wählbar), bietet 4 oder 5 Saiten, kein „Bundlos“. Kein Absturz, keine leere Liste, kein „Gerät defekt“.

**J-A5 Interface neben anderem Standard** · Personas: P2 Jens, P3 Mira · Phase 1 · LH-EIN-02, -18, LH-NF-01
1. Systemstandard für Ein- und Ausgabe ist die interne Karte (Studiomonitore), das Interface hängt am USB.
2. Einstellungen → Eingang: Liste ansehen, Interface wählen; Kontra neu starten.
3. `kontra-cli devices`; ohne PipeWire (`KONTRA_AUDIO_HOST=alsa`) dasselbe.
4. Interface ohne PipeWire-Eingang (Zustand nach „device busy“: Karte in `/proc/asound/cards`, kein Quellknoten).
5. Direktmonitor am Interface aus, in der Seitenleiste „Bass hören“ an, fünf Minuten nichts tun, dann spielen; Stimmgerät öffnen (der Bass ist dort ebenfalls zu hören) und schließen, wieder spielen.
6. Interface abziehen, Eingang auf „Automatisch“, „Bass hören“ aus und wieder an; danach einen Mixer-Regler bewegen.
7. Ein USB-Kabel ohne Ausgang (Gitarrenkabel oder Webcam) einstecken, während Kontra läuft; einmal „Instrument“, einmal „Mikrofon“ antworten, abziehen, wieder einstecken, Kontra neu starten; danach in den Einstellungen die Zeile „USB-Eingang <Name>“ (steckt das Gerät) von Instrument auf Mikrofon und zurück stellen, auch mit Tab und den Pfeiltasten (ohne Antwort führt Pfeil rechts zu „Instrument“).
Erwartet: Das Interface steht oben mit seinem Namen („Scarlett Solo USB Analoges Stereo“), ist bei „Automatisch“
vorgewählt und bleibt nach dem Neustart gewählt; keine Monitore („Monitor of …“), keine Streams anderer Programme in der
Eingangsliste; Block 128 Frames (`kontra-cli check --steps loopback`). Mit ALSA gilt die alte Liste weiter. In
Schritt 4 bleibt das Interface wählbar und vorgewählt (über ALSA, in `devices` markiert); beim Start des Eingangs
nennt Kontra die Abhilfe (ab- und anstecken oder `systemctl --user restart wireplumber`). In Schritt 5 ist der Bass auf beiden Seiten zu hören, auch nach dem Leerlauf (das Interface bleibt wach), im Stimmgerät und nach dem Stimmgerät wieder, ohne Lücke beim Wechsel (Runde 20). In Schritt 6 sagt Kontra beim Einschalten, dass es keinen Eingang hört, und nennt die Einstellungen; der Regler holt die Meldung nicht zurück, und das Laptop-Mikrofon wird nicht geöffnet. In Schritt 7 fragt Kontra einmal, ob ein Instrument oder ein Mikrofon am Gerät hängt (beide Knöpfe per Tab erreichbar); „Instrument“ macht es zur Vorwahl bei „Automatisch“, aber nur, wenn kein anderes Interface steckt (ein Scarlett steht davor), „Mikrofon“ nicht; das Gerät fragt danach nie wieder, auch nicht nach dem Neustart. Die Einstellungen zeigen für das Gerät eine Zeile mit der Wahl (ohne Antwort keine Seite gewählt); die Wahl wirkt sofort (bei „Automatisch“ springt der Eingang auf das Gerät, wenn es „Instrument“ ist und kein anderes Interface steckt), eine offene Frage zu dem Gerät verschwindet, und die Wahl bleibt nach dem Neustart.

**J-A6 Windows: installieren und erster Ton** · Persona: P8 Noah · nach Phase 1 (Win5, Win6) · LH-NF-17
1. Windows 11 ohne Adminkonto, Benutzerordner mit Leerzeichen und Umlaut, Skalierung 150 %; Interface mit ASIO-Treiber am USB.
2. Installer (`Kontra-<Version>-windows-setup.exe`) von der Landing Page laden und starten (Installation für den Benutzer, ohne Adminabfrage); Kontra aus dem Startmenü öffnen. Dazu einmal das portable ZIP entpacken und `bin\kontra.exe` starten.
3. Einrichtung: Interface wählen (ASIO, danach WASAPI), Pegel, Stimmgerät, Latenz „mit Kabel messen“.
4. Mitgelieferte Übung durchspielen; Sicherung von einem Linux-Rechner laden; Kontra schließen und wieder öffnen.
5. Mit einer älteren Version als der in `versions.json` (oder `--update-answer`): die Startseite zeigt den Hinweis mit Link; „×“ blendet ihn aus, der Schalter „Nach neuen Versionen suchen“ in den Einstellungen schaltet die Anfrage ab. Ohne Netz kein Hinweis und keine Verzögerung.
6. Kontra entfernen (Einstellungen > Apps oder Startmenü).
Erwartet: keine Warnung von SmartScreen (signiert), keine Frage nach Adminrechten; das Interface steht mit seinem
Windows-Namen in der Liste; Latenz mit ASIO ≤ 20 ms (LH-NF-01), mit WASAPI gemessen und genannt; Daten in
`%APPDATA%\Kontra`, nach dem Neustart alles wie verlassen; die Sicherung bringt Stücke und Fortschritt; nach dem
Entfernen bleibt nur der Datenordner; der Hinweis erscheint nur bei einer neueren Version. Bis Win5: in der KVM-Maschine des Eigentümers mit dem Paket aus der CI.

## B. Spielen und Üben

**J-B1 Lied durchspielen** ⚙ 📽 · Persona: P2 Jens · Phase 1 · LH-BIB-01, -04, LH-MOD-01, LH-WIE-03, LH-EIN-08, -17, -19, -20, -21, LH-ANZ-01, -02
1. Ordner mit vier Guitar-Pro-Dateien über „Stücke hinzufügen ▾“ → „Ordner hinzufügen …“ → „Als Setliste“ importieren (eine davon defekt).
2. Setlist „Freitag“ anlegen, Lieder über „Stück hinzufügen …“ aufnehmen (ohne sie zu öffnen), erstes Lied, Bassspur wählen.
3. Modus „Frei spielen“, Backing Track (synthetisch bzw. ab Phase 3 eigene MP3), durchspielen; im Mixer (X) Backing leiser, Metronom lauter, Mithören an, Tiefen +4 dB, Kompressor und Wärme halb, unter „Effekte“ über „Effekt hinzufügen …“ Overdrive, Delay und Stereobreite hinzufügen (ein Dialog; Escape schließt ihn ohne Änderung, „Hinzufügen“ oder Eingabe fügt hinzu und schließt ihn), Delay mit „Hoch“ an die erste Stelle, Overdrive aus und wieder an, den Drive-Regler und den Anteil des Delays bewegen, Stereobreite entfernen, „Plugins suchen“ (während der Suche stehen Balken und „Prüfe n von m: Name. Gefunden: k.“ dabei, danach das Ergebnis, kaputte Dateien mit Grund), ein gefundenes CLAP- oder VST3-Plugin hinzufügen (es steht unter dem Chip seiner Art, etwa „Dynamik“, und per Suche nach Name oder Hersteller; Zeile wählen mit Pfeiltasten, „Hinzufügen“ oder Eingabe; die Zeile sagt „… wird geladen“, bis es in der Kette steht; ein zweites „Hinzufügen“ in der Zeit nennt das laufende), dessen Regler bewegen, es aus und wieder an schalten und verschieben, „Fenster“ öffnen (ein CLAP- oder VST-3-Plugin mit eigener Oberfläche: sein Fenster erscheint; ohne: ein Hinweis in Worten), darin einen Regler drehen, Fenster schließen, die Kette als „Probe“ speichern, bis zur vollen Kette (8) weitere Effekte hinzufügen, Begrenzer aus und wieder an, „Neutral“, „Probe“ laden und wieder löschen, Mithören aus. „Griffbrett ausblenden“ (Lage, Techniken und Pegel stehen dann in einer Zeile über der Transportleiste, die Noten werden höher), Kontra neu starten (bleibt ausgeblendet), „Griffbrett zeigen“. Alles nur mit Tastatur (Tab, Leertaste, Pfeile).
4. Mitten im Lied von Tab-Lane auf Tabulatur umschalten.
5. Nächstes Lied der Setlist.
Erwartet: eine verständliche Meldung zur defekten Datei, drei Lieder importiert; kein Anhalten, keine Punkte; Mixer wirkt sofort ohne Neustart, Mithören nennt die Latenz; Klangregler und Effekte knacken nicht und ändern die Bewertung nicht; die Effektliste zeigt Reihenfolge, Schalter und Werte als Text und Zahl (nicht nur Farbe), der Fokus bleibt beim verschobenen Effekt, eine volle Kette sagt es in Worten und sperrt „Hinzufügen“, „Neutral“ leert die Liste, eine geladene Kette bringt Effekte, Werte und Plugins zurück, die Kette und die gespeicherten Ketten bleiben nach Neustart; ein Plugin-Eintrag nennt Name, Hersteller und Format, zeigt seine Parameter als Regler mit Zahl, behält Werte und Zustand nach Neustart, sein eigenes Fenster öffnet sich über „Fenster“ und was darin gedreht wird, steht danach auch im Mixer, und es sagt in Worten, wenn es nicht lädt (Datei fehlt): sein Eintrag bleibt dann stumm, der Rest der Kette klingt weiter; die Suche blockiert die Oberfläche nicht, ein abstürzendes Plugin gilt als kaputt. Automatisiert sind Suche, Hinzufügen, Regler, Speichern und der Pegel durch die Kette mit dem Test-Plugin in `crates/appcore` und `crates/engine` geprüft; Schritt 3 mit den eingebauten Effekten läuft in `tests/journeys/b1_song.rs`; Umschalten ohne Aussetzer; Liedwechsel in ≤ 2 Klicks.

**J-B2 Warten-Modus** ⚙ 🔍 · Persona: P1 Lea · Phase 1 · LH-MOD-03, LH-BEW-04
1. Übung „Seven-Nation-Riff“ (eigene Datei) im Modus Warten.
2. Drei falsche Töne, dann den richtigen.
3. Lange Pause (30 s), dann weiter.
Erwartet: Lied wartet ohne Zeitdruck; falscher Ton wird freundlich angezeigt (welcher Ton gespielt, welcher erwartet, in Worten und Symbol); nach der Pause kein Abbruch.

**J-B3 Refrain sitzt bis Freitag** ⚙ · Persona: P2 Jens · Phase 1 · LH-MOD-04, -05, -06, -07, LH-WIE-01, LH-BEW-03, LH-BEW-16, LH-TUN-05, LH-FOR-06
1. Modus „Durchlauf“, Lied 3 durchspielen, einige Noten auf einer anderen Saite als in der Tab, die A-Saite etwa 15 Cent zu hoch gestimmt; Auswertung zeigt schwierigste Takte.
2. Auf den schwierigsten Takt klicken → Riff-Werkstatt mit diesem Bereich.
3. Tempo 60 %, automatische Anpassung an das Zielband 80–90 % Treffer, Vorzähler an.
4. Fehlerfreie Durchläufe einspeisen bis 100 %.
5. Notendichte „50 %“ wählen (jeder Takt behält seine Schläge); nach einem sauberen Durchlauf (über 90 %) den Vorschlag „mehr Noten“ annehmen: 75 %.
6. Alles nur mit Tastatur wiederholen (Loop mit `[` und `]` setzen, Pos1 springt an den Loop-Anfang), dann mit einem Fußschalter (Bild ab/auf).
7. Liedkarte (in der Auswertung und angehalten über der Werkstatt): über Takt 5–8 ziehen; dann per Tastatur (Tab auf die
   Karte, Pfeile, Umschalt+Pfeile, Eingabe).
Erwartet: Die Liedkarte zeigt je Takt die Treffer der letzten Durchläufe als Höhe, der Tooltip und der Screenreader die
Zahl („Takt 5: 75 % im letzten Durchlauf (3 von 4 Noten)“), nie gespielte Takte gestrichelt statt null; beim Spielen
ist sie weg; Ziehen über Takt 5–8 setzt die Schleife 5–8 (aus der Auswertung in der Werkstatt). Die Auswertung nennt, wie viele Noten auf einer anderen Saite als in der Tab lagen (bei bundlosem oder 5-saitigem Profil „nicht unterstützt“), die Noten bleiben Treffer; sie fragt „Die A-Saite klingt etwa 15 Cent zu hoch – nachstimmen?“, „Zum Stimmgerät“ öffnet das Stimmgerät; Bereich und Tempo stimmen; Backing Track bei 60 % ohne metallischen Klang; Steigerung sichtbar; jede Funktion per Tastenkürzel; jeder Durchgang beginnt mit neutralen Noten (keine Marken des vorigen vor der Spiellinie).

**J-B4 Bundlos und tief** ⚙ · Persona: P3 Mira · Phase 1 · LH-TUN-01, LH-BEW-01, -02, -05, -12, LH-NF-03, LH-ANZ-03
1. Instrument „5 Saiten, bundlos“ anlegen.
2. Walking-Bass-Etüde in der Notenansicht, Zeitfenster „streng“.
3. Aufnahme mit absichtlichen Intonationsfehlern (±10 bis ±40 Cent) und tiefen Tönen auf der B-Saite einspeisen.
Erwartet: Cent-Werte pro Note weichen ≤ 3 Cent vom annotierten Wert ab; die Auswertung nennt die Intonation als Richtung und Streuung („2 unsauber · im Schnitt richtig, Streuung ±24 Cent“), keine Note dafür, und fragt nie nach dem Nachstimmen einer Saite; „Alle Noten“ nennt je Note das Timing in ms (auch „pünktlich (0 ms)“ und bei ausgelassenen Noten neben dem Fenster „40 ms zu früh, außerhalb des Zeitfensters“); B0 bis D1 sicher erkannt; Noten im Bassschlüssel korrekt.

**J-B5 Unterricht am Beamer** 📽 🔍 · Persona: P4 Thomas · Phase 1 (Präsentationsmodus Phase 2) · LH-ANZ-02, -04, -09, LH-ALL-02, LH-MOD-07
1. Auflösung 1280×720, Schrift 150 %.
2. Übung öffnen, kombinierte Ansicht Noten über Tab, Modus „Frei spielen“.
3. Stil auf Kante Light wechseln (heller Raum).
4. Mixer öffnen, dann Präsentation (F5 oder Knopf neben dem Mixer), durchspielen, mit Esc beenden.
Erwartet: nichts abgeschnitten, Lane und Noten aus 4 m lesbar, Kontrast ausreichend; der Mixer gleitet als Glasebene über die Noten, ohne sie zu verschieben, und geht mit F5 zu; in der Präsentation Vollbild ohne Seitenleiste, nur Titel, Noten und Transport, Notenlinien größer, in einer Neuauslegung ohne Zwischenstand (die Zeilenzahl darf sich dabei ändern); Esc stellt das Fenster wieder her wie vorher, der Mixer ist wieder offen; nach dem Durchspielen lässt „Fertig gespielt“ Noten und Tab lesbar stehen; in Kante Light die Ziffern der Tabulatur in jeder Saitenfarbe mindestens 4,5:1.

**J-B6 Song aus dem Netz** ⚙ 🔍 · Personas: P2 Jens, P3 Mira, P1 Lea, P7 Sam · Phase 2 · LH-BIB-10, -11
1. Jens: Bibliothek, „Stücke hinzufügen ▾“ → „Download …“, „Seven Nation Army“ eintippen, Enter (oder „Suchen“); einen Treffer mit Bassspur anklicken.
2. Im Browser die Guitar-Pro-Datei herunterladen (Songsterr Plus; ersatzweise eine eigene `.gp`), zurück zu Kontra.
3. Im selben Dialog unter „Neu in Downloads“ die Datei anklicken (der Dialog schließt, das Stück öffnet).
4. Mira: eine in MuseScore 4 gespeicherte `.mscz` (Fünfsaiter mit Tab) auf die Bibliothek ziehen; danach drei Dateien auf einmal, eine davon defekt.
5. Lea: dasselbe ohne Netz; dann ohne Browser (keine Standard-App für Links); „MuseScore im Browser“.
6. Sam (P7): alles nur mit Tastatur und Screenreader, 200 %.
Erwartet: eine Anfrage je Enter, „Suchen“ ist gesperrt, solange sie läuft; Treffer mit Titel, Interpret und „Bass: E A D G“ bzw. „ohne Bassspur“ als Text; ein Klick öffnet die Songsterr-Seite auf der Bassspur im Browser, Kontra lädt keine Tabs; die Datei erscheint beim Zurückkehren ins Fenster (sonst binnen 15 s) mit Art („Guitar Pro“, „MuseScore“) und Alter, ein Klick öffnet sie mit der Bassspur und nimmt sie aus „Neu“; `.mscz` mit gleichen Saiten und Bünden wie in MuseScore; mehrere abgelegte Dateien landen in der Bibliothek, die defekte mit Grund; ohne Netz nennt Kontra den Grund und verweist auf die Browser-Suche, die Bibliothek bleibt bedienbar; ohne Browser eine Warnung mit der Adresse zum Kopieren; Feld, Knöpfe und Treffer per Tab erreichbar, Enter öffnet einen Treffer, der Screenreader liest Titel, Interpret und Bassspur.

**J-B7 Doppelgriffe** · Personas: P3 Mira, P2 Jens, P5 Aylin, P6 Karl · Phase 4 · LH-BEW-11
1. Mira: Übung mit Quinten und Dezimen als Doppelgriff (Tab mit zwei Tönen je Anschlag), Modus „Durchlauf“; erst
   beide Töne sauber, dann bei jedem zweiten nur den unteren Ton, dann eine Oktave.
2. Jens: dieselbe Übung über das Laptop-Kabel bzw. die Aufnahme dazu (`--input-file`).
3. Aylin: dieselbe Übung am MIDI-Keyboard.
4. Karl: dieselbe Übung am Kontrabass über Mikrofon (Aufnahme `room-double`, `--input-file`).
Erwartet: beide Töne gespielt → beide getroffen, auch wenn die Tonhöhe des Gemischs ein anderer Ton war; nur der
untere → unterer getroffen, oberer ausgelassen, etwa 160 ms nach dem Anschlag, in der Auswertung unter „Alle Noten“ mit
„nur ein Ton des Griffs gehört“; Oktave → wie bisher (ein Ton zählt);
kein Doppelgriff wird als falscher Ton gewertet, nur weil zwei Töne klangen. MIDI hört jeden Ton selbst (als ganze
Menge erst mit M2). Mikrofon: kein Urteil über beide Töne, ein Ton zählt für beide; Einstellung „Mikrofon“ und
Auswertung sagen, dass Doppelgriffe über Mikrofon noch nicht so bewertet werden (Runde 17).

**J-B8 Abdämpfen** · Personas: P1 Lea, P3 Mira, P6 Karl · Phase 4 · LH-BEW-10
1. Lea (Laptop-Buchse): Lektion „Saiten dämpfen“, Übung „Groove Staccato auf einem Akkord in E“ (151), Modus
   „Durchlauf“; erst jede Note nach einer Achtel abdämpfen, dann im zweiten Durchgang die Töne durchklingen lassen
   (Aufnahme per `--input-file`, gerendert wie in `crates/session/tests/ends.rs`).
2. Mira: ein eigenes Stück mit Staccato-Punkten (MusicXML) über Kabel.
3. Karl: dieselbe Übung über Mikrofon.
Erwartet: Die Auswertung nennt „Abdämpfen: N von N Noten rechtzeitig gestoppt“ (alle außer der letzten) bzw. im
zweiten Durchgang „in die Pause geklungen: …“; unter
„Alle Noten“ steht je Note „rechtzeitig abgedämpft“, „… ms zu spät abgedämpft“ oder „klang in die Pause“; die
Trefferquote bleibt gleich (Abdämpfen ist Rückmeldung, kein Treffer); Noten ohne Pause danach, Akkorde, Dead Notes,
Palm-Mute und die letzte Note (ohne Staccato) werden nicht bewertet. Mikrofon: kein Urteil, die Auswertung sagt
„Abdämpfen bewertet Kontra über diesen Eingang noch nicht (nur über Kabel)“.

**J-B9 Bibliothek ordnen** (Daten und Zählung getestet in `crates/appcore/src/tests/library_view.rs`, Menü, Baum, Tabelle und Dialoge in `crates/app/qml/tests/tst_addpiecesmenu.qml`, `tst_librarytree.qml`, `tst_librarytable.qml`, `tst_librarydialogs.qml`, `tst_librarypage.qml`) · Personas: P2 Jens, P4 Thomas, P1 Lea, P7 Sam · Phase 3 · LH-BIB-17, -18, -19
1. Jens: 40 Stücke aus Downloads, einen Ordner „Band“ mit Unterordnern „Rock“ und „Slap“ über „Stücke hinzufügen ▾“ → „Ordner hinzufügen …“ verknüpfen (der Dialog fragt: Verknüpfen, Importieren, Als Setliste); im Baum „Heruntergeladen“, dann „Band“ und „Slap“ anklicken; nach „Zuletzt“ absteigend sortieren (Spaltenkopf, Pfeil im Titel), nach Tempo auf- und absteigend; über „Gruppieren“ nach Interpret gruppieren; „funk“ suchen.
2. Jens: eine Datei aus „Heruntergeladen“ in „Band/Rock“ verschieben, Bibliothek neu öffnen, das Stück spielen.
3. Thomas: einen Ordner auf einem langsamen oder nicht eingehängten Laufwerk verknüpfen, dann abziehen.
4. Lea: nur „Übungen“ und „Alle Stücke“; Suche nach einem Wort aus dem Titel.
5. Sam (P7): Baum, Spaltenköpfe, Gruppenüberschriften, „Stücke hinzufügen ▾“ (Pfeile, Enter, Esc) und die Zeilenmenüs „…“ nur mit Tastatur und Screenreader, 200 %; in schmalem Fenster steht die Quelle als Auswahlfeld statt des Baums.
6. Jens: „Stücke hinzufügen ▾“ → „Neues Stück generieren …“: Stil, Schwierigkeit, Tonart, Länge „Lang“, „Andere Variante“ zweimal (Variante 2), „Generieren“; das Stück öffnet, steht danach unter „Generiert“. Dieselben Einstellungen erzeugen dasselbe Stück.
7. Jens: eine Datei und einen Ordner aus dem Dateimanager auf die Bibliothek ziehen: die Fläche „Loslassen zum Hinzufügen“ erscheint, die Datei öffnet (mehrere gehen in die Bibliothek), der Ordner steht im Baum. „Download …“ öffnet die Suche im Netz als Dialog (J-B6), „Datei öffnen …“ den Dateidialog.
8. Lea: eine Setliste im Baum wählen: Stücke in Reihenfolge, verschieben, entfernen, hinzufügen, löschen (zweistufig); „+ Neue Setliste“ legt eine an und zeigt sie.
Erwartet: die Zahl am Baum stimmt mit der Zeilenzahl überein (Ordner zählt Unterordner mit); Sortierung hat Richtungsanzeige als Text oder Symbol, Stücke ohne Tempo oder Ergebnis stehen in beiden Richtungen am Ende; Gruppen haben Überschrift und Anzahl, „Ohne Interpret“ steht zuletzt; das verschobene Stück steht danach unter „Band/Rock“ und behält sein Ergebnis; ein langsames Laufwerk lässt das Fenster bedienbar, der Ordner füllt sich, wenn er gelesen ist; ein abgezogener Ordner steht als „fehlt“ im Baum, ohne Fehlerdialog, seine Stücke mit „fehlt“ in der Zeile; gewählt sagt er „Ordner nicht gefunden“ mit Pfad und wird neu gesucht; das „×“ am Ordner entfernt nur die Verknüpfung, keine Datei und kein Ergebnis; Herkunft steht als Zeichen und Wort (◆ Übung, ◇ Generiert, ↓ Download, ● Importiert, □ Ordner); Tabelle, Baum, Spaltenköpfe, Menü und Überschriften per Tab erreichbar, der Screenreader liest Sortierrichtung, Gruppe und je Zeile Herkunft, Tempo, Zuletzt und Ergebnis; ein Stück mit „Datei fehlt“ lässt sich weiter wählen und meldet den Grund; das Ziehen zeigt die Fläche, Dateien öffnen oder werden importiert, Ordner verknüpft.

## C. Lernen und Fortschritt

**J-C1 Erste Woche** · Persona: P1 Lea · Phase 1 (adaptive Schwierigkeit Phase 2) · LH-FOR-01, LH-MOD-06, LH-BEW-04
1. An fünf Tagen je zehn Minuten üben (simuliert über Datumsfortschreibung).
2. Kontra öffnen: die Startseite nennt das zuletzt gespielte Stück, die nächste Übung, die noch nicht sitzt, und die
   Minuten der letzten sieben Tage; jedes mit einem Klick erreichbar. Nach einer Nacht zeigt „Wiederholen“ die zwei
   Takte, die in den letzten drei Tagen am häufigsten am schwersten waren; ein Klick öffnet den Takt in der Werkstatt.
3. Fortschrittsansicht öffnen.
Erwartet: Übungszeit und Verbesserung sichtbar; Formulierungen ermutigend, keine Rangliste; ohne Suchen weiterüben.
Vor dem ersten Lauf steht auf der Startseite statt „0 min · 0“ ein Satz, der zum ersten Durchlauf einlädt; danach zählen
Minuten und Durchläufe beim Öffnen hoch (nicht bei „weniger Bewegung“), und die Auswertung zählt ihre Trefferquote hoch.
Liegt eine Übung nach dem Lauf über 90 %, bietet die Auswertung die nächste im Plan an; die Übungsliste zeigt sie mit
Häkchen.

**J-C2 Timing-Tendenz** ⚙ · Persona: P3 Mira · Phase 1 · LH-BEW-03, LH-BEW-08, LH-BEW-09, LH-EIN-14, LH-FOR-02
1. Auf der Übungsseite „Timing: Laid-back“ wählen, Hilfe dazu lesen.
2. Aufnahme mit konstanter Tendenz „18 ms zu früh“ im Modus „Durchlauf“.
3. Auswertung lesen.
Erwartet: Histogramm mit Achse und Legende; Tendenz in Worten („meist 18 ms zu früh“); zum Ziel ein Satz in Worten
(„Ziel laid-back (10 bis 30 ms dahinter): verfehlt, 28 ms vor dem Zielbereich“), nicht nur Farbe.
Unter der Trefferquote genau eine Beobachtung mit dem nächsten Schritt („3 von 8 Noten saßen noch nicht. Langsamer üben …“);
nach einem kurzen Lauf (unter 8 Noten) keine, ebenso unter 8 Treffern ohne andere Aussage.
4. Ziel auf „Frei“, Aufnahme mit konstanter Tendenz „25 ms zu spät“.
Erwartet: in derselben Beobachtung der Zusatz „… kann auch die gemessene Latenz nicht mehr stimmen“ mit Knopf „Neu
kalibrieren“, der zur Kalibrierung in den Einstellungen führt; keine Korrektur ohne Zutun. Mit Ziel laid-back kein
Zusatz (LH-EIN-14), ebenso nie bei einer mit Kabel gemessenen Latenz (J-A3).

**J-C3 MusicXML und Groove** · Persona: P3 Mira · Phase 2 · LH-BIB-03, LH-LRN-01, LH-LRN-04
1. MuseScore-Export (MusicXML) importieren, in der Notenansicht spielen.
2. Groove-Übung „Sechzehntel mit Ghost Notes“, 92 BPM.
3. Walking-Bass-Muster über ii-V-I in B♭ generieren lassen; dann Dorisch in Terzen in D und einen Groove in 7/8.
Erwartet: Notensatz wie in MuseScore; Streuung in ms; generierte Übung speicherbar; Taktart und Vorzeichen stimmen.

**J-C4 Hören und nachspielen** ⚙ · Personas: P1 Lea, P3 Mira · Phase 3 · LH-LRN-03, LH-LRN-18, LH-BEW-01
1. Lea: Seite „Gehör“ (Strg+7), „Nachspielen“; den Ton noch während der Vorgabe mitspielen, dann einen falschen, dann den richtigen.
2. Drei Fragen in Folge ohne Fehler; die nächste Linie hat zwei Töne. Ein Fehler, die Linie wird wieder kürzer.
3. Mira: „Intervalle“ und „Grundton“ je fünf Fragen, einmal „Nochmal hören“, „Vorspielen“ leiser stellen; bei Intervallen
   die kleine Terz mehrmals verfehlen, Kontra neu starten, weitere zwanzig Intervalle.
Erwartet: Töne während der Vorgabe zählen nicht; jeder gespielte Ton hat Haken oder Kreuz, ein falscher sagt „zu hoch“ oder „zu tief“; gelöst nennt Kontra, was es war (Töne, Intervall, Akkord) und zeigt, wo es am Griffbrett liegt; beim Grundton zählt jede Oktave; bei Intervallen zeigt das Griffbrett den ersten Ton schon vor der Antwort (Raute); ohne Ausgang ein Hinweis statt Stille. Ein oft verfehltes Intervall kommt danach deutlich öfter, auch nach dem Neustart; ein falscher Ton zählt einmal je Frage, Weiterklicken nicht.

**J-C5 Techniken üben** ⚙ · Personas: P3 Mira, P4 Thomas · Phase 4 · LH-BEW-06
1. Mira: Groove aus MuseScore (MusicXML) mit Hammer-on, Pull-off, Slide, Dead Notes (x) und einer Ghost Note ( ), Modus „Durchlauf“.
2. Mit den Techniken spielen, eine Dead Note aber normal anschlagen.
3. Auswertung lesen; Thomas zeigt sie am Beamer, Tab-Lane und Tabulatur.
Erwartet: jede Technik-Note hat ein Zeichen (h, p, sl, x, Klammer) und danach ein Ergebnis in Form, nicht nur Farbe; das Slide-Zeichen und sein Ergebnis stehen an der Ausgangsnote wie in gedruckter Tabulatur; die Ghost Note ( ) hat nur ihr Zeichen, kein Technik-Ergebnis; die Auswertung zählt je Technik („Dead Note: 1 von 2“), die Trefferquote bleibt davon unberührt (die normal angeschlagene Dead Note trifft, eine gedämpft gespielte auch, ohne Tonhöhe; Palm Mute hat eine Tonhöhe und wird an ihr gemessen); Hammer-on- und Pull-off-Töne sind keine zusätzlichen Noten; am Mikrofon sagt die Auswertung, dass Slap und Pop nicht bewertet werden.
4. Thomas (Runde 17): Slap-Groove über Kabel, den Slap normal angeschlagen.
Erwartet: ein erkannter Slap/Pop steht an der Note, ein normal angeschlagener ist kein Technik-Fehler (nur das Zeichen); die Auswertung zählt Slap und Pop nicht und sagt, dass sie nur als Hinweis gezeigt werden; der Ton zählt.

**J-C6 Sitzt es ohne Noten?** ⚙ · Personas: P2 Jens, P1 Lea · Phase 2 · LH-LRN-05, LH-MOD-04
1. Jens: Takt 1 in der Werkstatt üben, bis er im Zielband liegt; „Ohne Hilfe prüfen“.
2. Nach dem Abschnitt einfach weiterspielen.
3. Am nächsten Tag: die Startseite zeigt „Ohne Hilfe“ mit den Takten; ein Klick startet die Probe.
4. Lea: dasselbe mit einer Übung aus Stufe 1, bei 200 % Schrift.
5. Jens: einen Takt zweimal mit Fehlern üben (LH-LRN-16); am nächsten Tag „Stellen wiederholen“, ohne Noten sauber im
   Zieltempo; zwei Tage später wieder auf die Startseite.
Erwartet: Während der Probe keine Noten, nur Takte und Cursor, ein Hinweis sagt warum; nach einem Durchgang Schluss,
Weiterspielen zählt nicht; die Auswertung zeigt „Werkstatt, mit Noten“ und „Ohne Hilfe“ nebeneinander (Zahl und Satz,
nicht nur Farbe); danach ist die Werkstatt wieder normal, die Startseite bietet dieselben Takte nicht noch einmal an.
Der zweimal schwere Takt kommt nach einer Nacht als „Stellen wiederholen“ (höchstens 3, keine leere Karte, keine Zahl
offener Stellen); bestanden bleibt er zwei Tage weg (Abstand verdoppelt), auf „Wiederholen“ steht er
nicht zugleich.
Bringt ein normaler Lauf eine geplante Stelle wieder unter die schwersten Takte, zählt das als nicht bestanden: sie
kommt nach einem Tag wieder (Runde 19).

**J-C8 Erst der Puls** ⚙ · Personas: P1 Lea, P5 Sam · Phase 2 · LH-LRN-06, LH-LRN-01
1. Lea: Übungen, Stufe 1, „Groove Viertel auf einem Akkord in E (70 BPM), nur Timing“; das Ziel lesen.
2. Durchlauf auf der leeren A-Saite statt E, im Takt; dann einmal deutlich zu spät.
3. Sam: dieselbe Übung am Mikrofon.
4. Lea: „Übung erzeugen“, Groove Achtel über I-IV-V, Bewertung „Nur Timing“; dann dieselbe mit „Ton und Timing“.
Erwartet: Das Ziel sagt, dass nur der Anschlag zählt; jeder Ton zur rechten Zeit trifft, welcher Ton auch klingt; der
späte zählt als spät; die Streuung in ms steht in der Auswertung, mit dem Ziel „Bünde ab ±25 ms“; erst mit kleiner Streuung
bietet die Auswertung die nächste Übung an. Die nächste Übung mit Bünden bewertet wieder den Ton.
Titel „…, nur Timing“ / „…, timing only“; die zweite erzeugte Übung überschreibt die erste nicht, jede hat eigenen
Fortschritt; „Bewertung“ steht nur bei der Art „Groove“.

**J-C9 Bausteine** ⚙ · Personas: P1 Lea, P2 Jens, P4 Thomas · Phase 2 · LH-MOD-08, LH-MOD-04
1. Jens: ein eigenes Lied, Werkstatt, Takte 1–8, „Bausteine: Je 2 Takte“.
2. Spielen, bis „Alles zusammen: Takte 1–8“ steht; einmal mitten in einem Schritt schlecht spielen.
3. Lea: dasselbe mit einer Übung aus Stufe 1, mitten drin „Von hinten“ einschalten, dann „Aus“; bei 200 % Schrift.
4. Thomas: von Hand andere Takte wählen.
5. Jens: „Übung erzeugen“, Groove mit Pausen über I-IV-V; Werkstatt, Takte 1–8, „Bausteine: Nach Phrasen“.
Erwartet: Die Zeile nennt Takte und Schritt („Baustein: Takte 3–4 · Schritt 2 von 7“); ein Schritt endet erst nach zwei
Durchgängen im Zielband, ein schwacher zählt neu; danach Einzähler und der nächste Schritt ohne Klick; „Von hinten“
beginnt mit den letzten der gewählten Takte (nicht des Stücks), „Aus“ zeigt wieder alle gewählten Takte; von Hand gewählte
Takte beenden die Bausteine, die Liste steht wieder auf „Aus“. Nach Phrasen: Bausteine zu 2 Takten (die Pause am Taktende
trennt), Schritt 1 von 7; eine Linie ohne Pausen ergibt Bausteine zu 4 Takten, außer vor einem Takt, an dem Jens in
zwei der letzten fünf Durchläufe gezögert hat (erster Ton fehlt oder kommt spät): dort beginnt ein Baustein, sofern
der Baustein davor mindestens zwei Takte behält.

**J-C10 Griffbrett gezielt** ⚙ · Personas: P1 Lea, P2 Jens, P3 Mira, P7 Sam · Phase 2 · LH-LRN-18, LH-LRN-02
1. Lea: Seite „Griffbrett“, „Töne finden“; Bünde „0–4“ ist gewählt. Zehn Fragen, bei „H auf der A-Saite“ zuerst
   einen falschen Ton, eine Frage mit „Nächste Frage“ überspringen.
2. Kontra beenden und neu starten, wieder das Quiz; unter dem Griffbrett die Übersicht „Treffer je Saite und Bund“ lesen.
3. Jens: Bünde „Eigene“, ab Bund 5 bis Bund 9; Mira: „0–12“ am Fünfsaiter. Sam: alles mit Tastatur, 200 %, Deuteranopie.
Erwartet: Fragen nur aus den gewählten Bünden; die Wahl bleibt nach dem Neustart; der verfehlte Ton kommt deutlich öfter
als andere (bei 20 Plätzen mehr als jede zehnte Frage); Überspringen zählt nicht als Fehler, ein falscher Ton einmal je
Frage, auch in „x von y gefunden“; bei 0–12 ist Bund 12 kein eigener Platz; nach Export und Import (Ersetzen) auf einem
zweiten Rechner gleicher Bereich und gleiche Statistik, beim Ergänzen gleiche Statistik und der eigene Bereich; Bereich und Bünde haben Namen für den Screenreader. Die Übersicht zeigt je Saite (Reihenfolge wie das Griffbrett) und Bund den Anteil richtig als Zahl, Zählungen im Tooltip und für den Screenreader, nie gefragte Plätze leer; bei 1024×640 ist sie per Scrollen erreichbar.

**J-C7 Ein ganzes Stück** · Personas: P1 Lea, P2 Jens, P4 Thomas, P3 Mira, P5 Aylin · Phase 3 · LH-BIB-12, LH-BIB-13, LH-BIB-08, LH-MOD-02, -04, LH-WIE-05
1. Lea: Übungen, Stufe „9 Stücke“, die Ballade im Durchlauf von vorn bis hinten; danach „Übung erzeugen“, Stück Pop, Schwierigkeit „Leicht“.
2. Jens: „Übung erzeugen“, Art „Stück“, Stil Rock in E, 100 BPM; den ersten Refrain in der Werkstatt üben.
3. Thomas: dasselbe Stück am Beamer in Noten + Tab für die Klasse, Stil Funk in Am.
4. Mira (Aylin über MIDI): Einstellungen „Band reagiert“ an; das Funk-Stück im Modus „Spielen“ mit eigenen Phrasen,
   am Phrasenende zwei Schläge frei, einmal einen Takt durchgehend in Achteln; dann wieder aus.
Erwartet: Stück läuft zwei bis drei Minuten (Rock bei 100 BPM gut zwei); Akkordname über jedem Takt; Probenzeichen
(Intro, Strophe, Refrain …) über dem ersten Takt jedes Teils, in Lane, Tab und Noten, der Screenreader nennt den Teil; Schlagzeug, vor jedem neuen Teil ein Fill;
letzter Takt auf der Tonika; Titel in der Oberflächensprache („Stück Rock“ / „Rock piece“); der Generator passt in ein
schmales Fenster. Leicht: Pop mit Intro und Outro in Halben, Strophe und Bridge in Vierteln, Refrain in Achteln, ohne Synkopen; Titel „Stück Pop (leicht)“, Form und Schlagzeug wie
mittel; „Schwierigkeit“ steht nur bei der Art „Stück“. Band reagiert: nach zwei freien Schlägen am Phrasenende ein
Snare-Fill im zweiten, höchstens alle 8 Takte, nie mitten in einer Phrase; nach dem Achteltakt ein Takt ohne Hi-Hat;
keine Auswertung ändert sich; aus: das Schlagzeug spielt wie geschrieben.

**J-C11 Eigene Linie** ⚙ · Personas: P1 Lea, P3 Mira, P4 Thomas · Phase 3 · LH-BEW-13
1. Lea: Lektion „Eine eigene Linie“, „Ausprobieren“: die offene Übung (Groove I–IV–V mit Pausen, frei) im Durchlauf;
   eine Linie nur aus Grundton und Quinte, die Pausen des Beispiels frei lassen.
2. Mira: dieselbe Übung mit Durchgangstönen und zwei Tönen außerhalb der Tonart, einmal durch die Pausen spielen.
3. Thomas: am Beamer erklären, was „passend“ und „Akkordtöne“ heißen.
Erwartet: Titel endet auf „, frei“; die Tab zeigt das Beispiel, kein Ton daraus gilt als ausgelassen; die Auswertung
sagt „x % passend“ und zählt Akkordtöne, Töne der Tonart und fremde Töne, dazu den Anteil Akkordtöne und „Pausen: x
von y frei gelassen“; Lea: 100 % passend, 100 % Akkordtöne, alle Pausen frei; Mira: zwei fremde Töne, nicht alle
Pausen frei.

**J-C12 Ruf und Antwort** · Personas: P1 Lea, P3 Mira, P5 Aylin · Phase 3 · LH-LRN-19
1. Lea (Mikrofon): Übungen, ein Groove mit Akkorden (I–IV–V, Viertel); „Ruf und Antwort“ an, Modus „Durchlauf“.
   Kontra spielt zwei Takte, Lea antwortet; die ersten beiden Durchläufe absichtlich daneben.
2. Mira (Kabel): dasselbe im Modus „Frei spielen“ (ohne Bewertung) und in der Werkstatt mit einer Schleife über vier Takte.
3. Aylin (MIDI): ein importiertes Stück ohne Akkordsymbole öffnen.
Erwartet: Der Schalter steht nur bei Stücken mit Akkorden und mindestens vier Takten; an: je vier Takte ein Ruf (zwei
Takte, Kontra spielt, das Schlagzeug leiser) und die Antwort (zwei Takte, gleiches Motiv aus Akkordtönen, höchstens eine
Quinte); die Tab ist leer, der Hinweis „Hör zu und antworte …“ sagt warum; nach zwei Durchläufen (oder Durchgängen
der Schleife) unter 80 % erscheint die Tab; über Mikrofon zählt der Ruf weder als Treffer noch als zusätzliche Note; Aylin sieht keinen Schalter.

**J-C13 Stück wünschen** · Personas: P1 Lea, P2 Jens, P3 Mira, P4 Thomas · Phase 3 · LH-BIB-16
1. Jens: Übungen, „Stück wünschen …“ ohne eingerichteten Anbieter öffnen; den Hinweis lesen, „Anbieter einrichten …“.
2. Jens: Anbieter „OpenRouter“, Schlüssel einfügen, „Speichern“; zurück im Wunsch-Dialog einen Wunsch eintragen
   („Motown-Groove in A, 16 Takte Refrain mit Achtel-Pushes“), Schwierigkeit „Mittel“, „Generieren“.
3. Lea: denselben Wunsch, Schwierigkeit „Leicht“; während die Antwort noch läuft, den Dialog betrachten.
4. Mira: einen Anbieter ohne Schlüssel nötig (lokaler Server) einrichten, Modell über „Modelle laden“ wählen.
5. Thomas: einen Schlüssel ersetzen und wieder löschen; den Hinweis "kein Schlüssel" danach sehen.
Erwartet: Ohne Anbieter oder ohne Schlüssel nennt der Dialog das und bietet nur den Link zu den Einstellungen, kein
„Generieren“; während der Anfrage läuft, eine Zeile sagt das, kein zweiter Start möglich; eine gültige Antwort öffnet
das Stück wie ein erzeugtes (Praxisseite, Fortschritt, Werkstatt); eine zweimal ungültige Antwort zeigt die
gefundenen Probleme statt eines Absturzes; der Schlüssel steht nach „Speichern“ nie im Klartext im Feld und ist in
keiner Einstellungsdatei oder Sicherung zu finden; „Löschen“ setzt den Zustand auf „kein Schlüssel“ zurück.

## D. Inhalte teilen (ab Phase 3)

**J-D1 Original-Song als Backing Track** ⚙ · Persona: P2 Jens, P3, P4 (Schritt 6) · Phase 3 · LH-BIB-06, LH-WIE-01, LH-WIE-02
1. MP3 an das Guitar-Pro-Arrangement koppeln, Beat-Erkennung.
2. Zwei Taktanfänge von Hand korrigieren.
3. Bei 75 % Tempo durchspielen.
4. Die Sängerin will es einen Ganzton höher: „Transponieren“ +2 mitten im Lied, weiterspielen; Kontra neu starten.
5. Unter „Bass der Aufnahme“ „Ohne Bass“ wählen (erst ohne Demucs, dann mit), einmal abbrechen, nach der Trennung durchspielen, dabei im Mixer „Original-Bass“ auf 30 % und zurück auf 0 % ziehen, Kontra neu starten, die getrennte Datei löschen.
6. Einstellungen → Deine Daten: unter „Zwischenspeicher“ einen vorbereiteten Backing-Track löschen, während er spielt; danach „Songs ohne Bass“ ganz leeren.
7. Nach dem fertigen Lauf dieselbe Aufnahme noch einmal wählen, dann eine andere, dann den Backing-Track entfernen.
Erwartet: kein hörbarer Versatz > 20 ms über 4 Minuten; die Erkennung liegt auf dem Schlag, nicht auf dem Offbeat, und zeigen Kick und Bass den Schlag nicht, sagt sie „unsicher“ (Warnung statt Haken) und bietet „einen halben Schlag später“ an; nach dem Öffnen bereitet Kontra den Backing-Track im Hintergrund vor (Ladeanzeige, die Oberfläche bleibt bedienbar, Abspielen wartet mit Hinweis), beim nächsten Öffnen kommt er sofort aus dem Cache; der Wechsel hält das Lied nicht an, Bewertetes bleibt; transponiert passen Tab und Audio zusammen (gleiche Tonart, gleiche Takte), Bünde aus der Datei bleiben, wo es geht; die Einstellung bleibt beim Stück, der Fortschritt auch. Ohne Demucs ist „Ohne Bass“ ausgegraut und „?“ sagt, wie man es installiert; mit Demucs läuft die Trennung im Hintergrund (niedrige Priorität) mit Prozent, Restzeit und Abbrechen; Abbrechen lässt keine Dateien zurück. Danach spielt der Backing-Track ohne Bass, im selben Abgleich, und beim nächsten Mal ohne neue Trennung; die Datei liegt im Cache-Ordner, „?“ nennt Ort und Größe, „Getrennte Datei löschen“ gibt den Platz frei. „Original-Bass“ steht im Mixer nur, solange ohne Bass gespielt wird, und wirkt sofort (30 %: Bass leise hörbar, im Takt mit dem Rest; 0 %: weg). Ein Fehler nennt die Meldung von Demucs und was zu tun ist und bleibt bei seinem Song. Der Zwischenspeicher nennt beide Caches mit Summe und je Eintrag Song-Namen und Größe; Löschen nimmt den Eintrag aus der Liste und senkt die Summe, das laufende Lied spielt weiter, das offene Stück wird im Hintergrund neu vorbereitet; geleert spielt der Song wieder mit Bass. Wird der Backing-Track entfernt oder durch eine andere Aufnahme ersetzt, geht der alte Abgleich mit: die Takte stehen wieder im Tempo des Stücks, die neue Aufnahme gilt als nicht abgeglichen. Dieselbe Aufnahme noch einmal gewählt ändert nichts, die Auswertung des fertigen Laufs bleibt stehen; mitten im Lauf gewählt beginnt die Seite neu.

**J-D2 Übung an Schüler geben** ⚙ · Persona: P4 Thomas · Phase 3 · LH-BIB-07, LH-FOR-04
1. Übung aus Guitar Pro importieren, Begleitspur wählen und abgleichen, in der Werkstatt Loops merken („Loop merken“) und das Zieltempo setzen, einmal üben, als `.kontra` exportieren (mit Begleitspur).
2. Auf zweitem Profil importieren.
3. Fortschrittsbericht exportieren.
Erwartet: identische Darstellung, Loops und Zieltempo; die Begleitspur liegt im Datenordner des Schülers und spielt ohne neuen Abgleich; der Fortschritt des Schülers beginnt bei null (der des Lehrers kommt nicht mit). Audio wird nur mitgegeben, wenn er es ausdrücklich wählt; eine Begleitspur über 256 MiB lehnt der Export mit Grund ab. Der automatische Test gleicht eine Klick-Band ab, merkt zwei Loops, exportiert und spielt das Stück auf dem zweiten Profil mit Begleitspur.

## E. Weitere Bass-Eingänge (ab Phase 4)

**J-E1 Synth-Bass über MIDI** ⚙ · Persona: P5 Aylin · Phase 4 · LH-EIN-12
1. Instrument „Bass über MIDI“ (Einstellungen → Instrument; der Anschluss wird „MIDI-Instrument“). Zwei Keyboards angesteckt: unter „MIDI-Gerät“ das zweite wählen, Kontra neu starten, dann das zweite abziehen.
2. Funk-Groove im Modus „Durchlauf“, Ansicht Klaviatur (Alt+5).
Erwartet: Timing aus MIDI-Zeitstempeln, gleiche Auswertung wie beim E-Bass. Die Wahl des Keyboards bleibt nach dem Neustart; fehlt es, nennt Anschluss das Keyboard, das Kontra stattdessen nimmt. Die Noten fallen auf die Tasten, die nächste ist auf der Klaviatur markiert; die Tasten reichen über den Umfang des Basses (4 Saiten: E1–G4), Tastennamen in der Oberflächensprache (H statt B). Der automatische Test spielt eine MIDI-Datei wie ein Keyboard ein und prüft, dass jede Note auf einer gezeigten Taste liegt.

**J-E2 Kontrabass im Wohnzimmer** ⚙ 🔍 · Persona: P6 Karl · Phase 4 · LH-EIN-11, LH-NF-03, LH-NF-06
1. USB-Mikrofon, Instrument „Kontrabass“ (Anschluss Mikrofon vorgeschlagen, Wertung als Kontrabass), keine Kopfhörer, Backing Track über Lautsprecher.
2. Standard im Modus „Frei spielen“, Notenansicht, Schrift 150 %.
3. Auf Mindest-Hardware.
Erwartet: Hinweis, dass die Toleranzen weiter sind; Trefferquote ≥ 90 % auf dem Testkorpus; Lane flüssig. Testkorpus: `room-double` (ohne Band, erfüllt) und `room-double-backing` (Band aus den Lautsprechern, mit Abzug erfüllt).

**J-E3 Ton schwingt ein** ⚙ · Personas: P6 Karl, P3 Mira · Phase 4 · LH-BEW-07, LH-BEW-15
1. Walking-Linie im Song-Modus auf einem Kontrabass, dessen Töne zu tief beginnen und in rund 100 ms auf ihre Tonhöhe steigen.
Erwartet: Jede Note wird so schnell markiert wie sonst; eine Note, die beim Anschlag als falscher Ton gilt, wird binnen etwa 0,2 s zum Treffer. Keine zusätzlichen Noten (Hammer-on) für das Einschwingen. Der automatische Test spielt eine synthetische Linie (D2 E2 F2 G2, 140 Cent zu tief, 120 ms Anstieg).
2. Seit R1 (LH-BEW-15): vor einzelnen Anschlägen die Saite mit dem greifenden Finger leise antippen.
Erwartet: Der Fingertipp erscheint nicht als eigene Note; eine kurz markierte Note wechselt binnen etwa 0,1 s auf das Urteil des Anschlags. Automatisch geprüft in `early_tap_before_the_pluck_is_taken_back` (Erkennung) und `crates/session/tests/retraction.rs` (Urteil).
3. Im Warte-Modus die Saite auf dem richtigen Ton antippen, dann einen falschen Ton anschlagen.
Erwartet: Die Markierung des Tipps verschwindet, Kontra wartet ohne Markierung wieder auf die Note und nennt den falschen Ton. Automatisch geprüft in `tap_taken_back_clears_its_mark_waiting`.

## G. Lernen

**J-G1 Erste Lektion** ⚙ · Persona: P1 Lea · Phase 2 · LH-LRN-07, LH-LRN-08
1. Seite „Lernen“, Kapitel 1, erste offene Lektion; lesen, Hörbeispiel, ausprobieren am Bass, langsam üben, prüfen.
2. Kontra schließen und öffnen.
3. Dasselbe bei 200 % Schrift, ohne Eingang.
Erwartet: Jeder Schritt passt auf einen Bildschirm; beim Ausprobieren sagt ein falscher Ton, was er war; die Startseite
zeigt „Weiter: <nächste Lektion>“; ohne Eingang antwortet ein Klick auf die Klaviatur statt des Basses. Die Auswertung
einer Lektionsübung nennt ihr Ziel („Ziel 80 %, erreicht 72 %“, LH-LRN-20); eine Übung ohne Ziel zeigt keine Prozentzahl.
Unter jeder Übung die Übungsleiter (LH-LRN-17): fünf offene Stufen mit „neu“ oder „bisher x %“ und ihrer Schwelle, die
erste nicht erreichte hervorgehoben; nach einem Lauf über der Schwelle rückt die Hervorhebung eine Stufe weiter.

**J-G2 Was ist eine Septime?** ⚙ · Personas: P2 Jens, P1 Lea · Phase 2 · LH-LRN-09
1. Jens: Übung erzeugen, Akkordfolge II-V-I; „?“ daneben zeigt beim Zeigen den ersten Satz, ein Klick den Eintrag;
   weiter über „Septakkorde“ zu „Septime“, Hörbeispiel; zurück.
2. Lea: in der Auswertung „Slide: 3 von 5“, „?“ an „Slide“; zurück.
3. Jens: Walking-Übung II-V-I in der Ansicht „Noten“; unter dem Griffbrett „Akkordsymbol ?“; ein Stück mit Techniken
   zeigt dort seine Zeichen (h Hammer-on ?, sl Slide ? …).
Erwartet: Eintrag kurz mit Abbildung und Verweis auf die Lektion; zurück an dieselbe Stelle (auch gescrollt), nichts
verstellt, der Fokus wieder auf dem „?“ oder Link, der den Eintrag öffnete; in der Übungsliste bleibt danach jede
Zeile per Tab und Umschalt+Tab erreichbar. Eine Auswahl ohne Begriff (Funk) zeigt kein „?“; „Akkordsymbol“ nur, wo Akkordsymbole stehen (Noten).

**J-G3 Wiederholen** · Persona: P1 Lea · Phase 2 · LH-LRN-10
1. Prüffragen von Kapitel 1, eine falsch.
2. Drei Tage später (Datum fortgeschrieben) die Startseite öffnen.
Erwartet: fällige Fragen stehen auf der Startseite (Karte „Wiederholen“, Seite „Wiederholen“ fragt sie einzeln ab, mit
der Lektion, aus der sie stammen); die falsche kommt früher (nach einem Tag, richtige nach drei); „Kann ich schon“ (Lernen,
je Kapitel) fragt alle Fragen des Kapitels; alle richtig hakt es ab, sonst bleibt es offen. Test:
`tests/journeys/g3_review.rs`.

**J-G4 Eigene Lektion** · Persona: P4 Thomas · Phase 3 · LH-LRN-11
1. Lektion nach der Formatbeschreibung schreiben, in Kontra öffnen, am Beamer (1280×720) zeigen.
Erwartet: liest sich wie eine mitgelieferte; Fehler im Format mit Zeile genannt.

**J-G5 Kurs nachladen** · Personas: P2 Jens, P7 Sam · Phase 4 · LH-LRN-14
1. Jens: Kurs-Adresse eines Anbieters eingeben oder die heruntergeladene `.kontrakurs` öffnen (Lernen → „Kurs
   hinzufügen …“ oder Doppelklick); Anbieter, Lizenz und Video-Hosts lesen, hinzufügen; eine Lektion mit Video.
2. Sam: dieselbe Lektion bei 200 % und „Bewegung reduzieren“, mit Orca.
3. Ohne Netz neu starten.
4. Jens: „Nach Update suchen“ (Kurs von einer Adresse), dann den Kurs entfernen („Entfernen …“) und wieder hinzufügen.
Erwartet: Kurs erscheint neben dem Bass-Kurs; Video mit Untertiteln, Text darunter lesbar; ohne Netz bleibt ein
heruntergeladener Kurs nutzbar, ein Stream-Video sagt, dass es Netz braucht; der Lernstand gemeinsamer Lektionen gilt.
Das Update kommt als Vorschau und nennt die Fassung („Neuere Fassung 2 (vorhanden: 1)“, „schon da“ oder „ältere“); Entfernen fragt erst; danach lernt Jens den Bass-Kurs, nach dem erneuten Hinzufügen steht sein Lernstand wieder da.

**J-G6 Kurs bauen** · Persona: P4 Thomas · Phase 4 · LH-LRN-15
1. In der Kurswerkstatt einen Kurs anlegen, zwei Lektionen schreiben (eine mit Fehler im Format), Vorschau ansehen.
2. Prüfen, Fehler beheben, als `.kontrakurs` packen; dasselbe mit `kontra-cli course new|check|pack`.
3. Das Paket in Kontra hinzufügen und eine Lektion spielen.
4. Lernen → „Kursordner live öffnen …“ mit dem ungepackten Ordner, eine Lektion öffnen und ihre Übung kurz spielen; im
   Editor ihren Text ändern und speichern, dann einen Fehler hineinschreiben und speichern; „Ordner schließen“
   (LH-LRN-21).
Erwartet: Fehler mit Datei und Zeile; die Vorschau sieht aus wie in Kontra; das Paket lädt ohne Befund. Die geänderte
Lektion erscheint nach dem Speichern ohne Packen (binnen einer Sekunde); der Fehler steht mit Datei und Zeile über der
Lektion, die letzte lesbare Fassung bleibt sichtbar; gesehene Seiten, Antworten und Läufe der Übung werden im Live-Ordner
nicht gespeichert (Fortschritt bleibt leer); nach dem Schließen gilt wieder der hinzugefügte Kurs.

## F. Querschnitt

**J-F1 Barrierefreiheit** 🔍 · Persona: P7 Sam · Phase 1 · LH-ANZ-06, LH-MOD-07, LH-NF-07, LH-ALL-02
1. Skalierung 200 %, Farbfilter Deuteranopie, „Bewegung reduzieren“ an.
2. J-A1 und J-B3 nur mit Tastatur, dazu Saitenwahl im Stimmgerät und Auswertung schließen (Esc).
3. Mit Orca: Bibliothek, Einstellungen und Auswertung vorlesen lassen.
4. Bei 1280×720 und 150 % ein Stück in jeder Ansicht (Lane, Tab, Noten, Noten + Tab, Klaviatur) abspielen.
Erwartet: Fokus immer sichtbar; die Auswertung hält den Tab-Fokus wie ein Dialog, die Seite dahinter wird nicht vorgelesen, nach Esc steht der Fokus auf „Abspielen“; die Balken des Timing-Histogramms nennen Bereich und Anzahl; die Seitenleiste bleibt in Kante auch auf hellem System dunkel, ohne helle Fläche; Treffer/Fehler/früh/spät ohne Farbe unterscheidbar; alle Bedienelemente haben Namen; keine abgeschnittenen Texte; Animationen reduziert; Pegel und Pegel-Hinweis in jeder Ansicht unter den Noten, nichts überlappt; im niedrigen Fenster stehen Transponieren, Timing und Notendichte (in der Werkstatt auch Zieltempo, gemerkte Loops und Bausteine) hinter „Mehr Einstellungen“, der Tab zeigt alle Saiten; Abspielen, Pegel, Pegel-Hinweis und Griff-Hinweise verschieben die Noten nicht; im Einrichtungsschritt „Pegel und Stimmen“ erreicht Tab das Stimmgerät und zeigt Ton, Cent und Drehrichtung.

**J-F2 Sprache** · alle · Phase 1 · LH-ALL-01
1. System auf Englisch, J-A1 und J-B1.
2. In den Einstellungen auf Deutsch wechseln, Auswertung und Hilfe öffnen.
3. System auf Deutsch, Kontra auf Englisch starten; Seitenleiste ein- und ausklappen; auf Deutsch wechseln und zurück.
Erwartet: keine rohen Schlüssel, kein Sprachmix, Fachbegriffe in beiden Sprachen erklärt. In Schritt 3 spricht auch die Seitenleiste („Close Sidebar“) Kontras Sprache, nach dem Start, nach dem Einklappen und nach jedem Wechsel.

**J-F3 Störungen** · Persona: P2 Jens · Phase 1 · LH-ALL-04, LH-NF-04
1. Während des Spielens das Interface abziehen, wieder anstecken (10-mal).
2. 60 Minuten Modus „Frei spielen“ mit Time-Stretch auf Mindest-Hardware.
3. Interface abziehen; solange das Angebot „<Gerät> nutzen“ in der Leiste steht, in den Einstellungen einen anderen Eingang wählen.
Erwartet: kein Absturz, klare Meldung, Interface wird wieder genutzt; Xrun-Zähler 0. In Schritt 3 verschwindet das Angebot mit der Wahl, der gewählte Eingang gilt.

**J-F4 Eingang gestört** ⚙ 🔍 · Personas: P2 Jens, P1 Lea, P7 Sam, P6 Karl · Phase 1 · LH-EIN-15
1. Interface abgezogen, ein Stück öffnen, Abspielen.
2. Interface anstecken, „Erneut versuchen“, Abspielen.
3. Mitten im Lauf liefert das Gerät nichts mehr (am Solo: USB kurz trennen); „Erneut versuchen“.
4. Lauf beenden; Stimmgerät mit abgezogenem Interface öffnen und wieder verlassen.
Erwartet: „Eingang gestört“ mit Zeichen, Text, Meldung des Systems und „Erneut versuchen“ (keine zweite Fehlerleiste) bleibt in der Leiste unter der Seite, bis der
Eingang geht, auch nach anderen Meldungen; danach „Eingang verbunden: <Name>“ unter dem Pegel. Im Lauf „Kein
Signal vom Gerät“, keine Note als verpasst; „Erneut versuchen“ lässt den Lauf an seiner Stelle weiterlaufen, er wird einmal gespeichert.
Verweigert das System das Gerät: „Kein Zugriff auf den Eingang“ mit dem Weg zur Freigabe. Orca sagt jeden Wechsel.
Nach dem Stimmgerät keine Anzeige mehr. Schritte 1, 2 und 4 mit einer Aufnahme statt Interface automatisiert;
Schritt 3 am Solo (Aussetzer lassen sich mit einer Datei nicht erzeugen).
