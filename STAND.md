# Arbeitsstand

Übergabe zwischen Sitzungen. Stand: 2026-10-11 (Abend). Was erledigt ist, steht in Lastenheft und Roadmap; hier
steht nur, was offen ist und wo es liegt.

## Offen

- **Klaviatur-Ansicht** (J-E1, P5): PR #33 (Kante 1.30, Ansicht „Klaviatur“, Alt+5). Profilwahl „Keyboard“
  (`Instrument::keyboard`) erst mit den Tasteninstrumenten nach 1.0.
- **Techniken anzeigen und bewerten** (LH-BEW-06, Teil 2): `DetectedNote::technique` ist da; Session (Soll-Technik
  aus GP gegen erkannte), Auswertung („Slides 8 von 10“) und Kennzeichnung in Tab-Lane/Noten fehlen.
- **Transponieren mitten im Spiel**: der Backing-Track kann es (Pitch-Shifter im Mixer), die Session baut ihren
  Abgleicher aber beim Start; ein Wechsel beendet den Lauf.
- **MIDI**: bei mehreren Keyboards nimmt Kontra das erste; Port-Auswahl fehlt.
- **Stem-Trennung** (LH-BIB-09, Kann): nicht begonnen (bräuchte ein ML-Modell als Download).
- **Hardware-Prüfungen vor 0.1**: Klang (EQ, Wärme), Transponieren mit Bandaufnahme, Beamer 1280×720/150 %, echtes
  MIDI-Keyboard (bisher ALSA „Midi Through“), Mikrofon mit Lautsprechern (J-E2), 60-min-Soak, Mithör-Latenz.

## Für den Eigentümer

- OpenRouter-Schlüssel von PR-Agent: Monatslimit erreicht (Reset 1.11.); bis dahin scheitert jedes Review, es gilt
  „zweimal gescheitert, grüne CI genügt“.
