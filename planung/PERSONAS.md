# Kontra: Personas

Sieben feste Personas für Entwurf, User Journeys ([`QA.md`](QA.md)) und QA. Sie sind erfunden; Ähnlichkeiten mit echten Menschen sind nicht beabsichtigt.

**So werden sie benutzt** (verbindlich über [`agent.md`](../agent.md)):

- Jede Änderung an einem Ablauf nennt im PR die betroffenen Personas und spielt deren Journeys durch.
- Jede Anforderung im [Lastenheft](LASTENHEFT.md) nennt mindestens eine Persona, für die sie da ist.
- Bei Entwurfsfragen gilt: Was Lea (P1) überfordert, braucht einen einfacheren Weg; was Mira (P3) ausbremst, braucht eine Abkürzung.
- Personas, deren Instrument oder Eingang erst später kommt (P5, P6), prüfen vorher, dass die Architektur offen bleibt und dass Kontra ehrlich sagt, was noch nicht geht.

| ID | Persona | Instrument, Eingang | Kernziel | Ab Phase |
|---|---|---|---|---|
| P1 | Lea, Anfängerin | E-Bass per Klinke direkt in die Laptop-Buchse | Bass lernen, ohne sich zu blamieren | 1 |
| P2 | Jens, Wiedereinsteiger | E-Bass, kleines USB-Interface | Lieder für die Bandprobe am Freitag spielen | 1 |
| P3 | Mira, Musikstudentin | bundloser Fünfsaiter, Hi-Z-Interface | Intonation, Walking Bass, Notenlesen | 1 |
| P4 | Thomas, Basslehrer | E-Bass, Interface, Beamer im Unterricht | Übungen bauen und an Schüler weitergeben | 1 (Pakete ab 3) |
| P5 | Aylin, Keyboarderin | Synth-Bass am MIDI-Keyboard | Basslinien mit der linken Hand spielen | 4 |
| P6 | Karl, Kontrabassist | Kontrabass vor USB-Mikrofon | Jazz-Standards üben, ohne Technikärger | 4 |
| P7 | Sam, sehbehindert | E-Bass, Interface, Bildschirmlupe 200 % | Üben trotz Sehschwäche und Farbsehschwäche | 1 |

---

## P1 Lea Brandt, die Anfängerin

- **Alter, Lage:** 16, Schülerin, hat den Bass zum Geburtstag bekommen. Kein Unterricht, lernt mit Videos.
- **Ausrüstung:** Squier Jazz Bass, Klinkenkabel, Adapter auf die 3,5-mm-Kombibuchse ihres Laptops (KDE neon, 8 GB RAM, interne Soundkarte). Kopfhörer. Kein Verstärker, kein Interface.
- **Vorwissen:** Kann keine Noten, weiß nicht, was eine Tabulatur ist. Weiß nicht, wie man stimmt.
- **Ziel:** In der ersten Woche ein bekanntes Riff spielen (z. B. „Seven Nation Army“ als eigene Guitar-Pro-Datei) und merken, dass sie besser wird.
- **Frust:** Fachbegriffe ohne Erklärung, „Ihr Gerät wird nicht unterstützt“, Punkteabzug, der sie bloßstellt, lange Einrichtung.
- **Zitat:** „Ich will einfach anstecken und loslegen. Was ist Latenz?“
- **Prüft:** Erster Start, Einrichtungsassistent mit schwachem Signal aus der Laptop-Buchse, Stimmgerät, Warte-Modus, verständliche Sprache, Ermutigung statt Strafe.
- **Journeys:** J-A1, J-A2, J-B2, J-C1.

## P2 Jens Okafor, der Wiedereinsteiger

- **Alter, Lage:** 47, Projektleiter, hat vor 20 Jahren in einer Band gespielt. Jetzt Coverband, Probe freitags, vier neue Songs bis nächste Woche.
- **Ausrüstung:** Fender Precision, Focusrite Scarlett Solo, Desktop mit Fedora KDE, Studiomonitore. Hat Guitar-Pro-Dateien aus dem Internet und die MP3s der Setlist.
- **Vorwissen:** Liest Tabs, Noten mühsam. Kennt Begriffe wie Lage, Groove, Ghost Notes.
- **Ziel:** Lieder einfach durchspielen, schwierige Stellen gezielt üben, mit dem Original-Song im Hintergrund.
- **Frust:** Lektionen, die er nicht braucht; Gamification, die ihn bremst; Programme, die beim Wechsel zwischen Liedern viel klicken lassen.
- **Zitat:** „Ich hab zwanzig Minuten. Ich will den Refrain von Song drei sitzen haben.“
- **Prüft:** Spielen-Modus, Bibliothek und Import, Riff-Werkstatt (Loop, Tempo), Backing Track mit Time-Stretch, schnelle Wechsel, Setlist.
- **Journeys:** J-B1, J-B3, J-D1 (ab Phase 3).

## P3 Mira Szabo, die Musikstudentin

- **Alter, Lage:** 23, studiert Jazz-E-Bass. Übt täglich zwei Stunden, nimmt sich auf.
- **Ausrüstung:** bundloser Fünfsaiter (B-E-A-D-G), Hi-Z-Interface mit Direct Monitoring, Arbeitsplatz mit Arch Linux und Plasma, PipeWire mit kleinem Quantum.
- **Vorwissen:** Liest Bassschlüssel fließend, schreibt Arrangements in MuseScore (MusicXML).
- **Ziel:** Intonation in Cent sehen, Walking Bass über Standards, Tempo steigern, Timing-Tendenz kennen.
- **Frust:** Ungenaue Erkennung bei der tiefen B-Saite, Rundungen („gut genug“), fehlende Notenansicht, Mausbedienung ohne Tastenkürzel.
- **Zitat:** „Wenn das Ding bei 31 Hz rät, ist es für mich nutzlos.“
- **Prüft:** Tonerkennung tief und bundlos, Cent-Anzeige, Notenansicht, MusicXML (ab Phase 2), Timing-Histogramm, Tastaturbedienung, Latenz-Kalibrierung.
- **Journeys:** J-A3, J-B4, J-C2, J-C3 (ab Phase 2).

## P4 Thomas Reuter, der Basslehrer

- **Alter, Lage:** 58, unterrichtet an einer Musikschule und privat, 15 Schüler. Technisch solide, aber ohne Geduld für Bastelei.
- **Ausrüstung:** Tuxedo-Laptop mit Kubuntu, Interface, im Unterricht am Beamer (1280×720). Zuhause zweiter Rechner.
- **Vorwissen:** Alles Musikalische. Erstellt Übungen in Guitar Pro.
- **Ziel:** Übungen bauen oder importieren, im Unterricht groß zeigen, an Schüler weitergeben (ab Phase 3 als `.kontra`-Paket), Fortschritt besprechen.
- **Frust:** Kleine Schrift am Beamer, Dateien, die bei Schülern anders aussehen, Konten und Cloud-Zwang.
- **Zitat:** „Ich gebe dem Schüler eine Datei, und bei ihm sieht es genauso aus. Mehr will ich nicht.“
- **Prüft:** Darstellung bei kleiner Auflösung und großer Schrift, Import-Treue, Übungen anlegen, Export und Weitergabe, Fortschritt pro Übung, Präsentationsmodus.
- **Journeys:** J-B5, J-D2 (ab Phase 3).

## P5 Aylin Kaya, die Keyboarderin

- **Alter, Lage:** 34, spielt Keys in einer Funk-Band und übernimmt die Basslinien mit der linken Hand (Synth-Bass).
- **Ausrüstung:** MIDI-Keyboard über USB, Laptop mit openSUSE Tumbleweed und Plasma.
- **Vorwissen:** Liest Noten, versteht Harmonie, kennt kein Bassgriffbrett.
- **Ziel:** Basslinien im Timing sauber spielen; Groove-Training gilt für sie genauso.
- **Frust:** Programme, die nur Gitarrenhälse kennen; MIDI-Geräte, die nicht erkannt werden.
- **Zitat:** „Bass ist Bass. Ob ich ihn greife oder drücke, ist egal.“
- **Prüft:** Vor Phase 4: dass Instrument und Eingang im Datenmodell getrennt sind und Kontra klar sagt, dass MIDI noch nicht geht. Ab Phase 4: Bass über MIDI, Klaviatur-Ansicht für Basslinien, Timing-Auswertung.
- **Journeys:** J-E1 (ab Phase 4), J-A4.

## P6 Karl Weidner, der Kontrabassist

- **Alter, Lage:** 66, pensionierter Lehrer, spielt Kontrabass in einem Jazztrio. Liest gern, mag keine Kabel.
- **Ausrüstung:** Kontrabass ohne Tonabnehmer, USB-Mikrofon auf Stativ, älteres Notebook (Intel i5, 4. Generation) mit Kubuntu. Übt im Wohnzimmer, oft ohne Kopfhörer.
- **Vorwissen:** Bassschlüssel sicher, Tabs fremd. Wenig Computerwissen.
- **Ziel:** Standards mit Begleitung üben, Intonation prüfen, ohne Kopfhörer.
- **Frust:** Kleine Schrift, Fehlalarme durch Raumgeräusche, Begleitmusik, die das Mikrofon verwirrt.
- **Zitat:** „Ich spiele seit 40 Jahren. Wenn das Programm meint, ich treffe nichts, liegt es am Programm.“
- **Prüft:** Vor Phase 4: ehrliche Meldung, dass Mikrofon noch nicht unterstützt wird. Ab Phase 4: Mikrofon-Eingang, Echo-Abzug ohne Kopfhörer, weitere Toleranzen mit sichtbarer Erklärung, Leistung auf alter Hardware, große Schrift.
- **Journeys:** J-E2 (ab Phase 4), J-A4.

## P7 Sam Novak, sehbehindert

- **Alter, Lage:** 29, Softwaretester. Spielt E-Bass als Hobby. Sam nutzt die Pronomen they/them.
- **Ausrüstung:** E-Bass, Interface, Desktop mit Plasma, Bildschirmlupe auf 200 %, Hochkontrast-Farbschema. Rot-Grün-Sehschwäche (Deuteranopie).
- **Vorwissen:** Liest Tabs, Noten nur mit Vergrößerung.
- **Ziel:** Ohne Farbe als einzigen Hinweis erkennen, ob ein Ton getroffen war; alles mit Tastatur bedienen; Text und Lane skalieren.
- **Frust:** Rot/Grün als einziges Signal, feste Pixelgrößen, Elemente ohne Namen für den Screenreader, Bewegung, die sich nicht abschalten lässt.
- **Zitat:** „Grün heißt richtig? Für mich sind beide Kästchen ocker.“
- **Prüft:** Farbunabhängige Rückmeldung (Form, Symbol, Text), Skalierung 100 bis 200 % ohne abgeschnittene Inhalte, Kontrast in allen Stilen, Tastaturbedienung, „Bewegung reduzieren“, Barrierefreiheitsnamen.
- **Journeys:** J-F1, außerdem jede Journey einmal mit 200 % und Farbfilter Deuteranopie.
