# Kontra: Lastenheft

Version 1.0 · 9. Oktober 2026 · Grundlage: [`KONZEPT.md`](KONZEPT.md) mit den Entscheidungen aus Abschnitt 9.
Das Lastenheft beschreibt, **was** Kontra leisten muss und **wofür**, nicht wie es gebaut wird. Das Wie steht im Konzept und später im Code.

---

## 1. Einleitung

### 1.1 Zweck

Kontra ist ein freies Desktop-Programm zum Bass-Lernen und Bass-Spielen. Es hört am echten Instrument zu,
lässt Tab-Lane, Tabulatur oder Noten mitlaufen und meldet Ton und Timing sofort zurück.

### 1.2 Geltungsbereich

Dieses Lastenheft gilt bis Version 1.0. Bis dahin ist der Bass das einzige Instrument (E-Bass; ab Phase 4 auch Kontrabass und Bass über MIDI).
Andere Instrumente werden nach 1.0 in einem eigenen Lastenheft-Nachtrag beschrieben; Anforderungen, die sie heute schon vorbereiten, sind als solche markiert.

### 1.3 Begriffe

| Begriff | Bedeutung |
|---|---|
| Arrangement | Notenfolge einer Stimme mit Zeitachse, Tonhöhe, Saite und Bund, Dauer, Technik |
| Backing Track | Audiospur, zu der gespielt wird (eigene Datei oder Metronom/Drums) |
| Onset | erkannter Anschlagszeitpunkt eines Tons |
| Tab-Lane | horizontale Ansicht: Saiten als Bahnen, Noten laufen auf eine Spiellinie zu |
| Treffer | gespielter Ton mit richtiger Tonhöhe innerhalb des Zeitfensters |
| Zeitfenster | erlaubte Abweichung zwischen Soll- und Ist-Zeitpunkt (Standard ±60 ms, einstellbar) |
| Latenz | Zeit vom Anschlag bis zur Rückmeldung auf dem Bildschirm |
| Quantum | Puffergröße von PipeWire in Samples |
| Hi-Z | hochohmiger Instrumenten-Eingang eines Interfaces |

### 1.4 Prioritäten und Kennzeichnung

- **Muss:** ohne diese Anforderung kein Release der genannten Phase.
- **Soll:** gehört in die Phase; Verschieben nur mit Begründung im PR.
- **Kann:** gern, wenn Zeit ist.
- Anforderungen tragen IDs `LH-<Bereich>-<Nr>`. IDs werden nie neu vergeben; gestrichene Anforderungen bleiben mit „entfällt“ stehen.
- **Phase** bezieht sich auf die Roadmap im Konzept: 0 Machbarkeit, 1 MVP (0.1), 2 Lernen, 3 Inhalte, 4 Bass komplett (1.0).
- **Personas** P1 bis P7 aus [`PERSONAS.md`](PERSONAS.md).

---

## 2. Ziele

### 2.1 Muss-Ziele

1. Bass per Klinke (Laptop-Buchse oder Interface) anschließen und innerhalb von 60 Sekunden nach dem ersten Start den ersten Ton bewertet sehen.
2. Eigene Lieder (Guitar Pro, MIDI-Datei) mit Backing Track durchspielen, ohne Bewertung oder mit Bewertung.
3. Schwierige Stellen gezielt üben: loopen, verlangsamen, schrittweise beschleunigen.
4. Rückmeldung zu Tonhöhe und Timing pro Note, zuverlässig auch auf der tiefen B-Saite und auf bundlosen Bässen.
5. Läuft offline, ohne Konto, unter Linux mit KDE Plasma; als Flatpak installierbar.

### 2.2 Kann-Ziele (bis 1.0)

- Adaptive Schwierigkeit, Groove-, Griffbrett- und Gehörtraining, eigene Audiodateien als Backing Track, Weitergabe von Übungen, Kontrabass am Mikrofon, Bass über MIDI.

### 2.3 Abgrenzung (nicht bis 1.0)

- Andere Instrumente als Bass; mehrstimmige Erkennung über Audio.
- Online-Konten, Cloud-Sync, Song-Shop, Streaming-Dienste, Multiplayer übers Netz.
- Mitgelieferte lizenzierte Songs; Import von Rocksmith-CDLC (`.psarc`).
- Windows, macOS, Android (nach 1.0).
- 3D-Highway (nach 1.0).

---

## 3. Produkteinsatz

### 3.1 Zielgruppen

Anfänger (P1), Wiedereinsteiger (P2), Fortgeschrittene und Studierende (P3), Lehrkräfte (P4); ab Phase 4 Keyboarder mit Synth-Bass (P5) und Kontrabassisten (P6).
Barrierefreiheit gilt für alle (P7).

### 3.2 Betriebsumgebung

| | Mindestens | Empfohlen |
|---|---|---|
| Betriebssystem | Linux mit PipeWire oder PulseAudio, Flatpak | KDE Plasma 6 |
| Prozessor | 4 Kerne, x86-64 von 2014 (P6) | aktuell |
| Speicher | 4 GB | 8 GB |
| Audio-Eingang | Laptop-Kombibuchse (Mikrofonpegel) | Hi-Z-Interface oder USB-Instrumentenkabel |
| Bildschirm | 1280×720 (Beamer, P4) | 1920×1080 |

---

## 4. Funktionale Anforderungen

Spalten: **ID** · **Anforderung** · **Prio** · **Phase** · **Personas** · **Abnahmekriterium**

### 4.1 Einrichtung und Eingang (EIN)

| ID | Anforderung | Prio | Ph. | Personas | Abnahme |
|---|---|---|---|---|---|
| LH-EIN-01 | Beim ersten Start führt ein Assistent durch Instrument (Saitenzahl, Stimmung, bundlos ja/nein), Eingang und Kalibrierung. Jeder Schritt lässt sich überspringen und später wiederholen. | Muss | 1 | P1, P2, P3 | J-A1 ist in ≤ 60 s bis zum ersten bewerteten Ton durchspielbar. |
| LH-EIN-02 | Kontra listet alle Audio-Eingänge mit verständlichem Namen und wählt einen sinnvollen vor (Interface vor interner Buchse). Unter PipeWire sind das die Quellen-Knoten mit ihrem Namen (keine Monitore, keine Streams anderer Programme), Interfaces zuerst; jedes Gerät ist wählbar, auch wenn das System einen anderen Standard hat. | Muss | 1 | P1, P2, P3 | Mit Scarlett Solo und interner Soundkarte ist das Interface vorgewählt; die automatische Wahl fällt bei jedem Start neu. Interface wählbar, während der Systemstandard die interne Karte ist; die Wahl bleibt nach Neustart. |
| LH-EIN-03 | Pegelmessung mit Anzeige (zu leise, gut, übersteuert) und konkretem Hinweis, was zu tun ist (Gain, Lautstärkeregler am Bass). | Muss | 1 | P1 | Übersteuertes Testsignal führt zu Warnung mit Handlungsanweisung. |
| LH-EIN-04 | Erkennt Kontra ein schwaches oder verrauschtes Signal (typisch Laptop-Kombibuchse), erklärt es den Grund in einem Satz und empfiehlt Hi-Z-Interface oder USB-Kabel, ohne die Nutzung zu verweigern. | Muss | 1 | P1 | Testaufnahme aus der Laptop-Buchse: Hinweis erscheint, Spielen bleibt möglich. |
| LH-EIN-05 | Latenz-Kalibrierung: „Mit Kabel messen“ (Loopback, Ausgang per Kabel in den Eingang, zwölf Töne) misst die Systemlatenz und wird abgezogen. „Auf den Klick spielen“ (8 Anschläge) misst danach die **Neigung** des Spielers (Klick-Wert minus Kabel-Wert, „deine Neigung: −14 ms“) und zeigt sie, statt sie abzuziehen. Ohne Kabel (Laptop-Buchse: Ein- und Ausgang in einer Buchse, der Knopf fehlt dort) bleibt der Klick-Wert die Latenz und ist als „enthält deine Neigung“ markiert. Bei MIDI fehlt der Knopf ebenso, je mit einem Satz warum. Beide Werte bleiben in den Einstellungen mit den Geräten, an denen sie gemessen wurden (wie aufgelöst, also auch bei automatischer Wahl, Ersatzgerät oder Systemstandard). Löst ein Start oder eine Messung andere Geräte auf, oder wechseln Anschluss oder Gerätewahl, verwirft Kontra beide (die Latenz bleibt als von Hand eingetragener Startwert, ohne Neigung; LH-EIN-14 gilt wieder); eine Neigung zeigt nur ein Klick nach dem Kabel-Wert; Herkunft und Neigung stehen in Worten neben dem Wert. Kommen zu wenige Töne zurück, sagt Kontra das und nennt den Weg ohne Kabel (Zeitachsen-Bericht R4, 29.10.). | Muss | 1 | P3, P2, P1, P7 | Künstlich verzögertes Testsignal (+40 ms) wird nach Kalibrierung als pünktlich bewertet (±5 ms). Test: Kabel 20 ms, Spieler 15 ms früh → Latenz bleibt 20 ms, Neigung −15 ms, ein Lied mit derselben Neigung und Ziel „Auf den Punkt“ ist „verfehlt“ (vorher „gehalten“). Ohne Kabel: Hinweis, Latenz unverändert. |
| LH-EIN-06 | Warnung bei Bluetooth-Ausgabe (hohe, schwankende Latenz) mit Empfehlung für Kabel-Kopfhörer. | Soll | 1 | P1 | Bluetooth-Senke ausgewählt → Hinweis. |
| LH-EIN-07 | Hinweis, wenn das PipeWire-Quantum die Latenz-Ziele verhindert, mit Erklärung, wie man es ändert. | Soll | 1 | P3 | Quantum 2048 → Hinweis mit Wert und Anleitung. |
| LH-EIN-08 | Monitoring des eigenen Instruments über Kontra an/aus, mit einfacher Klangformung (EQ, Kompressor, leichte Sättigung). | Soll | 1 | P1, P2 | Ein/Aus wirkt sofort, kein Knacken. |
| LH-EIN-09 | Instrument und Eingang sind getrennte, austauschbare Profile (Datenmodell und Oberfläche), damit spätere Instrumente und Eingänge kein Umbau sind. Einrichtung und Einstellungen fragen das Instrument eigens: E-Bass, Kontrabass oder Bass über MIDI; „Bass über MIDI“ ist nur in der Oberfläche eine Wahl und setzt das Bass-Profil mit dem MIDI-Eingang, das Datenmodell bleibt getrennt. Bundlos und Stimmung nur, wo sie gelten (Runde 15). | Muss | 1 | P5, P6 | Architektur-Review: neues Profil ohne Änderung an Session und Bewertung möglich (Test mit Dummy-Profil). |
| LH-EIN-10 | Nicht unterstützte Eingänge (MIDI, Mikrofon vor Phase 4) werden in der Auswahl als „kommt später“ gezeigt, nicht stillschweigend weggelassen oder als defekt gemeldet. | Soll | 1 | P5, P6 | MIDI-Keyboard angesteckt → klare Meldung. |
| LH-EIN-11 | Mikrofon-Eingang mit Rauschsperre und Abzug des eigenen Backing Tracks; Toleranzen werden weiter und die Oberfläche sagt das. | Muss | 4 | P6 | J-E2 ohne Kopfhörer: Trefferquote ≥ 90 % auf dem Testkorpus „Kontrabass Raum“. |
| LH-EIN-12 | MIDI-Eingang (USB, ALSA-Seq) als Bass-Eingang: Note-On/Off werden wie erkannte Töne bewertet. Bei mehreren Geräten Auswahl unter Anschluss, die Wahl bleibt gespeichert; fehlt das gewählte, nimmt Kontra das erste und sagt es (Runde 9). | Muss | 4 | P5 | J-E1 durchspielbar, Timing aus MIDI-Zeitstempeln. |
| LH-EIN-13 | Am Ende der Einrichtung fragt Kontra nach dem Stand: „Neu am Bass“ führt zu Lektion 1, „Ich spiele schon“ zu den Übungen der passenden Stufe, „Eigene Songs“ in die Bibliothek (Runde 15). | Soll | 1 | P1, P3, P2 | J-A1: Lea landet in Lektion 1, Mira bei den Übungen, Jens („Eigene Songs“) in der Bibliothek. |
| LH-EIN-14 | Hinweis „Neu kalibrieren?“ in der Auswertung, nur wenn mindestens 8 Treffer, mindestens 60 % davon auf derselben Seite und das Mittel mindestens 5 ms vom Schlag; mit Knopf zur Kalibrierung, nie als automatische Korrektur (bewusst hinter dem Schlag spielen bleibt sichtbar; Harmonicon-Analyse, 29.10.). Nur wenn die Latenz nicht mit Kabel gemessen ist: ein Kabel-Wert enthält keine Neigung, eine beständige Neigung im Lauf ist dann die des Spielers (LH-BEW-09), kein Latenzfehler (R4, 29.10.). | Soll | 1 | P2, P3 | Test: 10 Treffer 25 ms spät → Hinweis; symmetrische Streuung mit Mittel 8 ms → kein Hinweis; Latenz mit Kabel gemessen, 15 ms früh → kein Hinweis, Neigung „vor dem Schlag“ als Beobachtung. |
| LH-EIN-15 | Eingangszustand dauerhaft sichtbar und angesagt (verbunden, kein Signal vom Gerät, Fehler, Berechtigung fehlt) mit „Erneut versuchen“. Liefert der Eingang 250 ms lang keine Daten, meldet die Engine das; die Bewertung pausiert, fehlende Daten zählen nicht als Fehler (Harmonicon-Analyse, 29.10.). | Soll | 1 | P6, P1, P7 | Test: Eingang hält an → Zustand „kein Signal“, keine Noten als verpasst gewertet; Screenreader sagt den Zustand. |
| LH-EIN-16 | Die Kalibrierung misst beim Einzählen den Rauschboden im Anschlagband (niedriges Perzentil) und setzt daraus die Anschlag-Schwelle, begrenzt auf sinnvolle Werte; bei Gerätewechsel neu (Harmonicon-Analyse, 29.10.). | Soll | 1 | P6, P1 | Test: Rauschen −50 dBFS → Schwelle darüber, keine Falschnoten im Leerlauf. |

### 4.2 Stimmgerät (TUN)

| ID | Anforderung | Prio | Ph. | Personas | Abnahme |
|---|---|---|---|---|---|
| LH-TUN-01 | Chromatisches Stimmgerät mit Tonname, Saite, Abweichung in Cent; Genauigkeit ±1 Cent ab B0 (30,9 Hz). | Muss | 1 | P1, P3 | Testtöne 30,9 Hz bis 400 Hz: Abweichung ≤ 1 Cent. |
| LH-TUN-02 | Stimmungen: Standard (4/5/6 Saiten), Drop D, Halbton und Ganzton tiefer, frei definierbar. | Muss | 1 | P2, P3 | Eigene Stimmung anlegen, wird im Stimmgerät und in Tabs verwendet. |
| LH-TUN-03 | Geführtes Stimmen für Anfänger: Saite für Saite, „höher/tiefer drehen“ in Worten und Symbol. | Muss | 1 | P1 | J-A2 ohne Vorwissen schaffbar. |
| LH-TUN-04 | Referenzton A ist einstellbar (Standard 440 Hz). | Kann | 2 | P3 | 442 Hz wirkt auf Stimmgerät und Bewertung. |
| LH-TUN-05 | Liegt eine Saite im Spiel gleichmäßig daneben (≥ 8 Treffer auf ≥ 3 Bünden, im Schnitt ≥ 12 Cent, die anderen Saiten nicht), fragt die Auswertung „Die A-Saite klingt etwa 15 Cent zu hoch – nachstimmen?“ mit Sprung ins Stimmgerät. Nie bei bundlosen Instrumenten (Harmonicon-Analyse, 29.10.). | Soll | 2 | P1, P2 | Test: A-Saite +15 Cent → Hinweis; bundlos → keiner. |

### 4.3 Bibliothek und Import (BIB)

| ID | Anforderung | Prio | Ph. | Personas | Abnahme |
|---|---|---|---|---|---|
| LH-BIB-01 | Import Guitar Pro 3, 4, 5 (`.gp3`–`.gp5`) und 6/7 (`.gpx`, `.gp`); Auswahl der Bassspur; Takt, Tempo, Tempowechsel, Wiederholungen, Saite/Bund, Dauern. | Muss | 1 | P2, P4 | Testsammlung von 20 Dateien: alle Bassspuren ohne Fehler, Noten stimmen mit Guitar-Pro-Export überein. |
| LH-BIB-02 | Import MIDI-Dateien; Bassspur auswählbar; Tab wird automatisch erzeugt (Lagen-Optimierung, minimale Lagenwechsel). | Muss | 1 | P2, P5 | MIDI-Basslinie ergibt spielbare Tab ohne Sprünge > 5 Bünde innerhalb einer Phrase, wo vermeidbar. |
| LH-BIB-03 | Import MusicXML (Bassschlüssel, mit oder ohne Tab-Information). | Muss | 2 | P3 | MuseScore-Export eines Walking-Bass-Chorus wird korrekt dargestellt. |
| LH-BIB-04 | Bibliothek mit Suche (Titel, Interpret, Tag), Sortierung (zuletzt gespielt, Fortschritt), Setlisten. | Muss | 1 | P2 | 200 Lieder: Suche antwortet < 100 ms. |
| LH-BIB-05 | Fehlerhafte oder nicht unterstützte Dateien werden mit Grund gemeldet; der Rest des Imports läuft weiter. | Muss | 1 | P2, P4 | Ordner mit einer defekten Datei: 1 Meldung, alle anderen importiert. |
| LH-BIB-06 | Eigene Audiodatei (MP3, FLAC, OGG, WAV) an ein Arrangement koppeln; Tempo-Map automatisch (Beat-Erkennung), von Hand korrigierbar (Taktanfänge verschieben). | Muss | 3 | P2 | Gekoppelter Song läuft über 4 Minuten ohne hörbaren Versatz > 20 ms. |
| LH-BIB-07 | Offenes Paketformat `.kontra` (Arrangement, Verweis auf oder Einbettung der Audiodatei); Import und Export. Es trägt die Übe-Abschnitte (Loops) und das Zieltempo, nicht den Fortschritt der Lehrkraft; es kann echte Backing Tracks für Übungen und Stücke enthalten, mit ihrem Abgleich (Runde 15). | Muss | 3 | P4 | Paket auf zweitem Rechner importiert: identische Darstellung, Loops und Zieltempo; Begleitspur spielt ohne neuen Abgleich; Fortschritt beginnt bei null. |
| LH-BIB-08 | Mitgelieferte, frei lizenzierte Übungen und Grooves (mind. 30 bis 1.0), nach Niveau sortiert. | Soll | 3 | P1 | Liste vorhanden, Lizenz je Übung dokumentiert; Titel und Ziel der Übung in der Oberflächensprache. |
| LH-BIB-09 | Optionale lokale Stem-Trennung, um den Bass aus einer Audiodatei zu entfernen: über ein externes Programm (Demucs), wenn installiert; Kontra liefert nichts davon mit (Runde 9). Ohne das Programm ist die Funktion ausgegraut und sagt, wie man es installiert. Läuft im Hintergrund mit Fortschritt, Restzeit und Abbrechen, Ergebnis wird im Cache-Ordner zwischengespeichert und ist je Song löschbar (14.10.); Einstellungen → Daten zeigt es mit den vorbereiteten Backing-Tracks, je Eintrag mit Größe und löschbar, dazu „alles leeren“ (Runde 16). Im Mixer bringt „Original-Bass“ den entfernten Bass stufenlos zurück, auch beim Spielen (0 % = entfernt). | Kann | 4 | P2 | 4-Minuten-Song wird auf Empfehlungs-Hardware in < 3 min getrennt. Einstellungen → Deine Daten zeigt Backing-Cache und getrennte Songs je Eintrag mit Song-Namen und Größe; ein gelöschter Eintrag verschwindet und die Summe sinkt; „Alles leeren“ leert den jeweiligen Cache samt halb geschriebener Reste. |
| LH-BIB-10 | Import MuseScore 3 und 4 (`.mscz`, `.mscx`) direkt, ohne Export nach MusicXML: Stimmen, Tab-Saiten/Bünde, Wiederholungen, Volten, Triolen, Haltebögen, Tempo. | Soll | 2 | P3, P4 | Mit MuseScore 4 gespeicherter Fünfsaiter-Tab mit Wiederholung und Triole: gleiche Noten und Bünde wie in MuseScore. |
| LH-BIB-11 | Songs aus dem Netz, soweit die Seiten es erlauben ([`QUELLEN-SONGS.md`](QUELLEN-SONGS.md)): „Suchen“ fragt Songsterr aus Kontra heraus (Runde 12: nur auf Wunsch, eine Anfrage zur Zeit, kein Download) und zeigt Titel, Interpret und Bassspur mit Stimmung als Text; ein Treffer öffnet die Seite im Browser. Suche bei Songsterr und MuseScore.com auch im Browser; Dateien, die der Nutzer dort selbst herunterlädt, zeigt die Bibliothek unter „Neu in Downloads“ (14 Tage, ein Klick öffnet); Dateien lassen sich auf die Bibliothek ziehen. Ohne Netz nennt Kontra den Grund. | Soll | 2 | P2, P3, P1, P7 | J-B6: Treffer mit „Bass: E A D G“ bzw. „ohne Bassspur“; heruntergeladene Datei erscheint beim Zurückkehren zu Kontra und öffnet mit einem Klick; offline Meldung mit Grund, Bibliothek bleibt bedienbar. |
| LH-BIB-12 | **Stücke zum Durchspielen** (24.10.): Der Generator schreibt ganze Stücke mit Form in fünf Stilen: Pop, Rock und Funk mit Intro, Strophe, Refrain, Bridge und Outro, die Ballade ohne Bridge, der Shuffle als Intro, vier Blues-Chorusse und Outro; je Teil eigene Akkordfolge und eigener Groove (Shuffle: durchgehend Blues), Pop und Ballade in Dreiklängen, Rock in Powerchords, Septimen bei Funk und Shuffle (Runde 16), jeder Teil mit Probenzeichen über dem ersten Takt (Name, mehrere Marker eines Takts zusammengefasst), Schlagzeug mit einem Fill vor jedem neuen Teil, Schluss auf der Tonika; in wählbarer Tonart und Tempo; bei den mitgelieferten Tempi (70–110 BPM je nach Stil) zwei bis drei Minuten lang. Mitgeliefert als Stufe „9 Stücke“. | Soll | 3 | P1, P2, P4 | J-C7: Stück erzeugt, im Durchlauf ganz gespielt; Akkordnamen über jedem Takt (Pop und Ballade ohne Septakkorde), Probenzeichen über dem ersten Takt jedes Teils, Schlagzeug hörbar, ein Teil (Refrain, beim Shuffle ein Chorus) in der Werkstatt übbar. |
| LH-BIB-13 | **Schwierigkeit der Stücke** (24.10.): Der Stück-Generator bietet leicht, mittel und schwer. Leicht spielt die Basslinie jedes Teils einfacher (Viertel statt Achtel, Halbe statt Viertel, ohne Synkopen und Ghost Notes), schwer dichter (Sechzehntel, Offbeats, Funk-Linie, Triolen im Shuffle); Form, Akkorde und Schlagzeug bleiben. Mittel ist das bisherige Stück. Jede Stufe hat ihren eigenen Titel und Fortschritt. | Soll | 3 | P1, P2, P3, P4 | J-C7: leicht hat weniger Töne als mittel, schwer mehr; Titel „Stück Rock (leicht)“ / „Rock piece (easy)“. |

### 4.4 Darstellung (ANZ)

| ID | Anforderung | Prio | Ph. | Personas | Abnahme |
|---|---|---|---|---|---|
| LH-ANZ-01 | Tab-Lane als Standardansicht: Saitenbahnen, Bundzahl auf der Note, Länge = Dauer, Spiellinie, Vorschau der nächsten Takte. | Muss | 1 | P1, P2 | J-B1 vollständig in der Tab-Lane spielbar. |
| LH-ANZ-02 | Klassische Tabulatur mit Takten, Rhythmus und Wiederholungen, mitlaufend. | Muss | 1 | P2, P4 | Umschalten während des Spiels ohne Unterbrechung. |
| LH-ANZ-03 | Notenansicht im Bassschlüssel (einstimmig), mitlaufend, mit Vorzeichen, Pausen, Bindebögen. | Muss | 1 | P3, P6 | 10 Referenzstücke werden ohne Notensatzfehler dargestellt (Sichtprüfung gegen MuseScore). |
| LH-ANZ-04 | Kombinierte Ansicht (Noten über Tab oder Tab-Lane). | Soll | 1 | P3, P4 | Wählbar und gespeichert pro Lied. |
| LH-ANZ-05 | Griffbrett-Ansicht unter der Lane: nächste Note, Lage, optional Fingersatz. | Soll | 1 | P1 | Zeigt bei Lagenwechsel die neue Lage ≥ 1 Takt vorher. |
| LH-ANZ-06 | Saitenfarben einstellbar; Bedeutung nie nur über Farbe (zusätzlich Form, Symbol oder Text). | Muss | 1 | P7 | Graustufen-Screenshot: Treffer, Fehler, zu früh, zu spät unterscheidbar. |
| LH-ANZ-07 | Saitenreihenfolge umschaltbar (tiefe Saite oben oder unten). | Soll | 1 | P2, P3 | Einstellung wirkt in Lane, Tab und Griffbrett gleich. |
| LH-ANZ-08 | Linkshänder-Darstellung für Griffbrett. | Kann | 2 | – | Spiegelung korrekt. |
| LH-ANZ-09 | Präsentationsmodus: große Darstellung, ruhige Oberfläche, für Beamer. | Soll | 2 | P4 | Bei 1280×720 und 150 % Schrift keine abgeschnittenen Elemente. |
| LH-ANZ-10 | 3D-Highway. | entfällt bis 1.0 | – | – | – |
| LH-ANZ-11 | Einstellung „Einfache Ansicht“: blendet auf der Übungsseite die seltenen Regler aus (Transponieren, Timing-Ziel, Notendichte, Bausteine, Backing, Präsentation); die Einrichtung schaltet sie für „Neu am Bass“ ein. Das Ziel der Übung steht als Zeile unter dem Titel (Runde 15). | Soll | 1 | P1, P6 | J-A2: Lea sieht Modus, Tempo, Metronom und das Ziel. |

### 4.5 Modi (MOD)

| ID | Anforderung | Prio | Ph. | Personas | Abnahme |
|---|---|---|---|---|---|
| LH-MOD-01 | **Frei spielen** (Runde 11, vorher „Spielen“): Lied läuft durch, nichts hält an, keine Punkte; Treffer dezent markiert, abschaltbar. Die Auswertung gibt es am Ende nur auf Wunsch. | Muss | 1 | P2, P1 | J-B1: Lied von Anfang bis Ende ohne Unterbrechung oder Wertungsbildschirm. |
| LH-MOD-02 | **Durchlauf** (Runde 11, vorher „Song“): wie frei spielen, mit Bewertung und Auswertung am Ende. | Muss | 1 | P2, P3 | Auswertung zeigt Trefferquote, Timing-Tendenz, schwierigste Takte. |
| LH-MOD-03 | **Warten:** Lied hält vor jeder Note, bis der richtige Ton gespielt ist; Backing Track pausiert mit. | Muss | 1 | P1 | J-B2 ohne Zeitdruck schaffbar. |
| LH-MOD-04 | **Riff-Werkstatt:** Bereich wählen (Takte oder Markierungen), Loop, Tempo 25 bis 150 % in 5-%-Schritten, automatische Anpassung an ein Zielband der Trefferquote (Standard 80–90 %: zwei Durchläufe darüber +5 %, zwei darunter −5 %, dazwischen gleich; Runde 5), Vorzähler. | Muss | 1 | P2, P3 | J-B3: Bereich, Tempo und Steigerung wirken wie eingestellt. |
| LH-MOD-05 | Metronom und Vorzähler in allen Modi, Lautstärke getrennt regelbar (Mixer, LH-WIE-03). | Muss | 1 | P2, P3 | Mixer: Instrument, Backing Track, Metronom getrennt. |
| LH-MOD-06 | Adaptive Schwierigkeit: Notendichte steigt mit der Trefferquote, sinkt bei Häufung von Fehlern; nur zwischen Durchläufen, sichtbar und als Vorschlag („Nächster Durchlauf mit mehr Noten?“), nie mitten im Lied (Runde 5); abschaltbar und pro Abschnitt fixierbar. Methode (Harmonicon-Analyse, 29.10.): je Abschnitt ein gespielter Anteil der Noten, ein sauberer Durchlauf erhöht ihn. | Soll | 2 | P1 | Testspieler mit 95 % Treffern erreicht volle Notendichte in ≤ 5 Durchläufen. |
| LH-MOD-07 | Tastenkürzel für alle Transportfunktionen (Start/Stop, Loop setzen, Tempo ±, zurück zum Loop-Anfang); optional Fußschalter über Tastatur-Emulation. | Muss | 1 | P3, P7 | Alle Funktionen ohne Maus erreichbar. |
| LH-MOD-08 | **Bausteine** (DIDAKTIK I6, 24.10.): In der Werkstatt teilt „Bausteine“ die gewählten Takte (oder das ganze Stück, wenn sie nur einen Baustein ergeben) in Bausteine zu 2 oder 4 Takten oder nach Phrasen (seit 25.10.: Schnitt an Taktgrenzen mit mindestens einer Viertel Pause, 2 bis 4 Takte je Baustein, ohne Pausen je 4 Takte; der letzte Baustein darf kürzer sein) und übt sie in der Folge 1 · 2 · 1–2 · 3 · 1–3 …; „Von hinten“ beginnt mit dem letzten Baustein. Ein Schritt sitzt nach zwei Durchgängen in Folge mindestens an der Untergrenze des Zielbands; dann endet sein Lauf und der nächste beginnt mit Einzähler. Die Zeile nennt Takte und Schritt. Andere Größe oder Reihenfolge teilt wieder die anfangs gewählten Takte, „Aus“ kehrt zu ihnen zurück; von Hand gewählte Takte beenden die Bausteine. | Soll | 2 | P1, P2, P4 | J-C9: fünf Schritte über drei Takte, jeder sitzende Schritt ein gespeicherter Lauf; ein Groove mit Pausen ergibt Phrasen zu 2 Takten. |

### 4.6 Bewertung (BEW)

| ID | Anforderung | Prio | Ph. | Personas | Abnahme |
|---|---|---|---|---|---|
| LH-BEW-01 | Pro Note: Treffer/daneben/ausgelassen, Timing-Abweichung in ms (früh/spät), Tonhöhen-Abweichung in Cent. | Muss | 1 | P3 | Werte in der Auswertung pro Note abrufbar. |
| LH-BEW-02 | Zeitfenster in Stufen (locker, normal, streng) und frei in ms einstellbar. | Muss | 1 | P1, P3 | Stufe „streng“ = ±25 ms. |
| LH-BEW-03 | Auswertung nach dem Durchlauf: Trefferquote, Timing-Histogramm mit Achse und Legende, Tendenz („meist 18 ms zu früh“), schwierigste Takte als Sprung in die Riff-Werkstatt. Schwierigste Takte nach Zahl der Fehler (Quote nur bei Gleichstand); „Stelle üben“ nimmt das dichteste 2-Takt-Fenster der Fehler mit einem Schlag Vorlauf (Harmonicon-Analyse, 29.10.). | Muss | 1 | P2, P3 | Klick auf schwierigsten Takt öffnet Werkstatt mit diesem Bereich. |
| LH-BEW-04 | Ton der Rückmeldung ermutigend; keine Strafen, keine Ranglisten. Punkte und Streaks abschaltbar. | Muss | 1 | P1 | Text-Review aller Meldungen gegen die Sprachregeln. |
| LH-BEW-05 | Bundlos-Modus: Intonation in Cent sichtbar, Treffergrenze für Tonhöhe einstellbar (Standard ±30 Cent, streng ±10). Der richtige Ton außerhalb der Grenze ist „unsauber“ (eigener Zustand mit eigener Form, nicht „falsch“). | Muss | 1 | P3 | Testkorpus bundlos: Anzeige weicht ≤ 3 Cent vom Referenzwert ab. |
| LH-BEW-06 | Techniken erkennen: Slides, Hammer-on/Pull-off, Slap/Pop, Ghost Notes. Soll-Technik aus der Partitur wird mit der erkannten verglichen und in einer eigenen Auswertung gezählt („Slide: 8 von 10“), getrennt von der Trefferquote (Runde 9); Zeichen und Ergebnis an der Note in Tab-Lane, Tabulatur und Noten, ein Slide an der Ausgangsnote (Runde 10). Dead Notes (x) werden als gedämpft bewertet, Ghost Notes ( ) nur angezeigt; Palm Mute behält die Tonhöhe (Runde 10). Bis zwei Bünde zählen Slide und Hammer-on/Pull-off füreinander (Runde 11: über einen nicht unterscheidbar; Runde 12: echte Zwei-Bund-Slides als Hammer-on gehört). Was der Eingang nicht erkennen kann (Slap/Pop am Mikrofon, alles über MIDI), wird nicht bewertet und so genannt. | Soll | 4 | P2, P3, P4 | Testkorpus Techniken: ≥ 85 % korrekt klassifiziert; J-C5. Echte Aufnahmen gemessen, noch ohne Ziel (`korpus techniques`, je Bass): IDMT-Linien Slap 80 %, Pop 66 %, Dead Note 96 %; IDMT-Einzelnoten dreier Bässe Slap 19–67 %, Pop 1–30 %, Dead Note 95–99 %; Palm Mute, Vibrato, Bending bleiben normal (≥ 99 %), normale Noten als Technik ≤ 2 %. |
| LH-BEW-07 | Tonhöhe nach dem Einschwingen (Runde 11): Die Note wird so schnell angezeigt wie bisher; beginnt der Ton tiefer und steigt binnen rund 100 ms auf seine Tonhöhe (Kontrabass, Anschlag mit Anteil eine Oktave tiefer), wird die Note danach mit der eingeschwungenen Tonhöhe neu bewertet. Ihre Markierung darf sich kurz nach dem Anschlag ändern. Nur nach oben (ein Halbton, eine Oktave; am Mikrofon nur ein Halbton). | Soll | 4 | P6, P3 | FiloBass: Halbton-Fehler deutlich weniger, keine richtige Note kippt; IDMT und eigene Aufnahmen nicht schlechter; J-E3. |
| LH-BEW-08 | **Timing-Ziel** (DIDAKTIK I5, 24.10.; in der Oberfläche „Timing“, Runde 15, weil „Lage“ die Handposition ist): Auf der Übungsseite wählbar „Frei“, „Auf den Punkt“ (±5 ms), „Laid-back“ (10 bis 30 ms hinter dem Schlag) oder „Vorn“ (10 bis 30 ms davor). Die Auswertung sagt in einem Satz, ob das mittlere Timing im Ziel lag, und sonst, um wie viele ms zu früh oder zu spät. | Soll | 2 | P3, P2, P5 | J-C2: Ziel laid-back, Aufnahme 18 ms zu früh → „verfehlt, 28 ms zu früh“. |
| LH-BEW-09 | **Eine Beobachtung je Durchlauf:** Die Auswertung nennt genau einen Hinweis in fester Rangfolge: eine Technik deutlich unter den normalen Noten, dann Noten neben dem Zeitfenster, dann viele ausgelassene Noten, dann ein verfehltes Timing-Ziel, dann Neigung früh oder spät (nur ohne Ziel), sonst „solide“. Ist der Hinweis „Neu kalibrieren?“ (LH-EIN-14) fällig (nie bei einer mit Kabel gemessenen Latenz), folgt er der Beobachtung als zweiter Satz und ersetzt „solide“. Jede Aussage erst ab einer Mindestzahl Noten (8 im Durchlauf, 4 je Technik; zum Timing 8 Treffer, darunter keine), damit ein einzelner Fehlversuch keine Diagnose wird (Harmonicon-Analyse, 29.10.). | Soll | 2 | P1, P6, P7 | Test: 1 von 1 Slides verfehlt → kein Technik-Hinweis; 2 von 6 bei 90 % normalen Noten → Hinweis Slide. |
| LH-BEW-10 | **Notenende:** Kontra erkennt, wann eine Note endet (abgedämpft, ausgeklungen, durch die nächste ersetzt); Übungen können Abdämpfen und Notenlänge (Staccato, Legato) bewerten, nur aktives Abdämpfen gilt als verlässlich (Harmonicon-Analyse, 29.10.). | Kann | 4 | P1, P3, P6 | IDMT-Offsets: Ende abgedämpfter Noten ≤ 30 ms genau. |
| LH-BEW-11 | **Mehrere Töne gleichzeitig:** Hört der einstimmige Audio-Eingang einen Doppelgriff, sagt Kontra „zwei Töne gehört“, statt einen erfundenen Ton als falsch zu werten. Jeder Eingang meldet, ob er mehrere Töne erkennen kann (Audio heute nein, MIDI ja); Akkord-Übungen werden nur dort bewertet. Offen für Gitarre und Tasten (LH-NF-14; Harmonicon-Analyse, 29.10.). | Soll | 4 | P3, P2, P5 | Korpus „Doppelgriffe“: kein erfundener Ton als Treffer oder Fehler gewertet. |
| LH-BEW-12 | **Intonation als Richtung und Streuung:** Auswertung „im Schnitt 9 Cent zu tief, Streuung ±6“, je Saite und Lage ab 8 Noten; keine Gesamtnote für Intonation. Später Absinken beim Aushalten und Vibrato (Harmonicon-Analyse, 29.10.). | Soll | 2 | P3, P6, P4 | Test: Noten konstant −10 Cent → „zu tief“; ±10 Cent gestreut um 0 → nur Streuung. |
| LH-BEW-13 | **Improvisation bewerten:** In offenen Lektionen zählt jeder Anschlag als Akkordton, Skalenton oder fremder Ton zum Akkord des Takts; Anteil passender Töne und Anteil Akkordtöne, optional Pausenfenster (Phrasen mit Luft). Der Jam-Modus bleibt ohne Punkte (Harmonicon-Analyse, 29.10.). | Soll | 3 | P1, P3, P4 | Test: Walking Bass nur aus Akkordtönen → 100 % Akkordtöne. |
| LH-BEW-14 | **Erwartete Note als Torwächter (R2):** Die Session nennt der Erkennung die Anschläge der Tab im Voraus (Zeitfenster auf der Uhr des Eingangs, Tonhöhen, Saiten wo bekannt; ein Doppelgriff hat zwei Töne). Stimmt die eigene Schätzung der Erkennung zweimal in Folge mit einem erwarteten Ton überein, meldet sie die Note früher; die Schätzung über das volle Fenster hat das letzte Wort und korrigiert sonst. Eine falsche Erwartung macht keine Note zum Treffer. Grundlage für Saite (R7) und Doppelgriff (R8). Nicht im Warte-Modus (dort hält die Uhr bis zum Anschlag). | Soll | 1 | P1, P2, P3, P6 | `kontra-cli korpus measure --expect` auf IDMT und FiloBass: Endgenauigkeit ±0,3 Punkte wie ohne Erwartung, erste Meldung nicht seltener richtig; mit `--expect-shift` ±1, ±12, +5 nicht mehr Noten auf der verschobenen Tonhöhe als ohne Erwartung (Test `wrong_expectation_changes_nothing`). |

### 4.7 Lernmodule (LRN)

| ID | Anforderung | Prio | Ph. | Personas | Abnahme |
|---|---|---|---|---|---|
| LH-LRN-01 | **Groove:** Rhythmusübungen auf Drum-Patterns (Viertel, Achtel, Sechzehntel, Pausen, Shuffle, Ghost Notes; seit 14.10. auch Dead Notes, Offbeat, Tresillo, Triolen, Funk, Slap und die Taktarten 3/4, 6/8, 5/4, 7/8), Auswertung als Timing-Streuung. | Muss | 2 | P2, P3, P5 | 20 Übungen vorhanden; Streuung in ms sichtbar. |
| LH-LRN-02 | **Griffbrett:** Töne finden, Intervalle, Tonleitern und Arpeggien in Lagen, Quiz mit dem echten Instrument. | Muss | 2 | P1, P3 | Quiz „alle C auf dem Griffbrett“ funktioniert mit 4 und 5 Saiten. |
| LH-LRN-03 | **Gehör:** Ton oder kurze Linie hören und nachspielen; Intervalle; Grundton zu einem Akkord. | Soll | 3 | P1, P3 | 3 Übungstypen, Bewertung wie LH-BEW-01. |
| LH-LRN-04 | Generierte Übungen (Tonleitern und Modi, auch in Terzen und Vierergruppen; Dreiklänge, Arpeggien, Walking-Bass-Muster über Akkordfolgen; Fingerübungen ab einem Bund) in wählbarer Tonart und Tempo. | Soll | 2 | P1, P3, P4 | Tonart und Tempo wirken; Ergebnis als Arrangement speicherbar. |
| LH-LRN-05 | **Ohne Hilfe prüfen** (DIDAKTIK I1, 24.10.): In der Werkstatt spielt „Ohne Hilfe prüfen“ den Abschnitt einmal ohne Noten auf dem Bildschirm (nur Takte und Cursor, mindestens ein Takt Vorzähler), im zuletzt geübten Werkstatt-Tempo, und endet nach einem Durchgang. Die Auswertung setzt das Ergebnis neben den letzten Werkstatt-Lauf derselben Takte (Trefferquote und Streuung, ein Satz dazu: „12 Punkte weniger als mit Noten“ oder „sitzt“); der Lauf wird als eigener Typ gespeichert. Die Startseite bietet einen Abschnitt an, der vor mindestens einer Nacht in den letzten drei Tagen geübt und seitdem nicht geprüft wurde. | Soll | 2 | P2, P1, P4 | J-C6: Probe endet nach einem Durchgang, Vergleich stimmt mit beiden Läufen überein. |
| LH-LRN-06 | **Erst der Puls** (DIDAKTIK I4, 24.10.): Die ersten Grooves der Stufe 1 (leere E-, A-, D-Saite; Viertel, Achtel, Pausen) bewerten nur den Anschlag, nicht die Tonhöhe; ihr Ziel sagt das. Die Übungsdatei trägt die Bewertungsart, sie gilt auch nach dem Import an anderer Stelle. Seit 24.10. bietet der Generator bei Grooves „Bewertung: Ton und Timing / Nur Timing“; der Titel sagt „nur Timing“, so hat die Übung eigenen Namen und Fortschritt. Seit 25.10. sitzt eine solche Übung erst mit mindestens 90 % Treffern und einer Streuung von höchstens ±25 ms; die Auswertung nennt das Ziel („Streuung ±31 ms · Bünde ab ±25 ms“), erst dann bietet Kontra die nächste Übung an. | Soll | 2 | P1, P5, P2 | J-C8: falscher Ton zur rechten Zeit trifft, in einer normalen Übung nicht; im Generator wählbar. |
| LH-LRN-07 | **Lernen** (Runde 13, `LERNEN.md`): Kurs mit Kapiteln und Lektionen, Fortschritt je Lektion und Kapitel, „Weiter“ auf der Startseite; Reihenfolge empfohlen, nichts gesperrt; keine Abzeichen. | Soll | 2 | P1, P2 | J-G1 |
| LH-LRN-08 | Lektion in Schritten Erklären (Text, Abbildung, Hörbeispiel; Videos nur in nachgeladenen Kursen, LH-LRN-14), Ausprobieren (am Bass, ohne Eingang per Auswahl), Üben (vorhandene Übung, drei Tempi), Prüfen. | Soll | 2 | P1, P3, P7 | J-G1, J-G3 |
| LH-LRN-09 | Glossar „Was ist …?“ (DE/EN), aus jedem Fachbegriff in Übungsziel, Generator, Auswertung und Technik-Zeichen erreichbar, zurück an dieselbe Stelle. | Soll | 2 | P1, P2, P5 | J-G2 |
| LH-LRN-10 | Prüffragen kehren nach Tagen wieder; fällige Fragen auf der Startseite; „Kann ich schon“ hakt ein Kapitel nach bestandenen Fragen ab. | Kann | 2 | P1, P2 | J-G3 |
| LH-LRN-11 | Offenes, dokumentiertes Lektionsformat; eigene Lektionen in der Bibliothek; ab Phase 4 als `.kontrakurs` gepackt (LH-LRN-14, -15). | Kann | 3 | P4 | J-G4 |
| LH-LRN-12 | Lektionen gelten für eine Stufe (`any`, Rolle wie `bass`, Familie, Instrument) aus dem Profil; ein Profil ohne passende Lektion bekommt einen Hinweis statt einer falschen Lektion. | Soll | 2 | P5, P6 | J-A4 |
| LH-LRN-13 | Kurs je Instrument aus einem gemeinsamen Lektionsvorrat (Varianten, `needs`); Wissen aus `any`-Lektionen zählt über Kurse hinweg. Bis 1.0 nur der Bass-Kurs, alle 15 Kapitel vor 0.1. | Soll | 2 | P5, P6, P1 | Tests je Kurs: `for`, Sprachen, Reihenfolge der `needs` |
| LH-LRN-14 | **Kurse nachladen** (Eigentümer, 25.10.): Kurspaket `.kontrakurs` (Lektionsformat, Medien, Manifest mit Anbieter und Lizenz) aus Datei oder HTTPS-Adresse hinzufügen; nachgeladene Kurse dürfen Videos mit Untertiteln enthalten, bevorzugt über das Internet eingebunden (kleine Pakete), mitgelieferte nie. Das Manifest nennt die Stream-Hosts; Kontra zeigt sie vor dem Hinzufügen und streamt nur von ihnen, erst auf Klick. Das Glossar gilt über alle installierten Kurse; doppelte Begriffe zeigen alle Texte hintereinander mit Herkunft. Kein Shop, kein Konto in Kontra. Videos als WebM oder MP4; Dateityp `application/x-kontrakurs`, ein Doppelklick zeigt erst die Vorschau (Runde 14). | Soll | 4 | P2, P4, P1, P7 | J-G5; Test: kein Video-Block in mitgelieferten Kursen |
| LH-LRN-15 | **Kurswerkstatt** (Eigentümer, 25.10.): kleines Begleitprogramm, das einen Kurs anlegt (Manifest, Kapitel, Lektionsordner), Lektionen beim Schreiben mit derselben Darstellung wie Kontra zeigt, sie prüft (Fehler mit Datei und Zeile) und als `.kontrakurs` packt; dieselben Schritte als `kontra-cli course new\|check\|pack`. | Soll | 4 | P4, P2 | J-G6; Test: gepacktes Paket lädt in Kontra ohne Befund |
| LH-LRN-16 | **Stellen wiederholen** (DIDAKTIK I3): Abschnitte, die zweimal unter den schwierigsten waren, kommen nach Abstand wieder (1 Tag, dann doppelt bis 64 Tage; früh bestanden verlängert nicht, nicht bestanden setzt auf 1 Tag). Bestanden heißt: ohne Hilfe (LH-LRN-05) mindestens 80 % im Zieltempo. Startseite höchstens 3, keine Karte, wenn nichts fällig ist; nie als Schuld formuliert (Harmonicon-Analyse, 29.10.). | Soll | 2 | P2, P1, P3 | Test der Abstände; J-C6 erweitert. |
| LH-LRN-17 | **Übungsleiter je Lektion:** Übungsblöcke in Lektionen erzeugen fünf Stufen (einzeln, festigen, abwandeln, im Zusammenhang, gemischt); von Stufe zu Stufe steigt die Notenrate um etwa 25 %, nicht Tempo und Unterteilung zugleich; Schwelle 60 bis 80 %. Gleicher Startwert je Lektion, damit Wiederholungen vergleichbar sind. Anzeige als Messung, nicht als Abzeichen; nichts gesperrt (Harmonicon-Analyse, 29.10.). | Soll | 3 | P1, P3, P4 | Test: Notenrate je Stufe; gleiche Lektion erzeugt gleiche Übung. |
| LH-LRN-18 | **Griffbrett-Quiz gewichtet:** schwache und lange nicht geübte Töne kommen öfter; Statistik je Saite und Bund bleibt gespeichert; Bereiche Bund 0–4 (Standard), 0–7, 0–12, eigene Auswahl; Übersicht als Heatmap mit Zahl. Zeitüberschreitung zählt nicht als Fehler. Gleiches für Intervalle im Gehörtraining (Harmonicon-Analyse, 29.10.). | Soll | 2 | P1, P2, P3, P7 | Test: ein oft verfehlter Ton kommt messbar häufiger. |
| LH-LRN-19 | **Ruf und Antwort im Groove** (DIDAKTIK I8): Kontra spielt 2 Takte Bass über Schlagzeug oder Akkorde, der Spieler antwortet in den nächsten 2. Motiv aus Akkordtönen, Umfang höchstens eine Quinte; die Begleitung wird während des Rufs leiser. Mit und ohne Bewertung; Tab erst nach 2 Fehlversuchen (Harmonicon-Analyse, 29.10.). | Soll | 3 | P1, P3, P5 | Test: Antwort im Takt wird erkannt; Ruf landet nicht in der Bewertung (Mikrofon). Eigene Journey in `QA.md` mit der Umsetzung. |
| LH-LRN-20 | **Ziel je Lektion:** Übungsblöcke nennen ihr Prüfkriterium (Trefferquote, Technik, Timing-Streuung oder keins); die Kurswerkstatt meldet unbekannte Werte mit Datei und Zeile; die Auswertung sagt „Ziel 80 %, erreicht 72 %“. Reine Anleitung zeigt nie 0 % (Harmonicon-Analyse, 29.10.). | Soll | 2 | P1, P4 | Test: unbekanntes Kriterium → Befund in `course check`. |
| LH-LRN-21 | **Kurs aus Ordner live:** ein ungepackter Kursordner lässt sich öffnen; bei jeder Änderung prüft Kontra neu, zeigt Befunde und die offene Lektion neu, ohne Fortschritt zu speichern (Harmonicon-Analyse, 29.10.). | Kann | 4 | P4 | J-G6 erweitert: geänderte Lektion erscheint ohne Packen. |

### 4.8 Wiedergabe (WIE)

| ID | Anforderung | Prio | Ph. | Personas | Abnahme |
|---|---|---|---|---|---|
| LH-WIE-01 | Backing Track mit Tempoänderung ohne Tonhöhenänderung (25 bis 150 %) in guter Qualität. | Muss | 1 | P2 | Hörtest bei 50 %: keine metallischen Artefakte (Blindvergleich mit Referenz). |
| LH-WIE-02 | Transponieren des Backing Tracks in Halbtönen (±6) mit passender Anpassung der Noten. | Kann | 3 | P2 | +2 Halbtöne: Audio und Tab passen zusammen. |
| LH-WIE-03 | Mixer: Instrument, Backing Track, Metronom, Referenz-Bass (synthetisch aus dem Arrangement) getrennt. Instrument heißt Mithören über Kontra: abschaltbar, Standard aus, mit Hinweis, dass Direct Monitoring des Interfaces schneller ist (Runde 6). | Muss | 1 | P2, P1 | Referenz-Bass zum Vorhören einer Stelle abspielbar. |
| LH-WIE-04 | Metronom mit Shuffle-Gefühl (Triolen-Achtel) für Shuffle-Grooves (Harmonicon-Analyse, 29.10.). | Kann | 2 | P2, P3 | Shuffle-Übung: Klick auf 1. und 3. Triole. |
| LH-WIE-05 | **Reagierende Band** beim freien Spiel mit erzeugter Begleitung: lässt der Bass am Phrasenende 2 Schläge frei, spielt das Schlagzeug ein Fill (höchstens alle 8 Takte); nach einer dichten Phrase wird die Begleitung dünner. Abschaltbar, ohne Bewertung (Harmonicon-Analyse, 29.10.). | Kann | 3 | P3, P5, P2 | Test mit Aufnahme: Fill nur nach Pause, Abstand ≥ 8 Takte. |

### 4.9 Fortschritt (FOR)

| ID | Anforderung | Prio | Ph. | Personas | Abnahme |
|---|---|---|---|---|---|
| LH-FOR-01 | Fortschritt pro Lied und Abschnitt (beste Trefferquote, Tempo, Datum), Übungszeit pro Tag. | Muss | 1 | P1, P2 | Nach Neustart vorhanden. |
| LH-FOR-02 | Verlauf in der Riff-Werkstatt (Tempo über Zeit) als Diagramm mit Achsen und Legende. | Soll | 2 | P3 | Diagramm zeigt die letzten 30 Tage. |
| LH-FOR-03 | Daten lokal als offene Dateien (JSON oder SQLite), Export und Import. | Muss | 1 | P4 | Export auf zweitem Rechner importiert: gleicher Stand; Import bietet Ergänzen (vorgewählt) und Ersetzen. |
| LH-FOR-04 | Fortschritt einer Übung als Bericht exportieren (für Unterricht). | Kann | 3 | P4 | PDF oder HTML mit Datum, Tempo, Trefferquote. |
| LH-FOR-05 | Übungsserie in Tagen, standardmäßig aus; ein verpasster Tag wird verziehen; gezählt werden nur Tage mit gewertetem Spiel; eine leise Zeile ab 2 Tagen, nie als Verlust formuliert (Harmonicon-Analyse, 29.10.). | Kann | 2 | P1, P2 | Test: Lücke von einem Tag hält die Serie. |
| LH-FOR-06 | **Liedkarte:** Streifen über das ganze Lied, Höhe je Takt = Trefferquote über die letzten Durchläufe, mit Zahl im Tooltip und per Tastatur erreichbar; nur angehalten und in der Auswertung, nie beim Spielen; Ziehen setzt die Schleife. Kante-Baustein (Harmonicon-Analyse, 29.10.). | Soll | 2 | P2, P3, P4, P7 | J-B3 erweitert: Ziehen über Takt 5–8 setzt die Schleife. |

### 4.10 Allgemein (ALL)

| ID | Anforderung | Prio | Ph. | Personas | Abnahme |
|---|---|---|---|---|---|
| LH-ALL-01 | Oberfläche auf Deutsch und Englisch; Sprache folgt dem System, umschaltbar. | Muss | 1 | alle | Keine rohen Übersetzungsschlüssel, kein Sprachmix (J-F2). |
| LH-ALL-02 | Stilwahl: Kante (Standard), Kante Light (abgespeckte Kante mit den Systemfarben), System (Breeze). Hell oder dunkel ist eine eigene Einstellung (wie System, hell, dunkel), im Präsentationsmodus auch als Schalter (Runde 15). | Muss | 1 | P7 | Alle Ansichten in jeder Kombination aus Stil und hell/dunkel lesbar (Kontrast ≥ 4,5:1 für Text). |
| LH-ALL-03 | Alle Einstellungen wirken sofort und überstehen einen Neustart. | Muss | 1 | alle | – |
| LH-ALL-04 | Kontra stürzt bei abgezogenem Interface nicht ab; meldet es und bietet den nächsten Eingang an; beim Wiederanstecken wird das Interface wieder genutzt. | Muss | 1 | P2 | Ab- und Anstecken während des Spiels, 10-mal. |
| LH-ALL-05 | Hilfe im Programm: kurze Erklärungen an Ort und Stelle (Latenz, Hi-Z, Zeitfenster), keine Pflichtlektüre. | Soll | 1 | P1, P6 | Jeder Fachbegriff in der Oberfläche hat eine Erklärung per Tooltip oder Info-Knopf. |
| LH-ALL-06 | Startseite: zuletzt gespieltes Stück, nächste mitgelieferte Übung, die noch nicht sitzt (über 90 %), Übungszeit der letzten sieben Tage, Einstieg in Stimmgerät, Griffbrett und Gehör; solange die Einrichtung fehlt, bietet sie sie an. Navigation nach Tätigkeit gruppiert, Einrichtung unter Einstellungen. | Soll | 1 | P1, P2, P3 | Vom Programmstart zum Weiterüben ein Klick (J-C1). |

---

## 5. Nichtfunktionale Anforderungen

| ID | Anforderung | Prio | Ph. | Abnahme |
|---|---|---|---|---|
| LH-NF-01 | **Latenz:** Anschlag bis Anzeige < 30 ms mit Interface (Quantum 128/48 kHz); < 60 ms mit Laptop-Buchse. | Muss | 0/1 | Messung mit Loopback und Kamera (240 fps) auf Empfehlungs-Hardware. |
| LH-NF-02 | **Onset-Genauigkeit:** Fehler < 10 ms gegenüber annotiertem Testkorpus. | Muss | 0/1 | Automatischer Test gegen Testkorpus. |
| LH-NF-03 | **Tonhöhe:** ≥ 98 % korrekt bei Einzelnoten B0 bis G4 (Interface), ≥ 95 % mit Laptop-Buchse, ≥ 90 % Mikrofon ohne Kopfhörer (ab Phase 4). | Muss | 0/1/4 | Automatischer Test gegen Testkorpus. |
| LH-NF-04 | **Stabilität des Audios:** keine Aussetzer (Xruns) in 60 Minuten „Frei spielen“ mit Time-Stretch auf Mindest-Hardware. | Muss | 1 | Dauertest, Xrun-Zähler = 0. |
| LH-NF-05 | **Startzeit:** bis zur Bibliothek < 3 s (Empfehlung), < 6 s (Mindest-Hardware). | Soll | 1 | Messung. |
| LH-NF-06 | **Darstellung:** 60 fps in der Tab-Lane auf Mindest-Hardware; die Lane ruckelt nicht bei Tempoänderung. | Muss | 1 | Frame-Zeit-Messung, 99. Perzentil < 20 ms. |
| LH-NF-07 | **Barrierefreiheit:** vollständige Tastaturbedienung, sichtbarer Fokus, Barrierefreiheitsnamen für alle Bedienelemente, Skalierung 100 bis 200 % ohne Verlust, „Bewegung reduzieren“ wird beachtet (Systemeinstellung und eigener Schalter; dann Tabulatur statt Lane), Information nie nur über Farbe. | Muss | 1 | J-F1 bestanden. |
| LH-NF-08 | **Datenschutz:** keine Netzwerkzugriffe außer auf ausdrücklichen Wunsch (Download Stem-Modell); keine Telemetrie; Audio wird nicht gespeichert, außer der Nutzer nimmt bewusst auf. | Muss | 1 | Netzwerk-Monitor während J-A1 bis J-C3: keine Verbindungen. |
| LH-NF-09 | **Offline:** alle Funktionen außer optionalen Downloads ohne Netz. | Muss | 1 | Test ohne Netz. |
| LH-NF-10 | **Design:** GUI aus Kante (QML-Komponenten unverändert); keine eigenen Farben, Schriften oder Maße; fehlende Komponenten zuerst in Kante. | Muss | 1 | Prüfskript: keine Hex-Werte und px-Maße für Bedienelemente außerhalb der vendorten Kante-Dateien. |
| LH-NF-11 | **Lizenz:** Kontra unter MIT; Abhängigkeiten nur unter freizügigen Lizenzen (MIT, Apache-2.0, BSD, ISC, Zlib, Unlicense, CC0; Schriften SIL OFL); Qt und KDE Frameworks (LGPL) nur dynamisch gelinkt; im Zweifel eigene Implementierung. Lizenzliste im Programm. | Muss | 1 | `cargo-deny` in `make check` schlägt bei jeder anderen Lizenz fehl. |
| LH-NF-12 | **Paketierung:** Flatpak mit KDE-Runtime, Zugriff nur auf Audio, MIDI und vom Nutzer gewählte Dateien. | Muss | 1 | Flatpak-Manifest-Review, keine `--filesystem=home`. |
| LH-NF-13 | **Testbarkeit:** Engine läuft ohne Hardware gegen WAV-Dateien und virtuelle MIDI-Quellen; Journey-Tests laufen in CI. | Muss | 0 | CI-Lauf mit Testkorpus grün. |
| LH-NF-14 | **Erweiterbarkeit:** neue Instrumente und Eingänge als Profile und Adapter, ohne Änderung an Session und Bewertung. | Muss | 1 | Dummy-Profil im Test. |
| LH-NF-15 | **Rückmeldung:** Eine Note ist im Korpus spätestens 70 ms nach ihrem Anschlag mit Tonhöhe gemeldet (p95, Linien-Sätze; Wert nach der ersten Messung bestätigen). LH-NF-01 misst Gerät und Puffer im Rundlauf, diese Anforderung die Zeit der Erkennung danach; ob beide zusammen das Ziel von LH-NF-01 halten, zeigt die Hardware-Prüfung. P1, P2, P7 (Harmonicon-Analyse, 29.10.). | Soll | 1 | `kontra-cli korpus check` mit Obergrenze. |
| LH-NF-16 | **Keine Falschnoten im Leerlauf:** Sätze ohne gespielte Noten (Saitengeräusch, Abdämpfen, Kabelknacken, Raum, Brummen) erzeugen höchstens 1 Falschnote je Minute. P1, P6, P7 (Harmonicon-Analyse, 29.10.). | Soll | 1 | Korpus-Sätze ohne Noten in der CI (29.10.: 0 je Minute). |

---

## 6. Rahmenbedingungen

- **Technik:** Rust; Qt 6/QML mit Kirigami und Kante-QML über cxx-qt; cpal; eigene Erkennung, eigener Time-Stretch und Notensatz; Details im Konzept, Abschnitt 6.
- **Repository:** `shrippen/kontra` auf git.arianw.de, GitHub als Spiegel; Änderungen nur als Pull Request.
- **Arbeitsregeln:** [`agent.md`](../agent.md) (Code-Stil, Persona- und QA-Regel, Release-, GUI- und Repository-Regel).
- **Release:** jede Version mit Release-Log (Neu, Verbessert, Behoben, Hinweis), Screenshots aus der Demo und Eintrag auf der Landing Page.

---

## 7. Abnahme

1. **Automatisch** (`make check`, CI): Unit-Tests, Erkennungs-Tests gegen den Testkorpus (LH-NF-02, LH-NF-03), Journey-Tests mit eingespeisten WAV-Dateien für alle Journeys mit messbaren Schritten.
2. **Explorativ** nach [`QA.md`](QA.md): Jede Phase gilt erst als abgenommen, wenn alle Journeys der Phase mit allen betroffenen Personas durchgespielt sind und kein Fund der Stufe „Fehler“ offen ist.
3. **Testkorpus** (`testdata/korpus/`): annotierte Aufnahmen (Onset, Tonhöhe, Saite), mindestens:
   - E-Bass 4 und 5 Saiten über Interface, alle Saiten, Bünde 0 bis 12, drei Anschlagsstärken
   - dasselbe über eine Laptop-Kombibuchse
   - bundloser Bass, absichtlich ±5 bis ±40 Cent verstimmt
   - Techniken (ab Phase 4)
   - Kontrabass am Mikrofon mit Raum und Backing Track aus Lautsprechern (ab Phase 4)
4. **Messprotokoll** für Latenz und Bildrate je Release auf Empfehlungs- und Mindest-Hardware.

---

## 8. Offene Punkte

| Punkt | Klärung in |
|---|---|
| cxx-qt mit Kirigami: Listenmodelle, Bildrate der Tab-Lane | Phase 0 |
| cpal oder direkter PipeWire-Treiber (Latenz) | Phase 0 |
| Notensatz-Komponente in Kante (Bravura, einstimmig zuerst) | Phase 0, mit Kante |
| Herkunft des Testkorpus (eigene Aufnahmen, Lizenz für Weitergabe in CI) | vor Phase 1 |
| Format für Fortschrittsdaten (JSON-Dateien oder SQLite) | Phase 1 |
