# Kontra: Roadmap

Phasen und Inhalte: [`KONZEPT.md`](KONZEPT.md), Abschnitt 7. Anforderungen je Phase: [`LASTENHEFT.md`](LASTENHEFT.md).

| Phase | Ziel | Stand |
|---|---|---|
| 0 Machbarkeit | Go↔QML-Brücke, Audio-Engine, Erkennung gegen Testkorpus, Latenz, Xruns, GC-Pausen | offen |
| 1 MVP (0.1) | E-Bass, Import GP/MIDI, Tab-Lane, Tab, Noten, Spielen/Song/Warten/Werkstatt | offen |
| 2 Lernen (0.2–0.4) | Groove, Griffbrett, adaptive Schwierigkeit, MusicXML | offen |
| 3 Inhalte (0.5–0.7) | eigene Audiodatei, `.kontra`-Pakete, Etüden, Gehör | offen |
| 4 Bass komplett (0.8–1.0) | Kontrabass am Mikrofon, Bass über MIDI, Techniken, Stem-Trennung | offen |

## Phase 0: nächste Schritte

- [ ] miqt mit Kirigami und `qml/Kante` testen; sonst C++-Host mit Go als `c-shared`
- [ ] Audio-Durchschleifen mit malgo, Xruns und GC-Pausen messen
- [ ] Onset- und Tonhöhenerkennung (YIN/MPM) gegen erste Testaufnahmen
- [ ] Latenz Anschlag → Anzeige messen (Interface, Laptop-Buchse)
- [ ] Signalsmith Stretch über cgo einbinden, Qualität bei 50 %
- [ ] Testkorpus: Aufnahmen, Annotation, Lizenz klären
- [ ] Fehlende Kante-Komponenten als Vorschlag in Kante (`proposals/`)

## QA-Runden

Noch keine.
