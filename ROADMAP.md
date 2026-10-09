# Kontra: Roadmap

Phasen und Inhalte: [`KONZEPT.md`](KONZEPT.md), Abschnitt 7. Anforderungen je Phase: [`LASTENHEFT.md`](LASTENHEFT.md).

| Phase | Ziel | Stand |
|---|---|---|
| 0 Machbarkeit | cxx-qt mit Kirigami und Kante, Audio-Engine, Erkennung gegen Testkorpus, eigener Time-Stretch, Latenz, Xruns | offen |
| 1 MVP (0.1) | E-Bass, Import GP/MIDI, Tab-Lane, Tab, Noten, Spielen/Song/Warten/Werkstatt | offen |
| 2 Lernen (0.2–0.4) | Groove, Griffbrett, adaptive Schwierigkeit, MusicXML | offen |
| 3 Inhalte (0.5–0.7) | eigene Audiodatei, `.kontra`-Pakete, Etüden, Gehör | offen |
| 4 Bass komplett (0.8–1.0) | Kontrabass am Mikrofon, Bass über MIDI, Techniken, Stem-Trennung | offen |

## Phase 0: nächste Schritte

- [ ] Cargo-Workspace, `make check` mit cargo fmt, clippy, Tests, cargo-deny
- [ ] cxx-qt mit Kirigami und `qml/Kante`: eine Seite mit Listenmodell aus Rust
- [ ] Audio-Durchschleifen mit cpal und mit pipewire-rs, Latenz und Xruns vergleichen
- [ ] Onset- und Tonhöhenerkennung (YIN/MPM) gegen erste Testaufnahmen
- [ ] Latenz Anschlag → Anzeige messen (Interface, Laptop-Buchse)
- [ ] Eigener Time-Stretch (Phasen-Vocoder), Qualität bei 50 % gegen Signalsmith Stretch als Referenz
- [ ] Testkorpus: Aufnahmen, Annotation, Lizenz klären
- [ ] Fehlende Kante-Komponenten als Vorschlag in Kante (`proposals/`)

## QA-Runden

Noch keine.
