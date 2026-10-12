# Arbeitsstand

Übergabe zwischen Sitzungen. Stand: 2026-10-12 (Morgen). Was erledigt ist, steht in Lastenheft und Roadmap; hier
steht nur, was offen ist und wo es liegt.

## Offen

- **Runde 10** (Entscheidungs-Artefakt): nächster Schwerpunkt, Ghost Notes und Hammer-on-Töne in der Trefferquote,
  Ort des Slide-Zeichens, Stem-Trennung im Flatpak, Demucs-Messung, Dead Notes (x) und Ghost Notes ( ).
- **Techniken an echten Aufnahmen** (LH-BEW-06): Slap/Pop/Ghost an IDMT 79/70/96 %, an denselben Aufnahmen
  abgestimmt; eigene Aufnahmen fehlen. Slides an echten Aufnahmen 0 von 20: Beim Gleiten meldet der Onset-Detektor
  neue Anschläge, schnelle Slides kommen als Hammer-on an; braucht eine Slide-Erkennung über die Tonhöhenkurve.
- **Flageoletts**: natürliche am Teilton (GP binär, GPIF); künstliche Flageoletts und MusicXML-Flageoletts fehlen.
- **Stem-Trennung** (LH-BIB-09): mit Ersatz-Programm getestet; mit echtem Demucs nicht gelaufen (Abnahme
  „4-Minuten-Song < 3 min“ offen). Im Flatpak ausgegraut (Erlaubnis fehlt bewusst).
- **MIDI-Auswahl** (LH-EIN-12): mit zwei echten Keyboards nicht geprüft (nur Unit-Test).
- **Hardware-Prüfungen vor 0.1** (zurückgestellt, Runde 9): Klang (EQ, Wärme), Transponieren mit Bandaufnahme,
  Beamer 1280×720/150 %, echtes MIDI-Keyboard, Mikrofon mit Lautsprechern (J-E2), 60-min-Soak, Mithör-Latenz,
  Techniken am echten Bass.
- **Profilwahl „Keyboard“** (`Instrument::keyboard`) erst mit den Tasteninstrumenten nach 1.0.

## Für den Eigentümer

- OpenRouter-Schlüssel von PR-Agent: Monatslimit erreicht (Reset 1.11.). Seit dem 11.10. abends startet ein
  `/review`-Kommentar auf Kontra gar keinen Lauf mehr (auch kein „skipped“); es gilt „zweimal gescheitert, grüne CI
  genügt“.
