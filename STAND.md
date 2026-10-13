# Arbeitsstand

Übergabe zwischen Sitzungen. Stand: 2026-10-13. Was erledigt ist, steht in Lastenheft und Roadmap; hier
steht nur, was offen ist und wo es liegt.

## Offene PRs (Reihenfolge zum Mergen)

Merges per `tea api …/merge` blockiert der Auto-Modus seit dem 13.10.; Eigentümer merged oder erlaubt es (Runde 11).
`/review` zweimal gefragt (PR-Agent bis 1.11. aus): grüne CI genügt.

1. #55 Kante 1.34 übernehmen (Dead-Note-Zeichen x, das #45 braucht).
2. #45, #46, #47 (vom 12.10.).
3. #48 Startseite, Navigation nach Tätigkeit (LH-ALL-06).
4. #49 GUI-Runde 13.10. (G3–G19; G18 offen), baut auf #48.
5. #50 Slides an echten Aufnahmen; #52 Oktavfehler Kontrabass baut auf #50.
6. #51 künstliche, getippte und MusicXML-Flageoletts.
7. #53 Halbtonfehler Kontrabass (nur Doku), nach #52.
8. #54 Laptop-Buchse: Brummfilter und Onset-Schwelle im Profil `laptop-jack` (IDMT-Simulation 37,9 → 87,7 % Recall), auf #52.
- Kante #58 (1.35, Loop in Tabulatur und Noten): Eigentümer merged. Danach `scripts/sync-kante.sh` und `loopStart`/
  `loopEnd` in `PracticePage.qml` an tabStaff, bassStaff, combinedBass, combinedTab binden (G9).

## Offen

- **Runde 11** (Entscheidungs-Artefakt): Merge-Erlaubnis, Modusnamen „Spielen“/„Song“, Ghost-Zeichen (F8), Startseite
  „nächste Übung“, Tastenkürzel, Slides über einen Bund, Termin Hardware-Prüfung.
- **Slides über einen Bund** sind von Hammer-ons nicht zu trennen (#50); Entscheidung Runde 11.
- **Oktavfehler** (#52): 30 FiloBass-Noten noch eine Oktave tief; 88 Noten einen Halbton tief: Ton steigt in den ersten 100 ms, Kontra entscheidet zu früh (#53, Doku); Korrektur nach dem Einschwingen = Runde 11.
- **Flageoletts** (#51): Bewertung oktavtolerant; künstliche ohne eigenes Zeichen in Kante. GP4-Arten 15/17 mit echter
  GP4-Datei gegenprüfen; MusicXML-Naturflageolett ohne Saite/Bund bleibt beim gegriffenen Ton.
- **Zweiter Leser 13.10.** (#50–#54): Befunde und Antworten in den PRs; offen: `heard()`-Testkopie (#50),
  doppelte Laptop-Simulation in `korpus measure` (#54), Einschwingen der Brummfilter am Gerät prüfen.
- **KanteKpi** ohne Namen für Screenreader: Kante #61 (1.36, nach #58); danach Kante synchronisieren.
- **Stem-Trennung** (LH-BIB-09): mit echtem Demucs nicht gelaufen; im Flatpak ausgegraut.
- **MIDI-Auswahl** (LH-EIN-12): mit zwei echten Keyboards nicht geprüft.
- **Hardware-Prüfungen vor 0.1** (Runde 10: zuerst): Klang, Transponieren mit Bandaufnahme, echtes MIDI-Keyboard,
  Mikrofon mit Lautsprechern (J-E2), 60-min-Soak, Mithör-Latenz, Techniken am echten Bass, Tastatur/Screenreader am
  Gerät.
- **Profilwahl „Keyboard“** erst mit den Tasteninstrumenten nach 1.0.

## Für den Eigentümer

- Runde 11 beantworten: https://claude.ai/artifact/EsQvZ5epqB3WeBgFEhytpN
- OpenRouter-Schlüssel von PR-Agent: Monatslimit erreicht (Reset 1.11.).
