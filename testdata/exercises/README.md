# Mitgelieferte Übungen (LH-BIB-08)

Eigenes Material unter MIT, erzeugt von `kontra_content::exercise::bundled()`:

```sh
kontra-cli exercises testdata/exercises
```

Ein Ordner je Niveau (1 Einstieg bis 9 Stücke). Die Nummer im Dateinamen ist der Platz im Plan; der Fortschritt hängt
am Pfad, darum werden neue Übungen nur angehängt (46–150 liegen hinter den alten ihrer Stufe), nie umbenannt oder
verschoben. Die App sortiert nach der Nummer. Runde 12 tauschte die Namen der Ordner 4 und 5 (Groove vor Walking Bass),
die Übungen zogen erst am 25.10. mit. Findet die App eine Übung aus Bibliothek oder Setliste nicht mehr in ihrem
Ordner, sucht sie sie beim Laden nach dem Dateinamen in den anderen.

| Stufe | Inhalt | Für |
|---|---|---|
| 1 Einstieg | Viertel und Achtel auf leeren Saiten, Pausen, I-IV-V und I-V-VI-IV, Grundton-Quinte, Fingerübung 1-2-3-4, erste Tonleitern | P1 |
| 2 Grundlagen | Achtel über Akkordfolgen, 12-Takt-Blues, Dreiklänge, Dead Notes, Fingerfolgen, Dur, Moll, Pentatonik | P1, P2 |
| 3 Fortgeschritten | Shuffle, Sechzehntel, Oktaven, Arpeggios (Septakkorde), Modi, Terzen, Tresillo, Offbeat, Triolen, Tempo-Treppe | P2, P3 |
| 4 Groove | Grooves auf Drums: Funk, Ghost Notes, Synkopen, Triolen-Blues | P2, P5 |
| 5 Walking Bass | II-V-I, Moll-II-V-I, I-VI-II-V, Blues in vielen Tonarten | P3, P6 |
| 6 Taktarten | 3/4, 6/8, 5/4, 7/8 | P3 |
| 7 Lesen | Dur in allen Tonarten, Terzen, Vierergruppen, harmonisch Moll, Modi, Arpeggios in entfernten Tonarten | P3, P4 |
| 8 Technik | Slap und Pop, Fingerfolgen in allen Lagen, Tempo-Treppen | P2, P3 |
| 9 Stücke | Ganze Stücke mit Form und Schlagzeug: Pop, Rock, Funk (Intro, Strophe, Refrain, Bridge, Outro), Ballade (ohne Bridge), Shuffle (vier Blues-Chorusse); ein Fill vor jedem neuen Teil | P1, P2, P4 |

Akkordnamen stehen als MIDI-Marker am Taktanfang; Grooves haben eine Schlagzeugspur auf Kanal 10. Techniken (`ghost`,
`dead`, `slap`, `pop`) und feste Griffe (`pos <Saite> <Bund>`, Saite 0 = E) stehen als Text-Ereignis an der Note.
Blues und Moll-Kadenz enden mit einem Schlusstakt auf der Tonika. Nicht von Hand ändern: die CI erzeugt den Satz neu
und vergleicht ihn mit diesem Ordner.
