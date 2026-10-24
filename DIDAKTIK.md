# Kontra: Didaktik

Stand: 9. Oktober 2026. Bestandsaufnahme und Thesen. Prototypen folgen nach 0.1 (Entscheidung des Eigentümers).
Grundlage: [`KONZEPT.md`](KONZEPT.md), [`LASTENHEFT.md`](LASTENHEFT.md), [`PERSONAS.md`](PERSONAS.md), [`ROADMAP.md`](ROADMAP.md) (Abschnitt „Didaktik“), [`QA.md`](QA.md).

Zur Beleglage: Die Forschung zu Lern-Apps für Instrumente ist dünn. Vieles in diesem Dokument stammt aus der allgemeinen Motorik-Forschung (Laboraufgaben wie Fingertippen oder Bewegungsbahnen) und wird auf den Bass übertragen. Das ist eine Annahme, kein Befund. Wo eine Aussage nur aus Lehrpraxis, Herstellerangaben oder Foren stammt, steht das dabei.

---

## 1. Kurzfassung (Thesen)

1. **Leistung beim Üben ist nicht Lernen.** Was mit Lane, Wartemodus und Dauerrückmeldung gelingt, muss Kontra auch ohne diese Hilfen prüfen, sonst misst es die Krücke statt den Spieler.
2. **Rückmeldung dosieren.** Zusammengefasste und seltener gegebene Rückmeldung fördert das Behalten eher als ein Urteil zu jeder Note; Kontra sollte Rückmeldung schrittweise ausblenden können.
3. **Groove vor Tönen.** Basslehrer nennen Timing als häufigsten Anfängerfehler, und Timing ist Kontras Alleinstellung; Anfänger sollten Rhythmus auf einer Saite spielen, bevor Griffbrett und Tonhöhe dazukommen.
4. **Schwierige Stellen isolieren und über Tage wiederholen.** Gute Übende arbeiten gezielt an der schwersten Stelle, und verteiltes Üben schlägt geballtes; Kontra sollte schwache Stellen merken und an späteren Tagen wieder anbieten.
5. **Schwierigkeit an den Spieler koppeln, aber transparent.** Eine Zone „fordernd, aber meist machbar“ ist plausibel; automatische Verdichtung mitten im Lied wie bei Rocksmith wird als frustrierend kritisiert, darum lieber Tempo und Ausdünnung vor dem Durchlauf anpassen und sagen, was passiert.
6. **Hören und Lesen als Ziel, nicht als Hürde.** Hören vor Lesen ist in der Musikpädagogik gut begründet; die Lane soll am Anfang tragen und später in Tab und Noten übergehen, damit Kontra nicht nur „Balken nachspielen“ lehrt.
7. **Motivation über Kompetenz und Wahl, nicht über Druck.** Gamification wirkt im Mittel klein bis mittel; Streaks und Punkte sind zweitrangig gegenüber sichtbarem Fortschritt, eigenen Liedern und echter Wahl.

---

## 2. Methoden

Die Spalte „Personas“ bezieht sich auf [`PERSONAS.md`](PERSONAS.md).

| Ansatz | Kern | Stärken | Schwächen | Passt zu |
|---|---|---|---|---|
| **Groove zuerst / Play-along** (z. B. BassBuzz „Beginner to Badass“ mit „100% play-along“, TalkingBass „Groove Trainer“ mit 60 progressiven Grooves) | Ab Tag eins zu Musik spielen, einfache Riffs sofort, Details (Töne am Hals) später nachliefern | Hohe Motivation, Timing und Zusammenspiel von Anfang an, Bass wird als Rhythmusinstrument erlebt | Theorie und Notenlesen bleiben optional und kommen oft zu kurz; Beleg vor allem Nutzerberichte | P1, P2, P5 |
| **Noten zuerst** (Hal Leonard Bass Method von Ed Friedland; Carol Kaye) | Notenschrift, Rhythmuslesen, Töne in den ersten fünf Bünden; bei Kaye früh synkopierte Rhythmen lesen, Dreiklänge in Lagen | Führt zu Lesefähigkeit, die Studio- und Ensemblearbeit verlangt; Rhythmuslesen schult Timing-Verständnis | Langsamer Einstieg, für Selbstlerner trocken; Kayes Bücher gelten Nutzern als wenig anfängertauglich (Forenaussagen) | P3, P4, P6 |
| **Gehör zuerst** (Suzuki-artig, „sound before symbol“; für Kontrabass gibt es eine Suzuki-Bassschule, hier nicht im Detail geprüft) | Erst hören und nachspielen (Echo, Rote, Improvisation), Notation später | Eine Übersichtsarbeit (Ausman 2022) findet Zusammenhänge mit besseren Rhythmus-, Gehör- und sogar Blattspielfähigkeiten; Gehörspiel stört das Lesen nicht | Forschungsbasis klein und uneinheitlich; schwer zu bewerten ohne Lehrer | P1, P3, P6 |
| **Lied/Tab zuerst** (Rocksmith, Fender Play, viele YouTube-Kurse) | Bekannte Songs, Tab oder Highway, Technik nebenbei | Schneller Erfolg, Songs motivieren; Fender Play mit klarem Tempo der Lektionen | Vereinfachte Arrangements bis zur Unkenntlichkeit; wenig Lesen, wenig Theorie; Abhängigkeit von der Anzeige | P1, P2 |
| **Akkordtöne vor Skalen** (TalkingBass: „chord tones should always be studied before scales“) | Grundton, Terz, Quinte, Oktave als Formen auf dem Griffbrett, dann Walking | Direkt nützlich für Basslinien und Walking; Griffbrett über Intervalle statt Bundnummern | Braucht Theorie-Grundlagen; für reine Lied-Spieler weniger attraktiv | P3, P4, P5 |
| **Technik-Bausteine** (TalkingBass „Beginner Bass Guitar“: Grundlagen, Greifhand, Hals, Rhythmus, Töne, Muting, Wechselschlag, Dur-Tonleiter, Riffs) | Feste Reihenfolge von Haltung über Muting bis Tonleiter | Systematisch, deckt die häufigsten Fehler ab | Ohne Lieder schnell langweilig | P1, P4 |

**Häufige Anfängerfehler laut Basslehrern** (Bass Musician Magazine 2025, BassBuzz-Forum; Lehrpraxis, keine Studien):

- Zu viel Aufmerksamkeit auf Töne, zu wenig auf Rhythmus; Empfehlung: auf die Bassdrum hören, mit Drums oder Metronom üben.
- Zu fest greifen, Daumen zu hoch; zu hart anschlagen.
- Offene Saiten klingen ungewollt mit (Muting mit beiden Händen fehlt).
- Greifen und Anschlagen nicht gleichzeitig, dadurch klingt der Ton wie ein Hammer-on.
- Tempo vor Genauigkeit; „um schnell zu spielen, langsam üben“.
- Greifhand bekommt alle Aufmerksamkeit, der Wechselschlag der Anschlagshand wird vernachlässigt.

**Reihenfolge** in den geprüften Kursen: Haltung und Stimmen, Greifhand, Rhythmus-Grundlagen, Töne am Hals, Muting, Wechselschlag, dann Tonleitern oder Akkordtöne, dann Walking. Notenlesen läuft bei TalkingBass parallel ab dem Grundkurs, bei Hal Leonard von Anfang an. Die genaue Kapitelreihenfolge von Hal Leonard Book 1 konnte ich nicht prüfen.

**Für Kontra relevant:** Muting und unerwünschtes Mitklingen sind für die Erkennung messbar (Ton klingt weiter, zweiter Ton erkannt). Das ist eine Chance für Rückmeldung, die kein Video geben kann. Es ist aber nicht geprüft, wie zuverlässig Kontras Erkennung das unterscheidet.

---

## 3. Apps und Spiele

| Produkt | Mechanik | Was trägt | Was fehlt / Kritik |
|---|---|---|---|
| **Rocksmith 2014 / Rocksmith+** | 3D-Highway, Dynamic Difficulty (Notendichte steigt mit Treffern), Riff Repeater (Abschnitt, Tempo, Schwierigkeit fix), Lektionen mit Lehrervideos (bis Slap, Walking), Session Mode mit virtueller Band (Tempo, Groove, Akkordwechsel wählbar), Guitarcade-Minispiele | Echtes Instrument; Riff Repeater als stärkstes Werkzeug; Highway verschwindet bei „Master“-Stufe (Bericht eines Rezensenten) | Dynamic Difficulty führt mitten im Lied ungeübte Varianten ein; Community rät oft, sie abzuschalten und lieber langsam bei voller Schwierigkeit zu üben; ausgedünnte Arrangements können schwerer sein, weil der Fluss fehlt; für Bass kein „erfahrener Spieler“-Schalter (Rezension Bass-Erweiterung). Studie Ly 2021 (4 Erwachsene, 60 Tage): Technik gelernt, aber wenig Motivation weiterzumachen; Transfer der Tab-Lesefähigkeit unklar. Herstellerstudien („schnellster Weg“) ohne veröffentlichte Methodik |
| **Synthesia** | Fallende Balken, Wartemodus, Hände getrennt, Tempo, Schleifen, beliebige MIDI-Dateien | Schneller Einstieg; Cremata & Powell 2016 (564 Schüler, Klasse 6 bis 12): schnellere Beherrschung einfacher Stücke und besseres Behalten nach 7 Tagen als Papiernoten, eMedia, YouTube | Lehrt Notenlesen kaum (verbreitete Kritik, keine Studie dazu gefunden); Studie nur Anfängerstücke, 7 Tage, Design aus der Kurzfassung nicht prüfbar |
| **Yousician** | Übungs- und Spielmodus; Tempo 25 bis 125 % in 5-%-Schritten mit „Auto-adjust speed“; „Wait To Play“ (Stück hält an); Schleife mit Vorzähler; Sterne und Punkte im Spielmodus; Lernpfad | Übungswerkzeuge wie in Kontra geplant; Gamification wird von Nutzern als motivierend genannt | Abo-Kosten, Erkennung über Handy-Mikro unzuverlässig, Lektionen „plateau and repeat“ (Nutzerbewertungen); keine unabhängigen Lernstudien gefunden |
| **Melodics** | Kurze Lektionen, Note färbt sich nach Timing, Zusammenfassung nach jedem Schritt, Übungsmodus (Loop, BPM) und Leistungsmodus, 5-Minuten-Tagesziel, Streaks, Grade 1 bis 14 | Klare Trennung Üben/Prüfen; Tagesziel klein; Fokus Timing | Effektivität nur Herstellerangaben |
| **Fender Play** | Video-Lektionen, Lieder auf jeder Stufe, Bass-Pfade Rock und Funk | Gutes Tempo, Technik am Anfang (Anschlag, Hand) | Wenig für Fortgeschrittene; Lieder stark vereinfacht; Video sieht Fehler nicht, Selbstbewertung nötig; kaum Notenschrift |
| **Moises** | Stem-Trennung (Bass entfernen), Tempo und Tonart ändern, Akkorderkennung, Klick zum Song | Play-along mit eigenen Songs, genau das will P2 | Keine Bewertung des eigenen Spiels; kein Lernpfad |
| **Duolingo (als Mechanik)** | Wiederholung nach Vergessenskurve (Half-Life Regression), Streaks, kurze Einheiten | HLR-Modell: rund 12 % mehr tägliche Aktivität im A/B-Test (Settles & Meeder 2016) | Messgröße ist Engagement, nicht Können; für motorische Fertigkeiten nicht geprüft |
| **Simply Guitar, GuitarTuna-Spiele, BandLab** | Hier nicht vertieft recherchiert | | Keine belastbaren Aussagen von mir |

**Muster über alle Produkte:** Keines der Produkte belegt Lernerfolg mit unabhängigen, kontrollierten Studien. Die gemeinsamen Werkzeuge (Loop, Tempo, Warten, Schwierigkeitsstufen) sind plausibel und decken sich mit Lehrpraxis. Wiederkehrende Kritik: Abhängigkeit von der Anzeige, vereinfachte Arrangements, Motivationsabfall nach der Anfangsphase, frustrierende Automatik.

---

## 4. Forschung

Beleglage: **stark** = mehrere Meta-Analysen oder viele Replikationen; **mittel** = gut belegt in der Motorik, aber wenig an Instrumenten oder mit offenen Streitpunkten; **schwach** = einzelne kleine Studien, Selbstauskünfte oder nur Theorie.

### 4.1 Deliberate practice

- **Was:** Gezieltes, anstrengendes Üben an Schwächen mit Rückmeldung erklärt Leistungsunterschiede (Ericsson, Krampe & Tesch-Römer 1993, Geiger).
- **Beleglage:** mittel. Übung zählt, aber weniger als behauptet: Meta-Analyse Macnamara, Hambrick & Oswald 2014: rund 21 % der Leistungsvarianz in Musik. Die Replikation der Geigerstudie (Macnamara & Maitra 2019, 39 Geiger) fand einen deutlich kleineren Effekt und bestätigte das Kernergebnis nicht.
- **Folgerung für Kontra:** Nicht „mehr Minuten“ belohnen, sondern gezielte Arbeit an schwachen Stellen anbieten. Übungszeit allein ist eine schwache Fortschrittsgröße.

### 4.2 Wie geübt wird, zählt mehr als wie lange

- **Was:** Bei 17 Pianisten sagten Übungsdauer und Wiederholungszahl die Leistung am nächsten Tag nicht vorher. Die Besten isolierten die schwere Stelle und arbeiteten daran, bis sie sicher war (Duke, Simmons & Cash 2009).
- **Beleglage:** schwach bis mittel (klein, korrelativ, eine Sitzung), aber gut mit Lehrpraxis vereinbar.
- **Folgerung:** Schwierigste Takte (LH-BEW-03) sind der richtige Einstieg. Kontra sollte das Isolieren aktiv vorschlagen.

### 4.3 Verteiltes Üben (spacing)

- **Was:** Verteiltes Üben schlägt geballtes. Meta-Analyse Donovan & Radosevich 1999: 63 Studien, mittlere Effektstärke 0,46 zugunsten verteilten Übens (Aufgaben allgemein, nicht nur Motorik). Lee & Genovese 1988: geballtes Üben senkt Leistung und, bei Behaltensmessung, auch Lernen.
- **Schlaf:** Walker et al. 2002 fanden Verbesserung über Nacht beim Fingertippen. Rickard et al. 2008 zeigten, dass der Effekt verschwindet, wenn Ermüdung durch geballtes Üben und Tageszeit kontrolliert werden. Schlaf schützt eher vor Vergessen, als dass er verbessert. Strittig.
- **Beleglage:** stark für spacing allgemein; mittel für Instrumente; Schlafeffekt strittig.
- **Folgerung:** Kurze Einheiten an mehreren Tagen; schwache Stellen am Folgetag wieder anbieten. Ein Plateau innerhalb einer Sitzung ist kein Grund zur Sorge; Kontra kann sagen „Morgen weiter“ statt „noch 20 Mal“.

### 4.4 Interleaving (gemischt statt geblockt)

- **Was:** Abwechselnd üben (A, B, A, B) statt erst A, dann B verbessert in der Motorik das Behalten (contextual interference).
- **Musik:** Carter & Grahn 2016: 10 Klarinettisten, gemischt geübte Stücke wurden besser bewertet, aber uneinheitlich zwischen Bewertern. Stambaugh & Demorest 2010 (Schüler): kein Effekt. Stambaugh mit Studierenden: Effekt bei Holz-, nicht bei Blechbläsern.
- **Beleglage:** mittel in der Motorik, schwach und gemischt bei Instrumenten.
- **Folgerung:** Als Option anbieten (2 bis 3 Stellen im Wechsel), nicht als Standard. Gut für Fortgeschrittene (P2, P3), fraglich für den ersten Kontakt (P1).

### 4.5 Chunking

- **Was:** Lange Bewegungsfolgen werden in Teilstücke zerlegt, die mit Übung zu längeren verschmelzen. Die Grenzen sind individuell (Sakai, Kitaguchi & Hikosaka 2003).
- **Beleglage:** mittel (Labor-Sequenzaufgaben, gut repliziert), für Bass übertragen.
- **Folgerung:** Lieder in Phrasen zerlegen und zusammensetzen; Kontra kann aus den Timing-Daten erkennen, wo ein Spieler zögert, und dort die Grenze eines Bausteins setzen (Hypothese, nicht geprüft).

### 4.6 Rückmeldung: Häufigkeit, Art und guidance hypothesis

- **Was:** Zu häufige, zu leicht nutzbare Rückmeldung verbessert die Leistung beim Üben, schadet aber dem Behalten, weil der Lernende die eigene Wahrnehmung nicht nutzt (Salmoni, Schmidt & Walter 1984). Winstein & Schmidt 1990: 50 % Rückmeldung besser behalten als 100 %. Bandbreiten-Rückmeldung (nur melden, wenn außerhalb der Toleranz; Sherwood 1988) ist eine Form davon.
- **Gegenbelege:** Nicht überall repliziert. Sonifikation (Ton statt Bild) kann die Abhängigkeit vermeiden: Dyer, Stapleton & Rodger 2017 (45 Personen, rhythmische Zweihandaufgabe) fanden den Vorteil sofort ohne Rückmeldung erhalten, nach 24 Stunden aber verschwunden. Blanco, Tassani & Ramirez 2021 (47 Anfänger, Tonhöhe treffen mit Stimme und Geige): Gruppe mit Echtzeit-Bildrückmeldung behielt den Gewinn. Echtzeit-Rückmeldung kann also bei Anfängern und bei Wahrnehmungsaufgaben (Tonhöhe) helfen.
- **KR und KP:** Ergebniswissen (getroffen, 18 ms zu früh) gegen Ausführungswissen (wie gespielt). Eine direkte Vergleichsstudie im Instrumentalunterricht habe ich nicht gefunden. Reine Richtig-/Falsch-Rückmeldung wirkt bei komplexen Aufgaben wenig; erklärende Rückmeldung mehr (Übersicht White, ACDA).
- **Beleglage:** mittel. Gut belegt in der Motorik, an Instrumenten widersprüchlich.
- **Folgerung:** Kontras Prinzip „jede Rückmeldung zeigt beides“ (KONZEPT 5.1, Punkt 3) ist für die Auswertung richtig, für die Anzeige während des Spiels aber zu prüfen. Rückmeldung pro Note abschaltbar oder ausblendbar machen, Zusammenfassung nach dem Durchlauf bevorzugen, und Lernen an einem Durchlauf ohne Hilfen messen.

### 4.7 Schwierigkeit: Challenge Point, Zone der nächsten Entwicklung, Flow

- **Was:** Lernen ist am größten, wenn die Aufgabe relativ zum Können fordernd ist; zu schwer schadet der Leistung, zu leicht dem Lernen (Guadagnoli & Lee 2004, Challenge Point Framework; verwandt mit Wygotskis Zone der nächsten Entwicklung und Csikszentmihalyis Flow). Wilson et al. 2019 leiten für eine Klasse von Lernalgorithmen eine optimale Fehlerrate von rund 15 % ab („85-%-Regel“).
- **Beleglage:** schwach bis mittel. Challenge Point ist ein Rahmenmodell mit wenig Anwendungsforschung (Scoping-Review Leeds Beckett). Die 85-%-Regel gilt für Modelle und binäre Klassifikation, nicht nachgewiesen für Menschen am Instrument.
- **Folgerung:** Ein Zielband (etwa 80 bis 90 % Treffer) ist ein vernünftiger Startwert für adaptives Tempo, muss aber im Prototyp geprüft werden. Kontras aktuelle Regel „+5 % nach 3 fehlerfreien Durchläufen“ verlangt 100 %; das ist eher streng.

### 4.8 Slow practice

- **Was:** Fast alle klassischen Musiker üben langsam (99,45 % in der Umfrage von Allingham & Wöllner). Genannte Funktionen: Informationslast senken, motorische Grundlage, Problemlösen, Gefühle regulieren.
- **Beleglage:** schwach. Selbstauskünfte; keine kontrollierten Studien gefunden, die langsam gegen schnell vergleichen. Langsam und schnell sind biomechanisch nicht gleich.
- **Folgerung:** Tempostufen behalten, aber immer auch das Zieltempo prüfen. Das Ende einer Werkstatt-Runde sollte ein Durchlauf im Originaltempo sein.

### 4.9 Mentales Üben

- **Was:** Vorstellen der Bewegung verbessert die spätere Leistung (Driskell, Copper & Moran 1994). Eine methodische Replikation (37 Studien, 1995 bis 2018) findet einen kleinen positiven Effekt (r = 0,131).
- **Beleglage:** mittel, Effekt klein.
- **Folgerung:** Niedrige Priorität. Denkbar: Stelle anhören und Griffe mitlesen, bevor man spielt (Referenz-Bass, LH-WIE-03).

### 4.10 Timing-Feedback und Mikro-Timing

- **Was:** Hörbare Unterschiede liegen bei rund 2,5 % der Schlaglänge (Friberg & Sundberg, zitiert nach Sekundärquelle). Frühauf, Kopiez & Platz 2013 (93 Musikstudierende, Drum-Pattern um ±15 und ±25 ms verschoben): quantisiert am besten bewertet, Abweichungen schlechter, frühe härter bewertet als späte. Senn et al. 2016: quantisierte und originale Profi-Aufnahmen gleich gut bewertet. Schlagzeuger, die „laid-back“ spielen sollen, verschieben die Snare um rund 17 ms.
- **Beleglage:** mittel für Wahrnehmung; keine Studie gefunden, die Timing-Training mit Vor-/Hinter-Rückmeldung bei Bassisten prüft.
- **Folgerung:** Kontras Zeitfenster „streng“ ±25 ms liegt im hörbaren Bereich, gut. „Vor/hinter dem Beat“ ist eine sinnvolle Sprache, aber nicht jede Abweichung ist ein Fehler. Für Fortgeschrittene (P3) kann eine absichtliche Lage (laid-back) ein Ziel sein. Die Tendenz (Mittelwert) und die Streuung getrennt zeigen: Streuung ist Ungenauigkeit, Tendenz kann Stil sein.

### 4.11 Motivation: Selbstbestimmungstheorie und Gamification

- **Was:** Motivation wird stabiler, wenn Kompetenz, Autonomie und Zugehörigkeit erfüllt sind (Evans 2015 für Musikpädagogik). Mehr als die Hälfte der Musikschüler hört bis 17 auf (zitiert in Frontiers 2024); Bedürfnisbefriedigung hängt mit Durchhalten zusammen (korrelativ). OPTIMAL-Theorie (Wulf & Lewthwaite 2016): Erwartung, Autonomie, externer Fokus fördern Lernen. Aber: Meta-Analyse McKay et al. 2022 zu selbstgesteuertem Üben: Vorteil nur in veröffentlichten Studien, bereinigt nicht von null unterscheidbar; McKay et al. 2023 ähnlich für gesteigerte Erwartung.
- **Gamification:** Sailer & Homner 2020: g = 0,49 kognitiv, 0,36 motivational, 0,25 Verhalten; die motivationalen Effekte waren in strengen Studien weniger stabil.
- **Beleglage:** SDT als Rahmen mittel (viel korrelativ); Autonomie als Lernverstärker schwach; Gamification klein bis mittel.
- **Folgerung:** Wahlmöglichkeiten anbieten, weil sie Menschen gefallen, nicht weil sie nachweislich schneller lernen. Kompetenz sichtbar machen (Verlauf, „vor einer Woche 70 %, heute 85 %“). Streaks und Punkte bleiben abschaltbar (LH-BEW-04) und sind nicht der Kern. Ermutigende Sprache ist für P1 wichtig; die Forschung dazu ist nach Bereinigung schwach.

### 4.12 Notenlesen gegen Lane

- **Was:** Die verbreitete Kritik, fallende Balken lehrten kein Lesen, ist plausibel, aber ich habe keine Studie dazu gefunden. Die einzige Vergleichsstudie (Cremata & Powell 2016) fand Synthesia beim schnellen Lernen einfacher Stücke vorn; Lesefähigkeit wurde nicht gemessen.
- **Beleglage:** schwach (keine Daten zur Abhängigkeit).
- **Folgerung:** Kontra sollte das selbst messen: Trefferquote mit Lane gegen Trefferquote mit Tab oder Noten allein.

---

## 5. Ideen für Kontra

Vorhandene Bausteine: Modi Spielen, Song, Warten, Werkstatt; Werkstatt mit Tempostufen 25 bis 150 % und automatischer Steigerung; Zeitfenster locker/normal/streng; Treffer dezent oder aus (LH-MOD-01); Timing-Histogramm und Tendenz; schwierigste Takte als Sprung in die Werkstatt; Referenz-Bass im Mixer; Fortschritt pro Stück mit bestem Ergebnis und Übungszeit; Tab-Lane, Tab, Noten.

Aufwand: S = Tage, M = ein bis zwei Wochen, L = mehr. Schätzungen ohne Code-Analyse.

### I1 Blind-Probe (Lernen statt Leistung messen)

*Prototyp umgesetzt (24.10., LH-LRN-05): Knopf „Ohne Hilfe prüfen“ in der Werkstatt und Vorschlag auf der Startseite
nach einer Nacht (Eigentümer). Offen: Behalten nach 1 und 3 Tagen auswerten, sobald echte Läufe vorliegen.*

- **Beschreibung:** Nach einer Werkstatt-Runde oder am Folgetag spielt man die Stelle einmal ohne Lane, ohne Treffer-Markierung, nur mit Backing Track und Vorzähler. Kontra wertet still und zeigt danach das Ergebnis neben dem mit Hilfe.
- **These/Quelle:** These 1, 2; guidance hypothesis (Salmoni et al. 1984; Winstein & Schmidt 1990); Kritik an Lane-Abhängigkeit.
- **Personas:** P1, P2, P3, P4 (Lehrer sieht, was wirklich sitzt).
- **Braucht:** Song-Modus mit ausgeblendeter Anzeige (Lane leer oder nur Taktlinien), Treffer „aus“, Speichern als eigener Lauftyp im Fortschritt.
- **Messgröße:** Differenz Trefferquote mit/ohne Anzeige; Timing-Streuung ohne Anzeige; Behalten nach 1 und 3 Tagen.
- **Aufwand:** S.

### I2 Ausblendende Lane (faded guidance)

- **Beschreibung:** Je sicherer ein Abschnitt sitzt, desto weniger zeigt die Lane: erst Bundzahlen weg (nur Rhythmus und Saite), dann nur Taktanfang, dann nichts. Bei Fehlern kehrt die Hilfe für diesen Abschnitt zurück. Alternativ Übergang Lane → Tab → Noten.
- **These/Quelle:** These 2, 6; guidance hypothesis; Bandbreiten-Rückmeldung (Sherwood 1988); Rocksmith blendet den Highway bei „Master“ aus.
- **Personas:** P1 (später), P2, P3; P4 für Unterricht.
- **Braucht:** Darstellungsstufen in der Kante-Tab-Lane, Sicherheit pro Abschnitt aus dem Fortschritt, Experiment-Schalter.
- **Messgröße:** Trefferquote und Streuung in der Blind-Probe (I1) mit und ohne Ausblenden; Abbrüche.
- **Aufwand:** M (Kante-Änderung nötig).

### I3 Stellen-Wiederholung über Tage

- **Beschreibung:** Kontra merkt sich schwache Takte (aus „schwierigste Takte“ und Werkstatt-Ergebnissen) und bietet beim nächsten Start „3 Stellen von gestern, 5 Minuten“ an. Abstände wachsen, wenn die Stelle hält (einfaches Leitner-Schema; später ggf. ein Halbwertszeit-Modell wie bei Duolingo).
- **These/Quelle:** These 4; spacing (Donovan & Radosevich 1999; Lee & Genovese 1988); Duke et al. 2009; Settles & Meeder 2016 als Mechanik.
- **Personas:** P2 (Refrain bis Freitag), P3, P1, P4.
- **Braucht:** Fortschritt pro Abschnitt mit Datum (LH-FOR-01 hat das als Muss), Startseite mit Vorschlag, Werkstatt mit vorbelegtem Bereich.
- **Messgröße:** Trefferquote im ersten Durchlauf eines neuen Tages (Behalten); Übungszeit pro Stelle bis „sitzt“; Anteil angenommener Vorschläge.
- **Aufwand:** M.

### I4 Groove-Start auf einer Saite

*Plan der mitgelieferten Übungen (Runde 12): Stufe 4 Groove vor 5 Walking Bass, Groove vor Tönen auch zwischen den Stufen.*
*Umgesetzt (24.10., LH-LRN-06): fünf Grooves der Stufe 1 auf leerer Saite bewerten nur den Anschlag. Offen: Schwelle der
Streuung, ab der Bünde kommen; Schalter „nur Timing“ im Generator.*

- **Beschreibung:** Erste Übungen nur auf der offenen E- oder A-Saite zu Drums: Viertel, Achtel, Pausen, Betonung auf 1. Bewertet wird nur der Anschlag (Onset), nicht die Tonhöhe. Erst wenn die Streuung klein ist, kommen Bünde dazu.
- **These/Quelle:** These 3; Lehrer nennen Rhythmus als häufigsten Anfängerfehler (Bass Musician Magazine 2025); Groove-zuerst-Kurse; Chunking (Teilfertigkeit isolieren).
- **Personas:** P1, P5 (Timing ist für sie genauso), P2 als Aufwärmen.
- **Braucht:** Generierte Arrangements (eine Saite, Rhythmusmuster), Bewertung „nur Timing“, Drum-Begleitung; teilweise Vorgriff auf das Groove-Modul (LH-LRN-01, Phase 2).
- **Messgröße:** Timing-Streuung (ms) über Sitzungen; Übungszeit bis Streuung unter Schwelle; Abbrüche in Woche 1 gegenüber Start mit Lied.
- **Aufwand:** M.

### I5 Vor/hinter dem Beat statt richtig/falsch

- **Beschreibung:** Rückmeldung nach dem Durchlauf in zwei Zahlen und einem Satz: Lage („im Schnitt 12 ms hinter dem Beat“) und Streuung („gleichmäßig ±9 ms“). Optional Aufgabe: „Spiel bewusst laid-back“ oder „auf den Punkt“, Kontra zeigt, ob die Lage gehalten wird.
- **These/Quelle:** These 2, 3; Mikro-Timing-Forschung (Frühauf et al. 2013; Senn et al. 2016); KR mit Erklärung wirkt mehr als richtig/falsch (Übersicht White).
- **Personas:** P3, P2, P5.
- **Braucht:** Timing-Histogramm und Tendenz sind da; Zielvorgabe für die Lage, Text-Bausteine.
- **Messgröße:** Abstand der mittleren Lage zum Ziel; Streuung; Selbstauskunft „verstanden, was ich ändern soll“.
- **Aufwand:** S.

### I6 Bausteine zusammensetzen (Chaining)

- **Beschreibung:** Kontra zerlegt ein Lied in Phrasen (aus Takten und Pausen, oder aus Stellen, an denen der Spieler zögert). Man übt Baustein 1, dann 2, dann 1+2, dann 3, dann 1+2+3. Rückwärts-Variante: letzte Phrase zuerst, damit man immer in Bekanntes hineinspielt.
- **These/Quelle:** These 4; Chunking (Sakai et al. 2003); Duke et al. 2009; Yousician „one part at a time“.
- **Personas:** P1, P2, P4 (Übungsfolgen bauen).
- **Braucht:** Werkstatt mit Bereichsliste und Abfolge; automatische Phrasengrenzen (einfach: alle 2 oder 4 Takte).
- **Messgröße:** Übungszeit bis Lied ganz durch mit ≥ 85 %; Behalten nach 2 Tagen.
- **Aufwand:** M.

### I7 Zielband statt „3 fehlerfrei“ (adaptives Tempo)

*Umgesetzt (Runde 5): Werkstatt hält 80–90 % Treffer, zwei Durchläufe darüber +5 %, zwei darunter −5 %.*

- **Beschreibung:** Werkstatt hält die Trefferquote in einem Band (Startwert 80 bis 90 %): darüber Tempo +5 %, darunter −5 %. Änderungen nur zwischen Durchläufen und mit Ansage („Tempo jetzt 75 %“). Am Ende ein Durchlauf im Zieltempo.
- **These/Quelle:** These 5; Challenge Point (Guadagnoli & Lee 2004); 85-%-Regel (Wilson et al. 2019, nur Modelle); Kritik an Rocksmiths Automatik mitten im Lied; Yousician „Auto-adjust speed“.
- **Personas:** P2, P3, P1.
- **Braucht:** Werkstatt-Steigerung existiert; Regel ändern, Experiment-Schalter, Band einstellbar.
- **Messgröße:** Zeit bis Zieltempo; Trefferquote im Zieltempo; Abbrüche; Vergleich mit heutiger Regel.
- **Aufwand:** S.

### I8 Hören vor Lesen (Ruf und Antwort)

- **Beschreibung:** Kontra spielt eine kurze Phrase mit dem Referenz-Bass (1 bis 2 Takte), die Anzeige bleibt leer. Man spielt nach. Nach zwei Versuchen erscheint die Tab als Hilfe. Für Anfänger mit Grundton und wenigen Tönen, für P3 mit Walking-Fragmenten.
- **These/Quelle:** These 6; sound before symbol (Ausman 2022; McPherson et al. 1997 nach Sekundärquelle); Gehör-Modul LH-LRN-03.
- **Personas:** P1, P3, P6 (ab Phase 4).
- **Braucht:** Referenz-Bass (LH-WIE-03), Warten-Modus ohne Anzeige, kurze Phrasen aus Liedern oder generiert.
- **Messgröße:** Trefferquote beim Nachspielen ohne Anzeige; Anzahl Versuche bis Hilfe; Behalten.
- **Aufwand:** M.

### I9 Griffbrett über Intervalle

- **Beschreibung:** Statt Bundnummern zeigt Kontra Formen relativ zum Grundton (Grundton, Terz, Quinte, Oktave). Aufgaben: „Spiel den Grundton, dann die Quinte darüber“ in wechselnden Lagen; dann Akkordtöne über eine Akkordfolge.
- **These/Quelle:** Lehrpraxis (TalkingBass: Akkordtöne vor Skalen); Carol Kaye: Dreiklänge in Lagen. Keine Studie gefunden.
- **Personas:** P3, P1 (später), P5 (kennt Harmonie, nicht das Griffbrett), P4.
- **Braucht:** Kante-Griffbrett mit Intervall-Beschriftung, generierte Übungen (LH-LRN-02, -04), Tonhöhenbewertung ohne Saitenvorgabe.
- **Messgröße:** Reaktionszeit bis richtiger Ton; Trefferquote in neuen Lagen (Transfer).
- **Aufwand:** L.

### I10 Gemischte Runde (Interleaving)

- **Beschreibung:** Statt eine Stelle 10 Mal: drei Stellen (auch aus verschiedenen Liedern) im Wechsel, je zwei Durchläufe. Als Option in der Werkstatt und in I3.
- **These/Quelle:** contextual interference; Carter & Grahn 2016 (klein, gemischt).
- **Personas:** P2, P3.
- **Braucht:** Werkstatt mit mehreren Bereichen in einer Warteschlange.
- **Messgröße:** Behalten am Folgetag gegen geblockt geübte Stellen (innerhalb einer Person vergleichbar).
- **Aufwand:** M.

### I11 Muting-Check

- **Beschreibung:** Kontra prüft, ob ein Ton aufhört, wenn er laut Arrangement aufhören soll (Pausen, Staccato), und ob offene Saiten mitklingen. Rückmeldung nach dem Durchlauf: „In Takt 5 klang die E-Saite in die Pause.“
- **These/Quelle:** Lehrpraxis (Mitklingen als häufiger Anfängerfehler); KP statt nur KR.
- **Personas:** P1, P2.
- **Braucht:** Erkennung von Ausklingen und Zweittönen; Machbarkeit unklar, erst am Testkorpus prüfen.
- **Messgröße:** Anteil sauberer Pausen über Sitzungen.
- **Aufwand:** L (Erkennung).

### I12 Kurzer Tagesplan mit Wahl

- **Beschreibung:** Startseite schlägt 10 Minuten vor: 2 Minuten Groove (I4), 5 Minuten Stellen von gestern (I3), 3 Minuten eigenes Lied. Jeder Teil abwählbar oder austauschbar. Fortschritt als Vergleich mit der eigenen Vorwoche, keine Rangliste, Streak optional.
- **These/Quelle:** These 7; SDT (Evans 2015); spacing; Melodics-Tagesziel; Gamification-Effekte klein (Sailer & Homner 2020).
- **Personas:** P1, P2; P3 schaltet ab.
- **Braucht:** Startseite, Fortschrittsdaten, I3 und I4.
- **Messgröße:** Tage mit Übung pro Woche; Abbrüche nach Woche 1 und 4.
- **Aufwand:** M (nach I3, I4).

---

## 6. Vorschlag: drei Prototypen nach 0.1

1. **I1 Blind-Probe** (S). Sie ist billig, nutzt vorhandene Bausteine (Song-Modus, Treffer aus, Fortschritt) und liefert die Messgröße, die alle anderen Prototypen brauchen: Lernen ohne Hilfe statt Leistung mit Hilfe. Ohne sie kann Kontra keinen Prototyp ehrlich auswerten. Sie prüft zugleich das größte Risiko des Lane-Ansatzes. I2 (ausblendende Lane) baut darauf auf.
2. **I3 Stellen-Wiederholung über Tage** (M). Verteiltes Üben ist der am besten belegte Befund in diesem Dokument. Die Daten (schwierigste Takte, Fortschritt pro Abschnitt) sind schon Muss-Anforderungen. Sie hilft P2 direkt („Refrain bis Freitag“) und macht Fortschritt für P1 sichtbar. I7 (Zielband) kann im selben Prototyp als Werkstatt-Regel mitlaufen, weil es nur eine Regeländerung ist.
3. **I4 Groove-Start mit I5 Vor/hinter-Rückmeldung** (M + S). Timing ist Kontras Alleinstellung, die Lehrpraxis nennt Rhythmus als häufigsten Anfängerfehler, und für P1 senkt eine Saite die Einstiegshürde. Histogramm und Tendenz existieren. Der Prototyp zieht Teile des Groove-Moduls (Phase 2) vor und zeigt, ob Kontra Timing wirklich besser vermittelt als Rocksmith.

**Wie messen:** Jeder Prototyp hinter einem Experiment-Schalter. Lokal gespeichert, keine Telemetrie (KONZEPT 5.1). Vergleich innerhalb einer Person (Stellen mit und ohne Methode) ist ohne viele Nutzer möglich. Messgrößen: Trefferquote und Timing-Streuung in der Blind-Probe, Behalten nach 1 und 3 Tagen, Übungszeit bis Ziel, Abbrüche. Für echte Aussagen braucht es echte Spieler; die Personas-Journeys prüfen nur Bedienbarkeit, nicht Lernerfolg.

**Offene Punkte:**

- Synthetische Eingaben reichen für Didaktik-Messungen nicht. Es braucht einige echte Testspieler (z. B. Schüler von Lehrern wie P4).
- Prinzip „jede Rückmeldung zeigt beides“ (KONZEPT 5.1) gegen These 2 abwägen: während des Spiels weniger, danach mehr.
- LH-MOD-06 (adaptive Notendichte) mit der Rocksmith-Kritik abgleichen: Ausdünnen vor dem Durchlauf und mit Ansage, nicht mitten im Lied.
- Ob Muting-Erkennung (I11) und Phrasengrenzen aus Zögern (I6) technisch zuverlässig gehen, ist ungeprüft.
- Nicht recherchiert: Simply Guitar, GuitarTuna-Spiele, BandLab, Scott's Bass Lessons im Detail (Lehrplan nicht gefunden), Kapitelreihenfolge Hal Leonard.

---

## 7. Quellen

Methoden und Lehrpraxis

- Hal Leonard, Bass Method Book 1 (Ed Friedland): https://halleonard.com/product/695068
- TalkingBass, Kursreihenfolge (Forenzitat der FAQ): https://forum.bassbuzz.com/t/talking-bass-course-descriptions-study-order/78380
- TalkingBass, Beginner Bass Guitar Course: https://members.talkingbass.net/product/beginner-bass-guitar/
- TalkingBass, Groove Trainer (Übersicht): https://www.talkingbass.net/wp-content/uploads/2023/04/Groove-Trainer-bundle-Breakdown-PDF.pdf
- BassBuzz, Our Story: https://www.bassbuzz.com/our-story
- BassBuzz-Forum, Beginner to Badass Reviews: https://forum.bassbuzz.com/t/beginner-to-badass-reviews/4459
- Carol Kaye, Erfahrungsbericht (Forum): https://surfguitar101.com/ygroup/thread/14554
- TalkBass, Bücher nur mit Notation: https://www.talkbass.com/threads/books-with-notation-only-and-no-tabs.1417984/
- Bass Musician Magazine (2025), Common Beginner Bass Mistakes: https://bassmusicianmagazine.com/2025/02/common-beginner-bass-mistakes-and-how-to-fix-them/amp/
- BassBuzz-Forum, Mistakes When First Learning Bass: https://forum.bassbuzz.com/t/mistakes-when-first-learning-bass/32082
- Ausman (2022), Sound Before Symbol Strategies and Beginning Band Performance Skills: https://journalhosting.ucalgary.ca/index.php/cjnse/article/view/75284

Apps und Spiele

- Ly (2021), Rocksmith als Lernwerkzeug, Masterarbeit Univ. Sydney: https://ses.library.usyd.edu.au/handle/2123/26621
- Rocksmith-Studie Uberlândia (Fallstudie): https://repositorio.ufu.br/handle/123456789/19686
- Rocksmith-Herstellerstudie (Pressebericht): https://godisageek.com/?p=121267
- Rocksmith 2014 Rezension (Dynamic Difficulty): https://www.gamegrin.com/reviews/rocksmith-2014-review/
- Rezension Rocksmith Bass-Erweiterung: https://theriffrepeater.com/review-rocksmith-bass-guitar-expansion/
- CustomsForge, Dynamic Difficulty revisited: https://customsforge.com/topic/24170-dynamic-difficulty-revisited-for-me-anyway/
- College Music Symposium, Rezension Rocksmith 2014 Remastered (Session Mode): https://symposium.music.org/volume-58-no-2/technology-and-online-resource-reviews/technologies-and-online-resources-reviews-rocksmith-rocksmith-2014-edition-remastered-a-comprehensive-musical-game-and-teaching-tool
- Ubisoft, Rocksmith+ Dev Diary Lessons: https://www.ubisoft.com/en-us/game/rocksmith/plus/news-updates/7ncvKJE9PCWmNWNrqmU5WJ/rocksmith-dev-diary-late-may-2022-lessons
- Cremata & Powell (2016), Digitally Mediated Keyboard Learning, J. of Music, Technology & Education 9(2): https://digitalcommons.montclair.edu/cali-facpubs/8
- Songscription, Kritik an Falling Notes (Wettbewerber): https://www.songscription.ai/blog/songscription-vs-synthesia
- Yousician Support, Practice vs Play Mode Bass: https://support.yousician.com/hc/en-us/articles/115005200829-Practice-vs-Play-Mode-Bass-
- Yousician Support, Best way to practice bass: https://support.yousician.com/hc/en-us/articles/115005254049-The-best-way-to-practice-bass-in-Yousician
- Yousician Nutzerbewertungen (Auswertung): https://kimola.com/reports/yousician-app-review-analysis-unlock-user-insights-app-store-us-148143
- Unstar, Kritik an Lern-Apps (1 bis 3 Sterne): https://unstar.app/blog/yousician-simply-piano-flowkey-fender-play-music-learning-apps-ranked-2026
- Melodics, How it works: https://melodics.com/how-it-works
- Melodics, Practice is fun: https://www.melodics.com/you
- Fender Play Review, Music Teacher Magazine: https://musicteachermagazine.co.uk/content/review/reviews-fender-play
- Fender Play Review, Australian Guitar: https://www.pressreader.com/australia/australian-guitar/20210315/281694027523937
- Moises (App Store): https://apps.apple.com/us/app/1515796612
- Settles & Meeder (2016), A Trainable Spaced Repetition Model for Language Learning, ACL: https://preview.aclanthology.org/landing_page/P16-1174
- Duolingo Blog, How we learn how you learn: https://blog.duolingo.com/how-we-learn-how-you-learn

Forschung

- Ericsson, Krampe & Tesch-Römer (1993) und Replikation Macnamara & Maitra (2019): https://pmc.ncbi.nlm.nih.gov/articles/PMC6731745/
- Macnamara, Hambrick & Oswald (2014), Meta-Analyse deliberate practice (Pressemitteilung): https://www.sciencedaily.com/releases/2014/07/140716123835.htm
- Hambrick et al. (2014), Facing facts about deliberate practice: https://pmc.ncbi.nlm.nih.gov/articles/PMC4101876/
- Platz et al. (2014), Meta-Analyse Musik: https://dx.doi.org/10.3389/fpsyg.2014.00751
- Duke, Simmons & Cash (2009), It's Not How Much; It's How, JRME 56(4), DOI 10.1177/0022429408328851 (Zusammenfassung): https://www.structural-learning.com/research-briefings/practise-the-hard-bit
- Donovan & Radosevich (1999), Distribution of practice effect: https://academicnewsletter.sufe.edu.cn/info/372811
- Lee & Genovese (1988), Distribution of practice in motor skill acquisition: https://www.gwern.net/doc/psychology/spaced-repetition/1988-lee.pdf
- Walker et al. (2002), Practice With Sleep Makes Perfect: https://walkerlab.berkeley.edu/reprints/Walker%20et%20al._Neuron_2002.pdf
- Rickard et al. (2008), Sleep does not enhance motor sequence learning: https://researchconnect.suny.edu/en/publications/sleep-does-not-enhance-motor-sequence-learning/
- Simmons & Duke (2006), Effects of Sleep on Performance of a Keyboard Melody, JRME 54(3), DOI 10.1177/002242940605400308 (nicht im Volltext gelesen)
- Carter & Grahn (2016), Optimizing Music Learning: https://www.frontiersin.org/articles/10.3389/fpsyg.2016.01251/full
- Sakai, Kitaguchi & Hikosaka (2003), Chunking during human visuomotor sequence learning, Exp Brain Res 152, DOI 10.1007/s00221-003-1548-8
- Salmoni, Schmidt & Walter (1984), Knowledge of results and motor learning, Psychol Bull 95(3) (zusammengefasst in): https://www.frontiersin.org/articles/10.3389/fnins.2016.00251/full
- Winstein & Schmidt (1990) und Sherwood (1988), Bandbreiten-Rückmeldung (Übersicht): https://pmc.ncbi.nlm.nih.gov/articles/PMC7205416
- Dyer, Stapleton & Rodger (2017), Sonification as concurrent feedback, Psychol Res 81(4): https://pmc.ncbi.nlm.nih.gov/articles/PMC5486555
- Blanco, Tassani & Ramirez (2021), Visual and Auditory Feedback in Violin and Singing Pitch Matching: https://www.frontiersin.org/journals/psychology/articles/10.3389/fpsyg.2021.684693/full
- White, Übersicht Feedback in der Musikpädagogik (ACDA): https://acda.org/wp-content/uploads/2025/12/IJRCSVol13White.pdf
- Hamond, Welch & Himonides, KR/KP im Klavierunterricht: https://discovery-pp.ucl.ac.uk/10087930/7/782-2843-1-PB.pdf
- Guadagnoli & Lee (2004), Challenge Point Framework: https://en.wikipedia.org/wiki/Challenge_point_framework
- Scoping-Review Challenge Point Framework (Leeds Beckett): https://eprints.leedsbeckett.ac.uk/id/eprint/12281
- Wilson et al. (2019), The Eighty Five Percent Rule for optimal learning: https://pmc.ncbi.nlm.nih.gov/articles/PMC6831579
- Allingham & Wöllner, Slow practice (Dissertation Hamburg): https://ediss.sub.uni-hamburg.de/handle/ediss/9763
- Driskell, Copper & Moran (1994), Mental practice, DOI 10.1037/0021-9010.79.4.481; Replikation 1995 bis 2018: https://pure.ul.ie/en/publications/does-mental-practice-still-enhance-performance-a-24-year-follow-u/
- Frühauf, Kopiez & Platz (2013), Music on the timing grid: https://labs.sonicfield.org/library/music-on-the-timing-grid-the-influence-of-microtiming-on-the-perceived-groove-qu
- Senn et al. (2016), Groove und Mikro-Timing: https://pmc.ncbi.nlm.nih.gov/articles/PMC5050221
- Kilchenmann & Senn (2015), Microtiming in swing and funk: https://www.frontiersin.org/journals/psychology/articles/10.3389/fpsyg.2015.01232/full
- Evans (2015), Self-determination theory in music education: https://selfdeterminationtheory.org/wp-content/uploads/2021/05/2015_Evans_SDT_MusicEdu.pdf
- Frontiers (2024), Dropout und SDT an Musikschulen: https://www.frontiersin.org/journals/psychology/articles/10.3389/fpsyg.2024.1378843/pdf
- Wulf & Lewthwaite (2016), OPTIMAL theory: https://oasis.library.unlv.edu/kns_fac_articles/123
- McKay et al. (2022), Self-controlled motor learning meta-analysis: https://conferences.lnu.se/index.php/metapsychology/article/view/2803
- McKay et al. (2023), Reporting bias in expectancy and self-control research: https://raw.github.com/cartermaclab/carterlab-papers/main/2023_mckay-etal_the-combination-of-reporting-bias-and-underpowered-study-designs-has-substantially-exaggerated-the-motor-learning-benefits-of.pdf
- Sailer & Homner (2020), The gamification of learning: a meta-analysis: https://link.springer.com/article/10.1007/s10648-019-09498-w
