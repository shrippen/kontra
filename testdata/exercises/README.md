# Mitgelieferte Übungen (LH-BIB-08)

Eigenes Material unter MIT, erzeugt von `kontra_content::exercise::bundled()`:

```sh
kontra-cli exercises testdata/exercises
```

Ein Ordner je Niveau (1 Einstieg bis 8 Technik). Die Nummer im Dateinamen ist der Platz im Plan; der Fortschritt hängt
am Pfad, darum werden neue Übungen nur angehängt (46–144 liegen hinter den alten ihrer Stufe), nie umbenannt oder
verschoben. Die App sortiert nach der Nummer. Runde 12 tauschte die Ordner 4 und 5 (Groove vor Walking Bass); die
App zieht Bibliothek und Setlisten beim Laden auf die neuen Pfade um.

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

Akkordnamen stehen als MIDI-Marker am Taktanfang; Grooves haben eine Schlagzeugspur auf Kanal 10. Techniken (`ghost`,
`dead`, `slap`, `pop`) und feste Griffe (`pos <Saite> <Bund>`, Saite 0 = E) stehen als Text-Ereignis an der Note.
Blues und Moll-Kadenz enden mit einem Schlusstakt auf der Tonika. Nicht von Hand ändern: die CI erzeugt den Satz neu
und vergleicht ihn mit diesem Ordner.
