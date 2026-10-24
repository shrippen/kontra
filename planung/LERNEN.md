# Kontra: Lernmodul „Lernen“ (Plan)

Stand: 25. Oktober 2026; Entscheidungen Runde 13 eingearbeitet (Abschnitt 11). Grundlage: [`KONZEPT.md`](KONZEPT.md) 5.1 Punkt 5,
[`DIDAKTIK.md`](DIDAKTIK.md), [`PERSONAS.md`](PERSONAS.md), Vorbild BassBuzz „Beginner to Badass“ (Konto des Eigentümers,
Kurs abgeschlossen; nur Aufbau angesehen, keine Inhalte übernommen).

## 1. Lücke

Kontra hat Übungen (150 in neun Stufen), Generator, Griffbrett-Quiz, Gehör, Werkstatt und Auswertung. Es **erklärt**
aber nichts: Wer nicht weiß, was eine Septime, ein Slap oder eine Synkope ist, findet in Kontra keine Antwort. Lea (P1)
nennt genau das als Frust („Fachbegriffe ohne Erklärung“); Übungsziele, Generator-Optionen und Technik-Zeichen benutzen
solche Begriffe ständig.

Das Lernmodul schließt die Lücke: **kurze Lektionen**, die einen Begriff oder eine Fertigkeit erklären, sofort am
echten Bass ausprobieren lassen und in vorhandene Übungen münden. Dazu ein **Glossar** „Was ist …?“, das überall
verlinkt ist, wo ein Fachbegriff steht.

## 2. Vorbild: BassBuzz „Beginner to Badass“

| Merkmal | BassBuzz | Übernehmen? |
|---|---|---|
| Aufbau | 15 Module zu 5–7 Lektionen (je 2–3 h), dazu Bonus-Module (Masterclass, Gear, Full Songs) | Ja: Kapitel mit wenigen Lektionen, kleiner |
| Lektion | Erklärvideo (in Teilen, 5–10 min), dann Workouts „Slow / Med / Full“ zum Mitspielen, Ressourcen (Theorie-Vertiefung, Gehörübung) | Ja, ohne Video: Erklärung, Ausprobieren, Übung in drei Tempi |
| Abschluss je Modul | ein bekannter Song, der das Gelernte bündelt | Ja, mit eigenen Stücken (Stufe 9, Generator); keine lizenzierten Songs |
| Reihenfolge | Technik und Rhythmus zuerst, Töne am Hals, Saitenwechsel, Noten lesen, Tempo, Dur, Synkopen + Akkorde, Moll, Taktarten, Stil-„Secrets“, Groove-Workout, Jammen mit Band | Ja, deckt sich mit DIDAKTIK (Groove vor Tönen, Akkordtöne vor Skalen) |
| Fortschritt | Haken je Lektion, Modul-Fortschritt in %, „Resume“, Lesezeichen | Ja |
| Lernplan | Zeitpläne 1, 3, 6 Monate als PDF | Als Vorschlag in der App (DIDAKTIK I12 Tagesplan) |
| Achievements | Abzeichen | Nein (These 7, LH-BEW-04), höchstens abschaltbar |
| Abschlussquiz | einmal am Ende | Besser: kurze Fragen je Lektion, später wiederholt (Abschnitt 5) |
| Grenzen | Video sieht den Spieler nicht, Selbstkontrolle | Kontras Stärke: hört zu und prüft jeden Schritt |

## 3. Bausteine

```
Kapitel (Lernpfad, Reihenfolge empfohlen, nie gesperrt)
 └─ Lektion (5–15 min)
     ├─ Erklären     kurzer Text, Abbildung (Griffbrett, Noten, Tab, Diagramm), Hörbeispiel
     ├─ Ausprobieren Kontra hört zu: „Spiel die Septime über A“, „Dämpfe die A-Saite“
     ├─ Üben         eine vorhandene Übung in drei Tempi (langsam, mittel, voll) oder Generator-Vorgabe
     └─ Prüfen       2–3 Fragen, Antwort per Bass oder Auswahl; kehren nach Tagen wieder
Glossar „Was ist …?“  ein Absatz + Abbildung + Link zur Lektion; aus jedem Fachbegriff erreichbar
```

- **Erklären:** höchstens ein Bildschirm je Schritt, Sätze kurz, Fachwort beim ersten Auftreten erklärt. Abbildungen
  sind lebendig (Kante-Griffbrett, Notenzeile, Tab-Lane mit Referenz-Bass zum Anhören), keine Bilder mit Text darin
  (Übersetzung, Skalierung, Screenreader).
- **Ausprobieren:** nutzt, was es schon gibt: Stimmgerät-Tonhöhe (Quiz, `appcore::quiz`), Gehör (`appcore::ear`),
  Session mit kurzem Arrangement. Kein Durchfallen; ein falscher Ton bekommt einen Hinweis („zu hoch“, „das ist die
  Sexte“). Ohne Eingang: Auswahl per Klick statt Bass.
- **Üben:** verweist auf Übungen aus `testdata/exercises` oder eine Generator-Vorgabe; „sitzt“ wie bisher (Zielband).
  Die drei Tempi sind Werkstatt-Tempi, kein neuer Modus.
- **Prüfen:** Fragen zum Wissen („Wie viele Halbtöne hat eine große Terz?“) und zum Hören. Spätere Wiederholung nach
  DIDAKTIK 4.3 (verteiltes Üben): fällige Fragen erscheinen auf der Startseite neben „Wiederholen“.

**Zwei Stränge** in einem Pfad, im Format getrennt markiert:

- **Instrument** (`instrument: bass`): Haltung, Stimmen, Anschlag und Wechselschlag, Greifen, Muting, Saitenwechsel,
  Slides, Hammer-on/Pull-off, Ghost und Dead Notes, Slap/Pop, Lagen. Gilt für das Instrument-Profil; Kontrabass (P6)
  und Keyboard-Bass (P5) bekommen später eigene Lektionen derselben Fertigkeit, Gitarre nach 1.0 ebenso.
- **Musik** (`instrument: any`): Töne und Halbtöne, Notenwerte und Pausen, Takt und Taktarten, Tab und Bassschlüssel,
  Intervalle (inkl. Septime), Dur und Moll, Tonarten, Dreiklänge und Septakkorde, Stufen und Akkordfolgen, Synkope,
  Triole, Shuffle, Form (Strophe, Refrain), Groove und Pocket. Instrumentneutral formuliert, Abbildungen kommen aus dem
  aktiven Profil (Griffbrett beim Bass, Klaviatur beim Keyboard).

## 4. Lernpfad (Entwurf)

Reihenfolge nach DIDAKTIK (Groove vor Tönen, Hören vor Lesen, Akkordtöne vor Skalen) und dem Vorbild. Jedes Kapitel
endet mit einem Stück aus vorhandenem Material.

| Kap. | Thema | Lektionen (Auswahl) | Üben mit | Abschluss |
|---|---|---|---|---|
| 1 | Loslegen | Der Bass und seine Teile · Stimmen · Anschlag mit zwei Fingern · Greifen ohne Kraft · Viertel im Takt | Stufe 1 „nur Timing“ | Groove Viertel I-IV-V |
| 2 | Rhythmus | Notenwerte und Pausen · Zählen · Achtel · Metronom und Drums · Tab lesen | Stufe 1–2 Grooves | Stück Pop (leicht) |
| 3 | Töne am Hals | Zwölf Töne, Halbton und Ganzton · Töne auf E und A · Oktaven · Grundton-Quinte | Griffbrett-Quiz, Stufe 1 | Grundton-Quinte-Groove |
| 4 | Saiten wechseln, Saiten dämpfen | Saitenwechsel · Muting mit beiden Händen · Fingerfolgen 1-2-3-4 | Stufe 1–2 Fingerübungen | Stück Rock |
| 5 | Intervalle | Was ist ein Intervall · Terzen · Quarte, Quinte · Sexte, **Septime**, Oktave · Intervalle hören | Griffbrett-Quiz Intervalle, Gehör | Gehör-Runde |
| 6 | Dur | Tonleiter · Tonarten und Vorzeichen · Dur in Lagen | Stufe 2, 7 | Generierte Übung Dur |
| 7 | Akkorde | Dreiklang · Arpeggio · Septakkorde · Stufen I-IV-V · Akkordnamen lesen | Stufe 2–3 | 12-Takt-Blues |
| 8 | Moll | Moll-Tonleiter · Moll-Dreiklänge · Pentatonik · diatonische Akkordfolgen | Stufe 2–3 | Stück Ballade |
| 9 | Synkopen und Sechzehntel | Synkope · Sechzehntel · Ghost Notes · Dead Notes | Stufe 3–4 | Stück Funk |
| 10 | Noten lesen | Bassschlüssel · Rhythmus lesen · Vorzeichen · Lesen ohne Lane (I2) | Stufe 7, Notenansicht | Übung in Noten ohne Lane |
| 11 | Taktarten und Triolen | 3/4, 6/8 · Triole · Shuffle · 5/4, 7/8 | Stufe 6 | Stück Shuffle |
| 12 | Techniken | Slide · Hammer-on, Pull-off · Slap und Pop · Oktav-Grooves | Stufe 8, J-C5 | Groove mit Technik |
| 13 | Walking Bass | Akkordtöne verbinden · Durchgangstöne · II-V-I | Stufe 5 | Walking über Blues |
| 14 | Groove und Pocket | Vor, auf, hinter dem Beat (I5) · Streuung · Ruf und Antwort mit dem Schlagzeug | Lage-Ziel, Histogramm | Groove laid-back |
| 15 | Mit Band spielen | Form eines Stücks · Akkordfolge folgen · eigene Linie über eine Folge | Generator „Stück“ | eigenes Stück, ganzer Durchlauf |

Umfang zum Start: Kapitel 1–5 (rund 25 Lektionen) und ein Glossar mit rund 60 Begriffen; der Rest danach.

## 5. Didaktische Regeln

1. **Erklären kurz, Spielen lang.** Ein Lektionsschritt hat höchstens einen Bildschirm Text; das Verhältnis Lesen zu
   Spielen ist etwa 1 : 3 (These „Groove zuerst“, Deliberate Practice 4.1).
2. **Sofort anwenden.** Auf jede Erklärung folgt ein Ausprobieren mit dem Bass; Kontra hört zu (Alleinstellung
   gegenüber Video-Kursen).
3. **Nichts sperren.** Der Pfad empfiehlt, sperrt aber nie (Jens, P2, will springen). „Kann ich schon“ prüft ein
   Kapitel mit seinen Prüffragen und hakt es ab.
4. **Wiederholen über Tage** (4.3, I3): Prüffragen kehren nach 1, 3, 7 Tagen wieder, falsch beantwortete früher.
5. **Rückmeldung ermutigend**, keine Punkte, keine Abzeichen (LH-BEW-04).
6. **Ehrlich:** Kontra ersetzt keinen Lehrer für Haltung und Handstellung; Lektionen dazu sagen das und zeigen, worauf
   man selbst achtet (Spiegel, Aufnahme).

## 6. Personas

| Persona | Braucht | Prüft |
|---|---|---|
| P1 Lea | Pfad von vorn, Begriffe erklärt, kleine Schritte, Erfolg in der ersten Lektion | Kapitel 1 ohne Vorwissen, Glossar aus Übungsziel |
| P2 Jens | Nur einzelne Antworten, nichts Pflicht, schnell zurück zum Lied | „Was ist …?“ aus der Auswertung und zurück in zwei Klicks; Kapitel überspringen |
| P3 Mira | Tiefe Theorie (Modi, Intervalle, Intonation bundlos), Tastatur | Kapitel 5, 6, 10, 13; alles ohne Maus |
| P4 Thomas | Lektionen am Beamer zeigen, später eigene Lektionen schreiben und weitergeben | Lesbarkeit bei 1280×720; Format offen und dokumentiert |
| P5 Aylin | Musik-Strang ohne Griffbrett-Zwang | Abbildungen aus dem Profil (Klaviatur ab Phase 4) |
| P6 Karl | Große Schrift, Kontrabass-Technik | Vor Phase 4: Instrument-Lektionen sagen, dass sie für E-Bass sind |
| P7 Sam | Text skalierbar, Abbildungen mit Textalternative, nichts nur über Farbe | J-F1 auf Lektionsseite und Glossar |

## 7. Architektur

```
ui/        LessonPage.qml (Schritt für Schritt), GlossaryPage.qml, „?“-Verweise an Fachbegriffen
  ↓        Kante-Bauteile; was fehlt, kommt zuerst nach Kante
appcore    Lektionsablauf: Schritte, Ausprobieren über die vorhandenen Wege quiz/ear/Session,
  ↓        JSON für QML (lessons_json, lesson_json, glossary_json)
session/   Kurs und Lektionen laden (für das aktive Instrument aufgelöst), Lernstand speichern
  ↓          ↘
engine/     content/  Lektionsformat: Parser, mitgelieferte Kurse, Lektionen, Glossar
```

- **Wer was besitzt:** `content` liest Dateien und kennt kein Instrument-Verhalten. `session` wählt den Kurs zum
  Instrument-Profil, löst Varianten auf (Abschnitt 12) und speichert den Lernstand. `appcore` führt durch die Schritte
  und prüft das Ausprobieren mit den Wegen, die es schon besitzt (`quiz`, `ear`, Session); nichts ruft nach oben.
- **Format:** je Lektion ein Ordner `lessons/units/<id>/` mit `de.md` und `en.md`. Kopf mit `id`, `for` (siehe
  Abschnitt 12), `needs`, `terms`; dann Markdown-Teilmenge (Überschrift, Absatz, Liste, Hervorhebung) und Blöcke für das
  Interaktive, z. B. `::: fretboard root=A steps="0 10"`, `::: listen root=A steps="0 10"`,
  `::: try interval=10`, `::: exercise …`, `::: question …`. Tonhöhen in Lektionen relativ (Stufen, Halbtöne über einem
  Grundton); Lage und Oktave wählt das Profil. Parser selbst gebaut im Crate `content` (keine neue Abhängigkeit),
  mitgeliefert über ein Build-Skript, das die Dateien einbettet.
- **Kurs:** `lessons/courses/<instrument>.json`: Kapitel mit Titel (DE/EN), Lektions-Ids und Abschlussstück.
- **Texte:** Lektionen sind Inhalt, keine UI-Texte; sie liegen je Sprache als Datei. Die UI-Texte der Lektionsseite
  laufen wie immer über Übersetzungsschlüssel. Ein Test prüft, dass jede Lektion und jeder Glossar-Eintrag in beiden
  Sprachen dieselben Blöcke und Ids hat.
- **Lernstand:** im vorhandenen Speicher, je Lektion erledigte Schritte und Prüffragen mit nächster Fälligkeit; Export
  und Import wie bisher (LH-FOR-03).
- **Verlinkung:** Übungsziele, Generator-Optionen, Technik-Zeichen und Auswertungssätze bekommen Glossar-Ids; die UI
  macht daraus „?“-Verweise. Glossar-Einträge verweisen auf ihre Lektion im aktiven Kurs.
- **Demo-Modus:** Lektionen sind eigenes Material, keine Nutzerdaten; Screenshots zeigen Lektionsseite und Glossar mit
  dem Lernstand der Demo-Welt.

## 8. Anforderungen (im Lastenheft, 4.7)

| ID | Anforderung | Prio | Personas | Abnahme |
|---|---|---|---|---|
| LH-LRN-07 | Lernpfad: Kapitel mit Lektionen, Fortschritt je Lektion und Kapitel, „Weiter“ auf der Startseite, nichts gesperrt. | Soll | P1, P2 | J-G1 |
| LH-LRN-08 | Lektion in Schritten Erklären, Ausprobieren (am Bass, ohne Eingang per Auswahl), Üben (vorhandene Übung, drei Tempi), Prüfen. | Soll | P1, P3, P7 | J-G1, J-G3 |
| LH-LRN-09 | Glossar „Was ist …?“ (DE/EN), aus jedem Fachbegriff in Übungsziel, Generator, Auswertung und Technik-Zeichen erreichbar, zurück an dieselbe Stelle. | Soll | P1, P2, P5 | J-G2 |
| LH-LRN-10 | Prüffragen kehren nach Tagen wieder; fällige Fragen auf der Startseite; „Kann ich schon“ hakt ein Kapitel nach bestandenen Fragen ab. | Kann | P1, P2 | J-G3 |
| LH-LRN-11 | Offenes, dokumentiertes Lektionsformat; eigene Lektionen in der Bibliothek, später im `.kontra`-Paket. | Kann | P4 | J-G4 |
| LH-LRN-12 | Lektionen gelten für eine Stufe (`any`, Familie, Instrument) aus dem Profil; ein Profil ohne passende Lektion bekommt einen Hinweis statt einer falschen Lektion. | Soll | P5, P6 | J-A4 |
| LH-LRN-13 | Kurs je Instrument aus einem gemeinsamen Lektionsvorrat (`for`, Varianten, `needs`, Abschnitt 12); gemeinsames Wissen zählt über Kurse. | Soll | P5, P6 | Tests je Kurs |

## 9. Journeys (`QA.md`, Abschnitt G)

- **J-G1 Erste Lektion** · P1 Lea · Kapitel 1, Lektion „Viertel im Takt“: lesen, ausprobieren, langsam üben, prüfen;
  Startseite zeigt danach „Weiter: …“. Bei 200 % Schrift.
- **J-G2 Was ist eine Septime?** · P2 Jens, P1 Lea · Walking-Übung II-V-I, „?“ am Akkordnamen „Dm7“ → „Was ist ein
  Septakkord?“ → weiter zu „Septime“; Hörbeispiel; zurück in die Übung an dieselbe Stelle. Dasselbe aus der Auswertung
  („Slide: 3 von 5“ → „Was ist ein Slide?“) und aus dem Generator mit unveränderten Einstellungen.
- **J-G3 Wiederholen** · P1 Lea · Prüffragen eines Kapitels, eine falsch; drei Tage später auf der Startseite fällig.
- **J-G4 Eigene Lektion** · P4 Thomas · Lektion als Datei schreiben, in Kontra öffnen, am Beamer zeigen.

## 10. Schritte

1. **Entscheidungen** (Abschnitt 11): erledigt.
2. **Format und Parser** in `content` mit `for`, Varianten und `needs` (Abschnitt 12), Kurs `bass.json`, drei
   Probelektionen und Glossar-Anfang; Tests für Parser, Sprachgleichheit und Kurs-Regeln.
3. **Kante:** Lernpfad (Kapitelliste mit Fortschritt), Lektionsschritt, Begriffs-Verweis; Test, README, Changelog.
4. **Session und appcore:** Lesson-Service, Fortschritt im Speicher, JSON für QML; Ausprobieren über Quiz/Gehör.
5. **UI:** Lektionsseite, Glossar-Seite, Startseite „Weiter“, „?“-Verweise; Journeys J-G1, J-G2 automatisiert (⚙).
6. **Inhalt:** Kapitel 1–5, dann der Rest; jede Lektion von P1 (verständlich) und P3 (fachlich richtig) gelesen.
7. **Wiederholen** (LH-LRN-10) und eigene Lektionen (LH-LRN-11).

Jeder Schritt ein eigener PR.

## 11. Entscheidungen (Runde 13, 25.10.2026)

| # | Frage | Entscheidung |
|---|---|---|
| E1 | Name im Menü | „Lernen“ / „Learn“ |
| E2 | Videos | keine: Text, lebendige Abbildungen, Hörbeispiele, Ausprobieren am Bass |
| E3 | Reihenfolge | Pfad empfohlen, alles offen, „Kann ich schon“ |
| E4 | Abzeichen | keine |
| E5 | Texte | Claude schreibt, der Eigentümer liest je Kapitel Stichproben |
| E6 | Lektionen und Übungsstufen | nebeneinander; Lektionen verweisen auf Übungen |
| E7 | Umfang vor 0.1 | alle 15 Kapitel vor 0.1; Beginn mit Kapitel 1–5 (Eigentümer, 25.10.) |

## 12. Kurse für weitere Instrumente

Der Kurs ist auf Bass ausgelegt. Kontra soll später auch andere Instrumente lehren (erst Saiteninstrumente wie Gitarre
und Ukulele, dann Tasteninstrumente); jedes bekommt einen **eigenen Kurs** mit eigener Reihenfolge und eigenen
Lektionen. Vieles überschneidet sich aber (Notenwerte, Intervalle, Tonleitern, Akkorde, Hammer-on auf Bass und
Gitarre). Damit das nicht doppelt geschrieben, doppelt übersetzt und auseinanderlaufend gepflegt wird, gilt schon für
den Bass-Kurs:

**Kurs und Lektion sind getrennt.** Ein Kurs ist nur eine Liste: Kapitel, Reihenfolge, Lektions-Ids, Abschlussstücke.
Die Lektionen liegen in einem gemeinsamen Vorrat (`lessons/units/`). Ein Gitarren-Kurs nimmt „Was ist ein Intervall“
aus demselben Vorrat, setzt es aber in ein anderes Kapitel.

```
lessons/
├── courses/   bass.json      guitar.json (später)   keys.json (später)
├── units/     intervall/  septime/  notenwerte/  …      für alle Instrumente   (for: any)
│              hammer-on/  slide/  muting/  …            für Saiten mit Bünden  (for: fretted)
│              slap/  anschlag-zwei-finger/  …           nur Bass               (for: bass)
└── glossary/  de.md  en.md   ein Glossar für alle Kurse
```

**Wofür eine Lektion gilt** (`for`), vom Allgemeinen zum Besonderen, abgeleitet aus dem Instrument-Profil:

| Stufe | Beispiele | Gilt für |
|---|---|---|
| `any` | Notenwerte, Takt, Intervalle, Dur/Moll, Akkorde, Groove | jedes Instrument |
| Familie: `strings`, `fretted`, `keys`, `voice` | Stimmen, Saitenwechsel (`strings`); Bünde, Hammer-on, Slide (`fretted`); Fingersatz auf Tasten (`keys`) | alle Instrumente der Familie |
| Instrument: `bass`, `guitar`, `ukulele`, `double-bass`, `piano` … | Slap und Pop, Anschlag mit zwei Fingern (Bass); Plektrum, Akkordgriffe (Gitarre) | nur dieses |

Eine Lektion steht auf der allgemeinsten Stufe, auf der sie wahr ist. Die Familie kommt aus dem Profil (Saiten und
Bünde, Tasten), nicht aus einer Liste im Code; ein neues Instrument ist damit ein Profil und ein Kurs, kein Umbau.

**Varianten statt Kopien.** Wo eine gemeinsame Lektion sich je Instrument nur in Teilen unterscheidet, bleibt sie eine
Lektion mit Varianten-Abschnitten:

```
::: variant for=bass
Am Bass liegt die Septime zwei Bünde unter der Oktave …
:::
::: variant for=keys
Auf der Tastatur ist die kleine Septime ein Ganzton unter der Oktave …
:::
```

Die Session löst Varianten beim Laden für das aktive Profil auf (das Besondere vor dem Allgemeinen). Fehlt für ein
Instrument eine nötige Variante, fällt das ein Test auf, nicht der Nutzer.

**Abbildungen und Tonhöhen aus dem Profil.** Blöcke beschreiben Musik, nicht das Instrument: `::: fretboard` wird
beim Keyboard zur Klaviatur, beim Kontrabass zum Griffbrett ohne Bünde; Tonhöhen stehen relativ (Grundton und
Halbtöne), das Profil legt Oktave und Lage in seinen Umfang. „Spiel die Septime über A“ ist am Bass A1 → G2, an der
Gitarre A2 → G3. So bleibt ein Ausprobieren gleich und wird für jedes Instrument richtig geprüft.

**Übungen als Vorgabe, nicht als Datei.** Wo es geht, verweist eine Lektion auf eine Generator-Vorgabe (Art, Tonart,
Tempo) statt auf eine Bass-Datei; der Generator erzeugt sie für das aktive Instrument. Mitgelieferte Dateien (Stufen
1–9) sind Bass-spezifisch und nur im Bass-Kurs.

**Gemeinsamer Lernstand für gemeinsames Wissen.** Der Lernstand hängt an der Lektions-Id, Lektionen `any` zählen über
Kurse hinweg: Wer Intervalle am Bass gelernt hat, sieht sie im Gitarren-Kurs als erledigt, mit „Wiederholen“ statt
„Neu“. Lektionen einer Familie oder eines Instruments zählen je Instrument, weil die Hand neu lernen muss. Prüffragen
zum Wissen wiederholen sich kursübergreifend nur einmal.

**`needs` statt Kapitelnummern.** Voraussetzungen nennen Lektions-Ids („septime braucht intervall“), nicht Kapitel.
Jeder Kurs ordnet anders, die Voraussetzungen bleiben gleich; ein Test prüft je Kurs, dass keine Lektion vor ihrer
Voraussetzung steht.

**Glossar einmal.** Ein Glossar für alle Kurse, Begriffe instrumentneutral, mit Varianten wie in Lektionen
(„Slap“ hat nur eine Bass-Variante und erscheint nur dort). Ein Begriff verweist auf die Lektion im aktiven Kurs.

**Pflege und Prüfung:** Tests je Kurs: jede Lektion gilt für das Instrument (`for`), jeder Block ist für das Profil
darstellbar und prüfbar, beide Sprachen gleich, `needs` in Reihenfolge. Eine Änderung an einer gemeinsamen Lektion
zeigt im PR, welche Kurse sie betrifft (Personas P1 und P5 lesen gegen).

**Für den Bass-Kurs heißt das jetzt:** jede neue Lektion bekommt die allgemeinste passende Stufe `for`, Tonhöhen
relativ, Übungen möglichst als Generator-Vorgabe; Bass-Eigenheiten stehen in `variant for=bass` statt im allgemeinen
Text. Mehr baut der Bass-Kurs nicht vor: kein Gitarren-Kurs, keine Klaviatur-Abbildung vor 1.0.
