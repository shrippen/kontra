# Kontra

Bass-Trainer für KDE Plasma: hört am echten Bass zu (Klinke, Interface), Tab-Lane, Tabulatur und Noten laufen mit,
Rückmeldung zu Ton und Timing. Lernen oder einfach Lieder spielen. MIT-Lizenz.

Stand: Phase 0/1 in Arbeit. Die Erkennung erfüllt die Lastenheft-Ziele auf synthetischen Aufnahmen; Tests mit echten
Bässen stehen aus.

## Bauen

```sh
cargo test --workspace --exclude kontra-app      # Kern, ohne Qt
cargo run -p kontra-cli -- korpus check          # Erkennungsqualität (LH-NF-02/03)
sudo scripts/setup-qt-env.sh && . scripts/qt-env.sh
cargo run -p kontra-app                          # App (Qt 6.10, Kirigami, Kante)
```

## Ausprobieren ohne Bass

```sh
kontra-cli render testdata/demo/etude-1.mid take.wav --lead-in 2.4 --wrong-every 7
kontra --open testdata/demo/etude-1.mid --input-file take.wav --autoplay --mode song
```

## Aufbau

```text
crates/app       Qt-App (QML, Kirigami, Kante über cxx-qt)      crates/cli   Kommandozeile
crates/appcore   Qt-freier App-Kern (Einstellungen, Texte, Bibliothek)
crates/session   Transport, Modi, Bewertung, Fortschritt, Live-Üben
crates/engine    Audio-Ein-/Ausgabe, Analyse-Thread, Player, Testkorpus
crates/content   Import Guitar Pro 3–8, MIDI, MusicXML; Tab-Generator
crates/dsp       Tonhöhe (YIN + Phasenverfeinerung), Anschläge, Stimmgerät, Resampler, Synth
crates/stretch   Time-Stretch (Phasen-Vocoder)     crates/drivers  cpal, Audiodateien
crates/model     gemeinsame Typen
```

## Dokumente

- [Konzept](KONZEPT.md) mit Analyse von Rocksmith und Synthesia und den Entscheidungen
- [Lastenheft](LASTENHEFT.md) · [Personas](PERSONAS.md) · [QA-Journeys](QA.md) · [Roadmap](ROADMAP.md)
- [Arbeitsregeln](agent.md)
