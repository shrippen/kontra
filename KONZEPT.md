# Kontra: Konzept für einen Bass-Trainer

Arbeitstitel **Kontra** (von Kontrabass; das K passt zu KDE). Stand: 9. Oktober 2026, abgestimmt. Entscheidungen in Abschnitt 9; Anforderungen in [`LASTENHEFT.md`](LASTENHEFT.md), Personas in [`PERSONAS.md`](PERSONAS.md), User Journeys in [`QA.md`](QA.md).

---

## 1. Vision in einem Satz

Ein freies Desktop-Programm, das am echten E-Bass zuhört, Noten und Tabs mitlaufen lässt und sofort zurückmeldet,
ob Ton und Timing stimmen. Man kann damit **Bass lernen** oder **einfach Lieder spielen**. Der Bass steckt per Klinke
direkt am Rechner; andere Instrumente kommen über Klinke, MIDI oder Mikrofon dazu. Es ist für KDE Plasma gebaut,
läuft aber auch unter Windows und macOS, ohne Abo, ohne Konto und offline.

---

## 2. Analyse: Rocksmith / Rocksmith+

**Was es ist.** Ubisofts Lernspiel für Gitarre und Bass; Rocksmith+ ist das Abo-Modell (seit 2022, inzwischen auch Klavier).
Das Instrument wird über das *Real Tone Cable* (USB-Klinkenkabel) oder über das Handy-Mikrofon (App *Rocksmith+ Connect*) angeschlossen.

**So funktioniert es.**

| Baustein | Umsetzung |
|---|---|
| Darstellung | 3D-„Note Highway“: Noten fliegen auf ein perspektivisches Griffbrett zu, Saiten farbcodiert, Bundbereich („Anchor“) hervorgehoben. Rocksmith+ bietet zusätzlich Tab- und Notenansicht. |
| Erkennung | Tonhöhe und Anschlagszeitpunkt aus dem Audiosignal, Techniken (Slides, Bends, Hammer-ons, Slap/Pop) teilweise. |
| Didaktik | **Dynamic Difficulty**: Ein Song startet mit wenigen Noten und wird dichter, sobald man trifft. **Riff Repeater**: Abschnitt wählen, verlangsamen, Schwierigkeit fixieren, wiederholen. |
| Motivation | Punkte, Streaks, Missionen, Minispiele (Guitarcade) für Technik-Drills, Sessions-Modus mit virtueller Band. |
| Inhalte | Lizenzierte Originalaufnahmen (Rocksmith+: mehrere tausend Arrangements), Workshop für eigene Arrangements zu lizenzierten Songs. |
| Sound | Verstärker- und Effektsimulation, das Instrument klingt über das Spiel. |

**Stärken.** Echtes Instrument statt Controller; Originalsongs motivieren stark; adaptive Schwierigkeit hält im Flow;
der Riff Repeater ist das didaktisch wertvollste Werkzeug.

**Schwächen (für uns relevant).**
- Kein Linux (Rocksmith 2014 läuft nur über Proton/Wine mit Basteleien am Audio-Stack).
- Abo, Online-Pflicht, plattformgebunden; Songs verschwinden, wenn Lizenzen auslaufen.
- Bass ist „Gitarre mit vier Saiten“: wenig Groove- und Timing-Didaktik, wenig Theorie (Intervalle, Skalen, Walking Bass).
- Erkennung toleriert vieles: Timing-Fehler im Bereich von 50 bis 100 ms gehen oft durch, für Bass ist genau das der Kern.
- Latenzprobleme mit HDMI- oder Bluetooth-Ausgabe; Kalibrierung ist versteckt.
- Keine eigenen Noten/Tabs (Guitar Pro, MusicXML) ohne Umwege.

---

## 3. Analyse: Synthesia

**Was es ist.** Klavier-Lernprogramm (Windows, macOS, iOS, Android), Freemium mit einmaligem Kauf.
Eingabe über MIDI-Keyboard; spielt beliebige MIDI-Dateien.

**So funktioniert es.**

| Baustein | Umsetzung |
|---|---|
| Darstellung | „Falling Notes“: Balken fallen auf eine Klaviatur, Länge = Notendauer; optional Notenblatt darüber oder stattdessen. |
| Erkennung | Trivial und exakt, weil MIDI: Taste, Zeit, Anschlagstärke. |
| Didaktik | **Melody Practice / Warte-Modus**: Das Stück hält an, bis die richtige Taste gedrückt ist. Hände getrennt üben, Tempo frei, Schleifen, Fingersatz-Hinweise. |
| Motivation | Punkte, Sterne, Fortschritt pro Song, Leuchttasten-Keyboards werden angesteuert. |
| Inhalte | Mitgelieferte Stücke plus **jede MIDI-Datei**: Die Community liefert den Katalog. |

**Stärken.** Offenes Format statt Lizenzkatalog; der Warte-Modus nimmt Anfängern den Druck;
sehr schnell vom Starten zum Spielen; klare, ruhige Oberfläche.

**Schwächen (für uns relevant).**
- Lehrt Notenlesen kaum, man „spielt Balken nach“; Reviews loben den Spaß, bemängeln den Lerneffekt.
- Keine Musiktheorie, kein Gehörtraining.
- Funktioniert nur, weil MIDI eindeutig ist. Für einen Bass mit Mikrofon oder Kabel muss Tonerkennung erst gebaut werden.

---

## 4. Vergleich und Lücke

| | Rocksmith+ | Synthesia | **Kontra (Ziel)** |
|---|---|---|---|
| Instrument | Gitarre, Bass, Klavier | Klavier | **Bass zuerst** (4, 5, 6 Saiten, bundlos), dann Keyboard, Gitarre, weitere |
| Eingabe | Audio (Kabel, Mikro) | MIDI | **Klinke direkt, Audio-Interface, Mikrofon, MIDI** |
| Linux / KDE | nein | nein | **ja, primär** |
| Kosten | Abo | einmalig | **frei (MIT)** |
| Online-Pflicht | ja | nein | **nein** |
| Eigene Inhalte | eingeschränkt | jede MIDI-Datei | **Guitar Pro, MusicXML, MIDI, eigene Audiodatei** |
| Warte-Modus | nein | ja | **ja** |
| Adaptive Schwierigkeit | ja | nein | **ja** |
| Abschnitt loopen, verlangsamen | ja | ja | **ja** |
| Timing/Groove-Fokus | schwach | schwach | **Kernfunktion** |
| Theorie, Griffbrett, Gehör | kaum | nein | **ja (eigene Module)** |

Die Lücke: Es gibt kein Programm, das Rocksmiths Echtinstrument-Feedback mit Synthesias offenem Inhaltsmodell verbindet,
den Bass als eigenes Instrument mit Groove-Fokus behandelt und nativ unter Linux läuft.

---

## 5. Produktkonzept

### 5.1 Grundprinzipien

1. **Zwei Wege, ein Programm.** Wer lernen will, bekommt Übungen, Bewertung und Fortschritt.
   Wer nur spielen will, wählt ein Lied und spielt los: ohne Punkte, ohne Anhalten, ohne Lektion davor.
2. **Instrument zuerst.** Bass anstecken, Stück wählen, spielen: in unter 60 Sekunden vom Start zum ersten Ton.
3. **Timing ist so wichtig wie Tonhöhe.** Kontra misst beides: richtig/falsch *und* früh/spät in Millisekunden. Während des
   Spielens zeigt es dosiert (Zustand als Zeichen, Lage zum Beat als Richtung), Millisekunden, Streuung und Tendenz
   ausführlich danach in der Auswertung; Werkstatt und Warten-Modus dürfen beim Üben mehr zeigen (Runde 5, nach
   DIDAKTIK.md These 2: dauernde Echtzeit-Urteile helfen beim Üben, schaden aber dem Behalten).
4. **Offene Inhalte statt Katalog.** Kontra bringt keine lizenzierten Songs mit; es liest, was der Nutzer hat.
5. **Lernen, nicht nur Nachspielen.** Neben dem Song-Modus gibt es Module, die Griffbrett, Theorie und Gehör trainieren.
6. **Lokal und privat.** Kein Konto, keine Telemetrie; Fortschritt liegt als Datei beim Nutzer.

### 5.2 Modi

| Modus | Vorbild | Beschreibung |
|---|---|---|
| **Spielen** | Jam-Modus | Lied läuft durch, Noten laufen mit, nichts hält an, keine Punkte (Treffer nur dezent markiert). Mixer für eigenes Instrument, Backing Track und Metronom. Der schnellste Weg vom Start zum Mitspielen. |
| **Song** | Rocksmith | Stück läuft mit Backing Track, Noten laufen mit, Treffer werden bewertet. Adaptive Schwierigkeit optional. |
| **Warten** | Synthesia | Stück hält an, bis der richtige Ton kommt. Für Anfänger und zum Erarbeiten neuer Stellen. |
| **Riff-Werkstatt** | Riff Repeater | Abschnitt markieren, Tempo 25 bis 150 %, Tempo hält die Trefferquote im Zielband 80–90 % (Runde 5), Schwierigkeit fixieren. |
| **Groove** | neu | Metronom- und Drum-Patterns, Aufgaben wie „Achtel auf 92 BPM, nur Zählzeit 2 und 4 betont“, Auswertung als Timing-Streuung (Histogramm). Ghost Notes, Pausen, Shuffle. |
| **Griffbrett** | neu | Töne finden („Zeig mir alle C“), Intervalle, Tonleitern und Arpeggien in Lagen, Quiz mit dem echten Instrument. |
| **Gehör** | neu | Ton oder Linie hören, nachspielen; Intervalle; Grundton zu einem Akkord finden. |
| **Stimmgerät** | Pflicht | Chromatisch, mit Unterstützung für Drop- und Tieftunings sowie B-Saite. |

### 5.3 Darstellung

- **Standard: horizontale Tab-Lane (2D).** Vier bis sechs Saiten als Bahnen, Noten wandern von rechts auf eine Spiellinie zu,
  Bundzahl auf der Note, Länge = Dauer. Das ist lesbarer als ein 3D-Highway, braucht keine GPU-Leistung und ist barrierearm.
- **Alternativ: Notenschrift (Bassschlüssel)** und **klassische Tabulatur**, umschaltbar oder kombiniert.
- **Griffbrett-Ansicht** unten: zeigt die nächste Note und die Lage (Anchor), optional mit Fingersatz.
- **3D-Highway** als spätere Option, nicht im MVP. Technik (Runde 3): zuerst 2,5D in Qt Quick, kein Wechsel zu Godot;
  Qt Quick 3D nur nach neuer Bewertung (GPL-3).
- Saitenfarben konfigurierbar; Bedeutung nie nur über Farbe (Form und Beschriftung zusätzlich).

### 5.4 Feedback und Bewertung

- Pro Note: Treffer/daneben, Abweichung in Cent (Intonation, wichtig für bundlos) und in Millisekunden (Timing).
- Pro Durchlauf: Trefferquote, Timing-Histogramm, „meist zu früh / zu spät“, schwierigste Takte.
- Fortschritt: pro Stück und Abschnitt, Übungszeit, Tempo-Verlauf in der Riff-Werkstatt.
- Ton: motivierend, nicht verspielt. Punkte und Streaks ja, aber abschaltbar.

### 5.5 Inhalte

| Quelle | Weg in Kontra |
|---|---|
| Guitar Pro (`.gp`, `.gpx`, `.gp5`, `.gp4`, `.gp3`) | Import, Bassspur wählen. Riesige Community-Bestände. |
| MusicXML | Import (z. B. aus MuseScore). |
| MIDI | Import; Tab wird automatisch erzeugt (Lagen-Optimierung). |
| Eigene Audiodatei (MP3, FLAC, OGG) | Als Backing Track an ein Arrangement koppeln; Tempo-Map über Beat-Erkennung, von Hand korrigierbar. |
| Bass aus Audio entfernen | Optional, lokal über ein Stem-Trennungsmodell (z. B. Demucs als ONNX). Separater Download, nicht im Kern. |
| Mitgelieferte Übungen | Eigene, frei lizenzierte Etüden und Grooves, generierte Übungen (Tonleitern, Arpeggien, Rhythmen). |
| Eigenes Format | Offenes Paket (`.kontra`: JSON-Arrangement plus Verweis oder Einbettung der Audiodatei), damit Nutzer Arrangements teilen können, ohne Audio zu verteilen. |

Bewusst **nicht**: Rocksmith-CDLC (`.psarc`) oder Streaming-Dienste. Beides ist rechtlich heikel und bindet an fremde Plattformen.

### 5.6 Instrumente und Eingänge

Kontra trennt **Instrument** (was gespielt wird) und **Eingang** (wie das Signal ankommt). Jedes Instrument ist ein Profil:
Stimmung und Saitenzahl oder Tastenumfang, Notenschlüssel, passende Ansicht (Tab-Lane, Klaviatur, Notenzeile), Erkennungsparameter.

**Eingänge**

| Eingang | Signal | Erkennung | Hinweise |
|---|---|---|---|
| **Klinke direkt** in den Rechner | Audio, nur das Instrument | Onset + Tonhöhe | Hauptweg für den Bass. Laptop-Buchsen sind meist Mikrofon-Eingänge mit unpassender Impedanz und wenig Pegelreserve: Es funktioniert, klingt aber dumpfer und kann übersteuern. Der Einrichtungsassistent misst Pegel und Rauschen und rät bei Bedarf zu Hi-Z-Interface oder USB-Instrumentenkabel, ohne sie vorauszusetzen. |
| **Audio-Interface / USB-Instrumentenkabel** (Hi-Z) | Audio, sauber | wie oben | Empfohlen: niedrigste Latenz, bester Klang. |
| **Mikrofon** (eingebaut, USB, am Interface) | Audio mit Raum, Störgeräuschen und Backing Track | wie oben plus Rauschsperre und Echo-Abzug | Für akustische Instrumente (Kontrabass, Akustikgitarre, Ukulele, Gesang, Bläser). Kopfhörer empfohlen; ohne Kopfhörer zieht Kontra den bekannten Backing Track vom Mikrofonsignal ab. Toleranzen werden automatisch weiter, und die Oberfläche sagt das. |
| **MIDI** (USB, Bluetooth-MIDI, MIDI-Interface) | Notenereignisse | keine nötig, exakt | Keyboards, E-Drums, MIDI-Gitarren und -Bässe, Blaswandler. Mehrstimmigkeit ohne Aufwand. |

**Instrumente**

| Instrument | Typische Eingänge | Ansicht | Phase |
|---|---|---|---|
| E-Bass (4/5/6 Saiten, bundlos) | Klinke, Interface | Tab-Lane, Tabulatur, Bassschlüssel | 1 (MVP) |
| Kontrabass | Mikrofon, Tonabnehmer | Bassschlüssel, Griffbrett | 4 |
| Bass über MIDI (Synth-Bass am Keyboard, MIDI-Bass) | MIDI | Tab-Lane oder Klaviatur, Bassschlüssel | 4 |
| Keyboard / Klavier | MIDI | Falling Notes auf Klaviatur, Noten | nach 1.0 |
| E-Gitarre, Akustikgitarre, Ukulele (Einzelnoten, Riffs) | Klinke, Interface, Mikrofon | Tab-Lane | nach 1.0 |
| Gesang, Bläser, Geige (einstimmig) | Mikrofon, MIDI-Blaswandler | Notenzeile mit Tonhöhenkurve | nach 1.0 |
| Gitarre mit Akkorden (mehrstimmig über Audio) | Klinke, Mikrofon | Tab-Lane, Akkordsymbole | nach 1.0, eigene Machbarkeitsprüfung |
| E-Drums | MIDI | Drum-Lane | nach 1.0 |

**Bis 1.0 nur Bass.** Andere Instrumente werden erst nach 1.0 geplant. Die Architektur bleibt trotzdem offen dafür:
Instrument-Profil und Eingangs-Adapter sind von Anfang an getrennte Schnittstellen, damit ein neues Instrument
ein neues Profil ist und kein Umbau.

Einstimmige Instrumente über Audio nutzen dieselbe Erkennung wie der Bass, nur mit anderem Frequenzbereich.
Mehrstimmige Erkennung über Audio (Gitarrenakkorde) ist deutlich schwieriger; über MIDI ist Mehrstimmigkeit exakt.
Mehrere Eingänge gleichzeitig sind im Datenmodell vorgesehen, aber nicht vor 1.0 aktiv.

### 5.7 Sound

- Direktes Mithören (Monitoring) mit einfacher Verstärker-Simulation (EQ, Kompressor, leichte Sättigung), damit man ohne Verstärker üben kann.
- Wer schon einen Verstärker oder ein Interface mit Direct Monitoring hat, schaltet das Monitoring ab, dann zählt nur die Erkennung.

---

## 6. Technik

### 6.1 Architektur

```
┌──────────────────────── UI (QML: Kirigami + Kante) ───────────────────────────┐
│  Bibliothek · Spielen · Song · Warten · Riff-Werkstatt                        │
│  Groove · Griffbrett · Gehör · Tuner · Einrichtung (Eingang, Instrument)      │
└───────────────▲───────────────────────────────────────────▲───────────────────┘
                │ Modelle, Signale (cxx-qt)                  │
┌───────────────┴─────────────┐   ┌─────────────────────────┴────────────────────┐
│  Session (Rust)             │   │  Inhalte (Rust)                              │
│  Zeitachse, Bewertung,      │◄──┤  Import GP/MusicXML/MIDI → Arrangement       │
│  adaptive Schwierigkeit     │   │  Tab-Generator, Tempo-Map, .kontra-Pakete    │
└───────────────▲─────────────┘   └──────────────────────────────────────────────┘
                │ Notenereignisse (Zeit, Tonhöhe, Saite?, Quelle, Sicherheit)
┌───────────────┴─────────────────────────────────────────────────────────────────┐
│  Engine (Rust, Audio-Callback ohne Allokation), Instrument-Profil wählt Param.   │
│  Eingänge: Klinke · Audio-Interface · Mikrofon · MIDI                            │
│  Audio: [Rauschsperre, Echo-Abzug nur Mikro] → Onset → Tonhöhe → Ereignis        │
│  MIDI: Note-On/Off → Ereignis (gleiches Format, gleiche Zeitbasis)               │
│  Backing Track (Decoder, Time-Stretch) + Metronom + Monitoring → Ausgang         │
└───────────────▲─────────────────────────────────────────────────────────────────┘
                │ Treiber: cpal (ALSA/PipeWire/JACK) · midir (ALSA-Seq)
```

Schichten sprechen nur mit dem direkten Nachbarn darunter: Die UI ruft nie die Engine oder Treiber direkt auf,
die Session nie einen Audio-Treiber. Rohes Audio- und MIDI-I/O liegt in einer eigenen Treiberschicht.

### 6.2 Stack

Entschieden: **Rust**, **MIT-Lizenz**, **Linux zuerst**, und: **im Zweifel eigene Funktionalität statt fremder Bibliothek.**

**Abhängigkeitsregel.** Erlaubt sind nur freizügige Lizenzen (MIT, Apache-2.0, BSD, ISC, Zlib, Unlicense, CC0) sowie
SIL OFL für Schriften. Einzige Ausnahme sind Qt und KDE Frameworks (LGPL), dynamisch gelinkt. Keine GPL, kein MPL,
kein LGPL in Rust-Crates (Rust linkt statisch). Ist eine Lizenz unklar, eine Bibliothek kaum gepflegt oder nur für
einen kleinen Teil nötig, bauen wir die Funktion selbst. `cargo-deny` setzt die Liste in `make check` durch.

| Bereich | Wahl | Lizenz | Begründung |
|---|---|---|---|
| Sprache | Rust (stable) | MIT/Apache-2.0 | Keine Speicherbereinigung, also keine Pausen im Audio-Pfad; Speichersicherheit; Erfahrung aus lmix und strom. |
| UI | Qt 6, QML, **Kirigami**, **Kante-QML** (`qml/Kante`, unverändert kopiert) | LGPL / Kante | Nativ in Plasma; Aussehen kommt aus Kante (GUI-Regel). |
| Rust ↔ QML | **cxx-qt** (KDAB) | MIT/Apache-2.0 | QObjects, Signale und Listenmodelle in Rust, in KDE-Projekten im Einsatz. Phase 0 prüft das Zusammenspiel mit Kirigami. |
| Audio-I/O | **cpal** (ALSA, JACK; PipeWire über seine ALSA- und JACK-Schicht) | Apache-2.0 | Bewährt, plattformübergreifend für später. Phase 0 vergleicht die Latenz mit einem direkten PipeWire-Treiber (pipewire-rs, MIT); der bessere gewinnt. |
| Ringpuffer | **rtrb** | MIT/Apache-2.0 | Lock-frei, Echtzeit-tauglich. |
| MIDI | **midir** (Eingang, ab Phase 4), **midly** (Dateien) | MIT / Unlicense | Klein, gepflegt. |
| FFT | **rustfft** | MIT/Apache-2.0 | Grundlage für Erkennung und Time-Stretch. |
| Tonhöhe, Onset | **eigene Implementierung** (YIN/MPM, spektraler Fluss) | MIT | Volle Kontrolle über Fensterlänge und Bass-Optimierung. |
| Time-Stretch | **eigene Implementierung** (Phasen-Vocoder mit Phasengradienten-Integration, veröffentlichtes Verfahren) | MIT | Rubber Band ist GPL; Signalsmith Stretch (MIT, C++) dient nur in Tests als Qualitätsreferenz und wird nicht ausgeliefert. |
| Decoder | **hound** (WAV), **claxon** (FLAC), **lewton** (Ogg Vorbis), **minimp3** (MP3) | Apache-2.0 / MIT / CC0 | Reines Rust bzw. kleiner C-Kern; symphonia (MPL) wird gemieden. |
| Import | **eigene Parser** für Guitar Pro (GP3–GP5 binär, GP6/7 als ZIP+XML) und MusicXML; **zip**, **quick-xml** als Bausteine | MIT | Formatbeschreibungen aus TuxGuitar und alphaTab nur gelesen, kein Code übernommen. |
| Notensatz | **eigene Kante-Komponente** mit der Musikschrift **Bravura** (SMuFL) | SIL OFL | Verovio (LGPL) und MuseScore (GPL) fallen weg. |
| Stem-Trennung (optional) | **ort** + ONNX Runtime + Demucs-Modell | MIT/Apache-2.0, MIT | Separater Download, nach Phase 3. |
| Build | Cargo-Workspace, `make check` (cargo fmt, clippy, Tests, cargo-deny, Token-Prüfung) | | |
| Pakete | **Flatpak** mit KDE-Runtime und Rust-SDK-Erweiterung | | Linux zuerst; Windows und macOS erst nach 1.0. |

**Rust und Echtzeit-Audio.** Der Audio-Callback allokiert nicht, sperrt nicht und wartet auf nichts; Ergebnisse gehen über
vorab angelegte Ringpuffer (rtrb) an die Session. In Debug-Builds prüft ein Allokations-Wächter den Callback.
Phase 0 misst Aussetzer (Xruns) und Latenz unter Last.

### 6.3 Die schwierigste Stelle: Bass-Tonerkennung mit wenig Latenz

- Tiefe E-Saite: 41,2 Hz, eine Periode = 24 ms. Tiefe B-Saite: 30,9 Hz, eine Periode = 32 ms.
  Klassische Verfahren brauchen 2 bis 3 Perioden, also 50 bis 100 ms, bis die Tonhöhe sicher ist.
- **Lösung in drei Teilen:**
  1. **Timing und Tonhöhe trennen.** Der Anschlag (Onset) wird in wenigen Millisekunden erkannt und bestimmt den Zeitpunkt.
     Die Tonhöhe darf 40 bis 80 ms später nachgereicht werden; die Bewertung ordnet sie dem Onset zu.
  2. **Obertöne nutzen.** Bass-Töne haben starke 2. und 3. Harmonische; daraus lässt sich die Grundfrequenz früher schätzen.
     Die Erwartung („jetzt sollte ein A kommen“) dient als Prior, aber nie als Ersatz für die Messung.
  3. **Latenz kalibrieren.** Assistent beim ersten Start: Loopback oder „spiel auf den Klick“, gemessene Gesamtlatenz wird abgezogen.
- **Ziele:** Onset-Fehler < 10 ms, Gesamtlatenz Eingang → Anzeige < 30 ms mit Audio-Interface, Tonhöhe korrekt in ≥ 98 % der Einzelnoten
  (gemessen an einem Testkorpus aus aufgenommenen Bassspuren).
- Mehrstimmigkeit (Doppelgriffe, Akkorde) über Audio ist im MVP nicht vorgesehen; Bass ist überwiegend einstimmig.
  Über MIDI ist sie von Anfang an exakt.
- **Mikrofon:** zusätzlich Rauschsperre und Abzug des Backing Tracks (das Programm kennt das Signal, das es abspielt).
  Timing- und Intonationstoleranzen werden automatisch weiter.
- **Klinke am Laptop:** Der Assistent misst Pegel und Rauschen; die Erkennung muss auch mit diesem Signal die Ziele erreichen.
- **Testbarkeit:** Die Audio-Engine läuft auch ohne Hardware gegen WAV-Dateien. Damit sind Erkennung und Bewertung in CI prüfbar.

### 6.4 GUI und Design-Regel

Die GUI wird aus **Kante** erzeugt: Kante hat QML-Komponenten (`qml/Kante`, `KanteStyle`), die unverändert in
Kontra kopiert werden. Kirigami liefert Seitenstruktur und Navigation, Kante Farben, Typografie, Formen, Bewegung und Komponenten.
Stilwahl wie in anderen Projekten: Kante (Standard), Kante Light, System (Breeze).

Was Kontra braucht und Kante noch nicht hat, entsteht **zuerst in Kante** (QML-Komponente, Test, Eintrag in der Galerie), danach in Kontra:

| Komponente | Zweck |
|---|---|
| Tab-Lane | Saiten als Bahnen, laufende Noten mit Bund, Spiellinie, Treffer-Markierung |
| Tabulatur-Zeile | klassische Tab mit Takten, Rhythmus, Wiederholungen |
| Notenzeile | Bassschlüssel, Noten, Pausen, Vorzeichen; eigener Notensatz mit Glyphen aus Bravura (SMuFL, SIL OFL) |
| Griffbrett | Bünde, Saiten, Lagen-Markierung, Fingersatz |
| Stimmgerät | Tonname, Cent-Abweichung, Saite |
| Pegelanzeige | Eingangspegel mit Übersteuerungswarnung |
| Timing-Histogramm | Verteilung früh/spät in ms mit Achse und Legende |
| Transportleiste | Abspielen, Loop-Bereich, Tempo in %, Metronom |

Die Landing Page (`docs/index.html`) folgt wie bei allen Projekten der zentralen Kante-Seite.

---

## 7. Umfang und Roadmap

| Phase | Inhalt | Ergebnis |
|---|---|---|
| **0 Machbarkeit** | cxx-qt mit Kirigami und Kante-QML, Audio-Engine (cpal gegen pipewire-rs), Tuner, Onset + Tonhöhe gegen Testkorpus, eigener Time-Stretch gegen Referenz, Latenz und Xruns messen | Zahlen; Entscheidung je Baustein |
| **1 MVP** | E-Bass über Klinke/Interface, Guitar-Pro- und MIDI-Datei-Import, **Tab-Lane, Tabulatur und Notenansicht**, Spielen-, Song- und Warte-Modus, Riff-Werkstatt (Loop, Tempo), Backing Track mit Time-Stretch, Stimmgerät, Kalibrierung, Fortschritt lokal | 0.1 als Flatpak |
| **2 Lernen** | Groove-Modul, Griffbrett-Modul, adaptive Schwierigkeit, MusicXML-Import | 0.2 bis 0.4 |
| **3 Inhalte** | Eigene Audiodatei koppeln (Beat-Erkennung), `.kontra`-Pakete, mitgelieferte Etüden, Gehör-Modul | 0.5 bis 0.7 |
| **4 Bass komplett** | Mikrofon-Eingang (Kontrabass), Bass über MIDI, Techniken (Slap/Pop, Slides, Hammer-ons), optionale Stem-Trennung | 0.8 bis 1.0 |
| **nach 1.0** | Weitere Instrumente (Keyboard, Gitarre, Ukulele, Gesang, Bläser, Drums), 3D-Highway, Windows und macOS, Android | 1.x |

Nicht im Umfang: Online-Konten, Song-Shop, Multiplayer übers Netz.

---

## 8. Risiken

| Risiko | Auswirkung | Gegenmaßnahme |
|---|---|---|
| Erkennung zu langsam oder unzuverlässig bei tiefen Tönen | Kernversprechen bricht | Phase 0 als Go/No-Go; Onset/Tonhöhe getrennt; Testkorpus in CI |
| Audio-Latenz unter Linux (PipeWire-Quantum, Bluetooth, HDMI) | Timing-Feedback falsch | Kalibrierung, Warnung bei Bluetooth, Hinweis auf Quantum-Einstellung |
| Wenig Inhalte ohne Lizenzsongs | geringe Motivation | Guitar-Pro-Import, eigene Audiodatei koppeln, gute mitgelieferte Grooves |
| Guitar-Pro-Formate sind proprietär und komplex | Importfehler | Mit GP5 und GP7 beginnen, Testdateien, alphaTab/TuxGuitar nur als Formatreferenz |
| cxx-qt-Lernkurve, lange Rust-Builds | langsamer Start | Phase 0 mit kleiner QML-Seite; Workspace in kleine Crates teilen |
| Eigene Implementierungen (Time-Stretch, Notensatz, Parser) statt Bibliotheken | mehr Aufwand, Qualitätsrisiko | Referenz-Tests gegen Signalsmith Stretch und MuseScore-Exporte; Notensatz zuerst einstimmig im Bassschlüssel |
| Notenansicht schon im MVP | Notensatz ist aufwendig | Bassschlüssel einstimmig zuerst, Komponente in Kante, Bravura |
| Stem-Trennung ist groß und rechenintensiv | Downloadgröße, Wartezeit | Optional, getrennter Download, Ergebnis wird zwischengespeichert |
| Mikrofon: Raumklang und Backing Track stören die Erkennung | Fehlbewertungen | Kopfhörer empfehlen, Echo-Abzug, weitere Toleranzen, eigener Testkorpus mit Mikrofonaufnahmen |
| Klinke direkt in Laptop-Mikrofonbuchse | schwaches, verrauschtes Signal | Pegelassistent, Testkorpus mit solchen Aufnahmen, klarer Hinweis auf Hi-Z |
| Viele Instrumente verwässern den Bass-Fokus | mittelmäßig für alle | Bis 1.0 nur Bass; Instrument-Profile als klare Schnittstelle |
| Umfang wächst | nie fertig | Phasen strikt; MVP nur Spielen, Song, Warten, Werkstatt |

---

## 9. Entscheidungen (9. Oktober 2026)

| Frage | Entscheidung |
|---|---|
| Name | **Kontra** |
| Weitere Instrumente | **Bis 1.0 nur Bass** (E-Bass, in Phase 4 Kontrabass und Bass über MIDI); andere Instrumente nach 1.0, Architektur bleibt offen dafür |
| Repository | **Eigenes Repo `shrippen/kontra` auf git.arianw.de**, GitHub als Spiegel |
| Design | **Kirigami mit Kante**: Kante hat QML-Komponenten, sie werden unverändert übernommen; fehlende Komponenten zuerst in Kante |
| Stack | **Rust** (zuerst Go gewählt, nach Abwägung geändert: keine GC-Pausen im Audio-Pfad, cxx-qt für QML, Erfahrung aus lmix und strom) |
| Darstellung | **2D-Tab-Lane als Standard, 3D später; Tabulatur und Notenansicht schon im MVP** |
| Lizenz | **MIT**; nur freizügige Abhängigkeiten, **im Zweifel eigene Implementierung** (Time-Stretch, Tonerkennung, Notensatz, Import) |
| Plattformen im MVP | **Nur Linux (Flatpak)** |
| Personas | Alle sieben vorgeschlagenen, siehe [`PERSONAS.md`](PERSONAS.md) |

---

## 10. Dokumente

- [`LASTENHEFT.md`](LASTENHEFT.md): nummerierte Anforderungen mit Priorität, Phase und Abnahmekriterium.
- [`PERSONAS.md`](PERSONAS.md): sieben Personas mit Ausrüstung, Zielen, Frust und Prüfschwerpunkt.
- [`QA.md`](QA.md): User Journeys je Persona für explorative QA und automatisierte Journey-Tests.
- [`agent.md`](agent.md): Arbeitsregeln für das künftige Repo, inklusive Persona- und QA-Regel.

---

## Quellen

- Ubisoft, Rocksmith+: https://www.ubisoft.com/ru-ru/game/rocksmith/plus
- Business Wire, Ankündigung Rocksmith+ (2021): https://www.businesswire.com/news/home/20210612005018/en/Ubisoft-Reveals-Rocksmith-the-Future-of-Interactive-Music-Learning
- Guitar World, Rocksmith+: https://www.guitarworld.com/news/rocksmith-plus
- Synthesia, About: https://synthesia.app/About
- Pianoers, Synthesia Review 2026: https://pianoers.com/synthesia-piano-review/
