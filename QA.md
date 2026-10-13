# Kontra: QA-Journeys

Feste User Journeys für explorative QA und für automatisierte Journey-Tests. Jede Journey gehört zu einer oder mehreren Personas aus [`PERSONAS.md`](PERSONAS.md)
und nennt die Anforderungen aus dem [Lastenheft](LASTENHEFT.md), die sie prüft. Wer etwas an Kontra ändert, spielt die betroffenen Journeys durch (siehe [`agent.md`](agent.md)).

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
2. Assistent: Instrument „E-Bass, 4 Saiten“, Stimmung „Standard“; Anschluss „Klinke direkt in den Laptop“ (MIDI und Mikrofon stehen als „kommt später“ in der Liste); Eingang wählen (ein Interface ist vorgewählt). Pegel ist zu leise.
3. Den Hinweis lesen und befolgen (Lautstärke am Bass auf, Gain im Assistenten).
4. Kalibrierung überspringen.
5. Erste Übung wählen: von der Startseite („Nächste Übung“) oder aus der leeren Bibliothek („Mit einer Übung anfangen“), ersten Ton spielen.
Erwartet: ≤ 60 s bis zum ersten bewerteten Ton; der Hinweis zur Laptop-Buchse erklärt in einem Satz, warum es dumpf klingt, und verweigert nichts; kein Fachbegriff ohne Erklärung.

**J-A2 Stimmen ohne Vorwissen** ⚙ 🔍 · Persona: P1 Lea · Phase 1 · LH-TUN-01, -02, -03, -04
1. Stimmgerät öffnen, „Geführt“.
2. E-Saite 30 Cent zu tief anschlagen (`stimmen-e-30c.wav`, erzeugt der Journey-Test), Anweisung befolgen.
3. Alle vier Saiten durchgehen.
4. Einstellungen: Stimmung „Drop D“, dann „Eigene“ (oberste Saite F), Kammerton 442 Hz; Stimmgerät erneut öffnen, eine MIDI-Datei öffnen.
Erwartet: Anweisung in Worten („etwas höher drehen“) und Symbol; Erfolg pro Saite eindeutig, auch ohne Farbe. Nach Schritt 4 führt das Stimmgerät durch D A D F, Cent beziehen sich auf 442 Hz, die Tab liegt auf den neuen Saiten; alles bleibt nach einem Neustart.

**J-A3 Latenz kalibrieren** ⚙ · Persona: P3 Mira · Phase 1 · LH-EIN-05, -07
1. Einstellungen → Eingang → Kalibrieren, „Spiel auf den Klick“.
2. Acht Anschläge mit +40 ms Versatz einspeisen.
3. Song-Modus, pünktlich gespielte Aufnahme einspeisen.
4. PipeWire-Quantum auf 2048 setzen, Kontra neu starten.
Erwartet: gemessene Latenz ≈ 40 ms angezeigt; danach Bewertung pünktlich (±5 ms); bei Quantum 2048 Hinweis mit Wert und Anleitung.

**J-A4 Eingang, den es noch nicht gibt** · Personas: P5 Aylin, P6 Karl · Phase 1 bis 3 · LH-EIN-09, -10, -12
1. MIDI-Keyboard anstecken, Kontra starten, Einstellungen → Anschluss (P5). Danach USB-Mikrofon als Eingang wählen wollen (P6).
Erwartet: „MIDI-Instrument“ ist wählbar und nennt das erkannte Keyboard (ohne Keyboard: „gerade ist keins angeschlossen“); das Mikrofon erscheint als „kommt in einer späteren Version“ mit einem Satz Erklärung; kein Absturz, keine leere Liste, kein „Gerät defekt“.

## B. Spielen und Üben

**J-B1 Lied durchspielen** ⚙ 📽 · Persona: P2 Jens · Phase 1 · LH-BIB-01, -04, LH-MOD-01, LH-WIE-03, LH-EIN-08, LH-ANZ-01, -02
1. Ordner mit vier Guitar-Pro-Dateien der Setlist importieren (eine davon defekt).
2. Setlist „Freitag“ anlegen, erstes Lied, Bassspur wählen.
3. Modus Spielen, Backing Track (synthetisch bzw. ab Phase 3 eigene MP3), durchspielen; im Mixer (X) Backing leiser, Metronom lauter, Mithören an, Tiefen +4 dB, Kompressor und Wärme halb, „Neutral“, Mithören aus.
4. Mitten im Lied von Tab-Lane auf Tabulatur umschalten.
5. Nächstes Lied der Setlist.
Erwartet: eine verständliche Meldung zur defekten Datei, drei Lieder importiert; kein Anhalten, keine Punkte; Mixer wirkt sofort ohne Neustart, Mithören nennt die Latenz; Klangregler knacken nicht und ändern die Bewertung nicht; Umschalten ohne Aussetzer; Liedwechsel in ≤ 2 Klicks.

**J-B2 Warten-Modus** ⚙ 🔍 · Persona: P1 Lea · Phase 1 · LH-MOD-03, LH-BEW-04
1. Übung „Seven-Nation-Riff“ (eigene Datei) im Modus Warten.
2. Drei falsche Töne, dann den richtigen.
3. Lange Pause (30 s), dann weiter.
Erwartet: Lied wartet ohne Zeitdruck; falscher Ton wird freundlich angezeigt (welcher Ton gespielt, welcher erwartet, in Worten und Symbol); nach der Pause kein Abbruch.

**J-B3 Refrain sitzt bis Freitag** ⚙ · Persona: P2 Jens · Phase 1 · LH-MOD-04, -05, -06, -07, LH-WIE-01, LH-BEW-03
1. Song-Modus, Lied 3 durchspielen; Auswertung zeigt schwierigste Takte.
2. Auf den schwierigsten Takt klicken → Riff-Werkstatt mit diesem Bereich.
3. Tempo 60 %, automatische Anpassung an das Zielband 80–90 % Treffer, Vorzähler an.
4. Fehlerfreie Durchläufe einspeisen bis 100 %.
5. Notendichte „Auf den Schlägen“ wählen; nach einem Durchlauf über 90 % den Vorschlag „mehr Noten“ annehmen.
6. Alles nur mit Tastatur wiederholen (Loop mit `[` und `]` setzen, Pos1 springt an den Loop-Anfang), dann mit einem Fußschalter (Bild ab/auf).
Erwartet: Bereich und Tempo stimmen; Backing Track bei 60 % ohne metallischen Klang; Steigerung sichtbar; jede Funktion per Tastenkürzel.

**J-B4 Bundlos und tief** ⚙ · Persona: P3 Mira · Phase 1 · LH-TUN-01, LH-BEW-01, -02, -05, LH-NF-03, LH-ANZ-03
1. Instrument „5 Saiten, bundlos“ anlegen.
2. Walking-Bass-Etüde in der Notenansicht, Zeitfenster „streng“.
3. Aufnahme mit absichtlichen Intonationsfehlern (±10 bis ±40 Cent) und tiefen Tönen auf der B-Saite einspeisen.
Erwartet: Cent-Werte pro Note weichen ≤ 3 Cent vom annotierten Wert ab; B0 bis D1 sicher erkannt; Noten im Bassschlüssel korrekt.

**J-B5 Unterricht am Beamer** 📽 🔍 · Persona: P4 Thomas · Phase 1 (Präsentationsmodus Phase 2) · LH-ANZ-02, -04, -09, LH-ALL-02, LH-MOD-07
1. Auflösung 1280×720, Schrift 150 %.
2. Übung öffnen, kombinierte Ansicht Noten über Tab, Spielen-Modus.
3. Stil auf Kante Light wechseln (heller Raum).
4. Präsentation (F5 oder Knopf neben dem Mixer), durchspielen, mit Esc beenden.
Erwartet: nichts abgeschnitten, Lane und Noten aus 4 m lesbar, Kontrast ausreichend; in der Präsentation Vollbild ohne Seitenleiste, nur Titel, Noten und Transport, Notenlinien größer; Esc stellt das Fenster wieder her wie vorher.

## C. Lernen und Fortschritt

**J-C1 Erste Woche** · Persona: P1 Lea · Phase 1 (adaptive Schwierigkeit Phase 2) · LH-FOR-01, LH-MOD-06, LH-BEW-04
1. An fünf Tagen je zehn Minuten üben (simuliert über Datumsfortschreibung).
2. Kontra öffnen: die Startseite nennt das zuletzt gespielte Stück, die nächste Übung, die noch nicht sitzt, und die
   Minuten der letzten sieben Tage; jedes mit einem Klick erreichbar.
3. Fortschrittsansicht öffnen.
Erwartet: Übungszeit und Verbesserung sichtbar; Formulierungen ermutigend, keine Rangliste; ohne Suchen weiterüben.
Liegt eine Übung nach dem Lauf über 90 %, bietet die Auswertung die nächste im Plan an; die Übungsliste zeigt sie mit
Häkchen.

**J-C2 Timing-Tendenz** ⚙ · Persona: P3 Mira · Phase 1 · LH-BEW-03, LH-FOR-02
1. Aufnahme mit konstanter Tendenz „18 ms zu früh“ im Song-Modus.
2. Auswertung lesen.
Erwartet: Histogramm mit Achse und Legende; Tendenz in Worten („meist 18 ms zu früh“).

**J-C3 MusicXML und Groove** · Persona: P3 Mira · Phase 2 · LH-BIB-03, LH-LRN-01, LH-LRN-04
1. MuseScore-Export (MusicXML) importieren, in der Notenansicht spielen.
2. Groove-Übung „Sechzehntel mit Ghost Notes“, 92 BPM.
3. Walking-Bass-Muster über ii-V-I in B♭ generieren lassen.
Erwartet: Notensatz wie in MuseScore; Streuung in ms; generierte Übung speicherbar.

**J-C4 Hören und nachspielen** ⚙ · Personas: P1 Lea, P3 Mira · Phase 3 · LH-LRN-03, LH-BEW-01
1. Lea: Seite „Gehör“ (Strg+9), „Nachspielen“; den Ton noch während der Vorgabe mitspielen, dann einen falschen, dann den richtigen.
2. Drei Fragen in Folge ohne Fehler; die nächste Linie hat zwei Töne. Ein Fehler, die Linie wird wieder kürzer.
3. Mira: „Intervalle“ und „Grundton“ je fünf Fragen, einmal „Nochmal hören“, „Vorspielen“ leiser stellen.
Erwartet: Töne während der Vorgabe zählen nicht; jeder gespielte Ton hat Haken oder Kreuz, ein falscher sagt „zu hoch“ oder „zu tief“; gelöst nennt Kontra, was es war (Töne, Intervall, Akkord) und zeigt, wo es am Griffbrett liegt; beim Grundton zählt jede Oktave; ohne Ausgang ein Hinweis statt Stille.

**J-C5 Techniken üben** ⚙ · Personas: P3 Mira, P4 Thomas · Phase 4 · LH-BEW-06
1. Mira: Groove aus MuseScore (MusicXML) mit Hammer-on, Pull-off, Slide, Dead Notes (x) und einer Ghost Note ( ), Song-Modus.
2. Mit den Techniken spielen, eine Dead Note aber normal anschlagen.
3. Auswertung lesen; Thomas zeigt sie am Beamer, Tab-Lane und Tabulatur.
Erwartet: jede Technik-Note hat ein Zeichen (h, p, sl, x, Klammer) und danach ein Ergebnis in Form, nicht nur Farbe; das Slide-Zeichen und sein Ergebnis stehen an der Ausgangsnote wie in gedruckter Tabulatur; die Ghost Note ( ) hat nur ihr Zeichen, kein Technik-Ergebnis; die Auswertung zählt je Technik („Dead Note: 1 von 2“), die Trefferquote bleibt davon unberührt (die normal angeschlagene Dead Note trifft, eine gedämpft gespielte auch, ohne Tonhöhe; Palm Mute hat eine Tonhöhe und wird an ihr gemessen); Hammer-on- und Pull-off-Töne sind keine zusätzlichen Noten; am Mikrofon sagt die Auswertung, dass Slap und Pop nicht bewertet werden.

## D. Inhalte teilen (ab Phase 3)

**J-D1 Original-Song als Backing Track** ⚙ · Persona: P2 Jens · Phase 3 · LH-BIB-06, LH-WIE-01, LH-WIE-02
1. MP3 an das Guitar-Pro-Arrangement koppeln, Beat-Erkennung.
2. Zwei Taktanfänge von Hand korrigieren.
3. Bei 75 % Tempo durchspielen.
4. Die Sängerin will es einen Ganzton höher: „Transponieren“ +2 mitten im Lied, weiterspielen; Kontra neu starten.
5. „Bass der Aufnahme entfernen“ einschalten (erst ohne Demucs, dann mit), nach der Trennung durchspielen.
Erwartet: kein hörbarer Versatz > 20 ms über 4 Minuten; der Wechsel hält das Lied nicht an, Bewertetes bleibt; transponiert passen Tab und Audio zusammen (gleiche Tonart, gleiche Takte), Bünde aus der Datei bleiben, wo es geht; die Einstellung bleibt beim Stück, der Fortschritt auch. Ohne Demucs ist der Schalter ausgegraut und „?“ sagt, wie man es installiert; mit Demucs läuft die Trennung im Hintergrund mit Prozent und Abbrechen, danach spielt der Backing-Track ohne Bass, im selben Abgleich, und beim nächsten Mal ohne neue Trennung.

**J-D2 Übung an Schüler geben** · Persona: P4 Thomas · Phase 3 · LH-BIB-07, LH-FOR-04
1. Übung aus Guitar Pro importieren, Loop-Bereiche und Zieltempo festlegen, als `.kontra` exportieren.
2. Auf zweitem Profil importieren.
3. Fortschrittsbericht exportieren.
Erwartet: identische Darstellung und Loop-Bereiche; Audio wird nur mitgegeben, wenn er es ausdrücklich wählt.

## E. Weitere Bass-Eingänge (ab Phase 4)

**J-E1 Synth-Bass über MIDI** ⚙ · Persona: P5 Aylin · Phase 4 · LH-EIN-12
1. MIDI-Keyboard als Eingang (Einstellungen → Anschluss „MIDI-Instrument“), Instrument „Bass über MIDI“. Zwei Keyboards angesteckt: unter „MIDI-Gerät“ das zweite wählen, Kontra neu starten, dann das zweite abziehen.
2. Funk-Groove im Song-Modus, Ansicht Klaviatur (Alt+5).
Erwartet: Timing aus MIDI-Zeitstempeln, gleiche Auswertung wie beim E-Bass. Die Wahl des Keyboards bleibt nach dem Neustart; fehlt es, nennt Anschluss das Keyboard, das Kontra stattdessen nimmt. Die Noten fallen auf die Tasten, die nächste ist auf der Klaviatur markiert; die Tasten reichen über den Umfang des Basses (4 Saiten: E1–G4), Tastennamen in der Oberflächensprache (H statt B). Der automatische Test spielt eine MIDI-Datei wie ein Keyboard ein und prüft, dass jede Note auf einer gezeigten Taste liegt.

**J-E2 Kontrabass im Wohnzimmer** ⚙ 🔍 · Persona: P6 Karl · Phase 4 · LH-EIN-11, LH-NF-03, LH-NF-06
1. USB-Mikrofon, Instrument Kontrabass, keine Kopfhörer, Backing Track über Lautsprecher.
2. Standard im Spielen-Modus, Notenansicht, Schrift 150 %.
3. Auf Mindest-Hardware.
Erwartet: Hinweis, dass die Toleranzen weiter sind; Trefferquote ≥ 90 % auf dem Testkorpus; Lane flüssig. Testkorpus: `room-double` (ohne Band, erfüllt) und `room-double-backing` (Band aus den Lautsprechern, mit Abzug erfüllt).

**J-E3 Ton schwingt ein** ⚙ · Personas: P6 Karl, P3 Mira · Phase 4 · LH-BEW-07
1. Walking-Linie im Song-Modus auf einem Kontrabass, dessen Töne zu tief beginnen und in rund 100 ms auf ihre Tonhöhe steigen.
Erwartet: Jede Note wird so schnell markiert wie sonst; eine Note, die beim Anschlag als falscher Ton gilt, wird binnen etwa 0,2 s zum Treffer. Keine zusätzlichen Noten (Hammer-on) für das Einschwingen. Der automatische Test spielt eine synthetische Linie (D2 E2 F2 G2, 140 Cent zu tief, 120 ms Anstieg).

## F. Querschnitt

**J-F1 Barrierefreiheit** 🔍 · Persona: P7 Sam · Phase 1 · LH-ANZ-06, LH-MOD-07, LH-NF-07, LH-ALL-02
1. Skalierung 200 %, Farbfilter Deuteranopie, „Bewegung reduzieren“ an.
2. J-A1 und J-B3 nur mit Tastatur, dazu Saitenwahl im Stimmgerät und Auswertung schließen (Esc).
3. Mit Orca: Bibliothek, Einstellungen und Auswertung vorlesen lassen.
Erwartet: Fokus immer sichtbar; Treffer/Fehler/früh/spät ohne Farbe unterscheidbar; alle Bedienelemente haben Namen; keine abgeschnittenen Texte; Animationen reduziert.

**J-F2 Sprache** · alle · Phase 1 · LH-ALL-01
1. System auf Englisch, J-A1 und J-B1.
2. In den Einstellungen auf Deutsch wechseln, Auswertung und Hilfe öffnen.
Erwartet: keine rohen Schlüssel, kein Sprachmix, Fachbegriffe in beiden Sprachen erklärt.

**J-F3 Störungen** · Persona: P2 Jens · Phase 1 · LH-ALL-04, LH-NF-04
1. Während des Spielens das Interface abziehen, wieder anstecken (10-mal).
2. 60 Minuten Spielen-Modus mit Time-Stretch auf Mindest-Hardware.
Erwartet: kein Absturz, klare Meldung, Interface wird wieder genutzt; Xrun-Zähler 0.
