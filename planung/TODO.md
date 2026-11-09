# Für später (Arian)

Notizen aus den Entscheidungsrunden, die bewusst aufgeschoben sind.

- **GitHub-Spiegel:** Bis 1.0 bleibt Kontra nur intern auf
  git.arianw.de. Danach entscheiden, ob es einen Push-Spiegel auf
  GitHub (`shrippen/Kontra`) gibt, wie bei den anderen Repos.

## Offen nach der Sitzung vom 30./31.10.2026

### Entscheidungen des Eigentümers (nächste Runde im Entscheidungs-Artefakt)
- LH-NF-15: Ausnahmen bestätigen oder nacharbeiten: `line-5` ab E1 74,7 ms, `hum-line` 72,0 ms (Ziel 70 ms).
- R4 `ON_TIME_MS` 10 ms: Neigung ±10 ms nur in ~10 % erkannt; Toleranz ändern?
- #202: Ersatzgerät verwirft Messungen endgültig (LH-EIN-05 verlangt das); Messungen je Gerät merken?
- Neue `pub`-Schnittstellen seit Runde 17 zur Freigabe listen (siehe `planung/STAND.md`: tech-hint, cleanup,
  input-state, shuffle-glyphs, noise-floor, fret-quiz, split-core, #222–#224).

### Hardware-Sitzung am Solo (etwa 1 h)
- Loopback/Kalibrierung, dabei Rauschboden-Schwelle (E6) einmal am Kabel, einmal mit Mikrofon.
- R1 Rücknahme beim Tippen, R8 Doppelgriffe aufnehmen, R7 Saiten, Stretch-WAVs (`~/.cache/claude/kontra-exp/dyadstretch/wav/`).
- #209/#213: Aussetzer und USB ziehen mitten im Lauf (Scarlett als Ein- und Ausgang), „Erneut versuchen“, Latenz und
  Xruns nach dem Wiederöffnen, J-F4 Schritt 3; Eingang ohne Zugriffsrecht.
- R3 Jitter gegen die 4-ms-Schwelle; R4 Sinus auf E2 gegen Zupfen.
- Slap/Pop am eigenen Bass: Hinweis an der Note, kein Fehler beim Zupfen (#211); eigene Slaps und Pops aufnehmen.
- MIDI-Keyboard: gleiche Auswahl behält Latenz (#214).

### Harmonicon-Liste (ROADMAP „Entscheidungen Harmonicon-Analyse“), noch offen
- A3, A4, L1, L2, L3, L7, L8, L9, L10, E4 (CI-Änderung: Bestätigung), E5, E7, M2, M3, M4 (nach 1.0),
  W1, W3, W4, W6, W8, W9.

### Kleinigkeiten (Details: `~/.cache/claude/kontra-exp/FOLLOWUPS.md`)
- #210: Rücknahme löscht das Technik-Urteil des Tipps nicht (braucht eigenes Session-Ereignis).
- #215: ♯/♭ im Fließtext hängen an der Systemschrift (Bravura nur mit Notenzeile geladen) → in Kante global laden;
  Glyphen-Test zählt nicht geladene Schriften. Kante selbst nutzt ✓/✕ ohne mitgelieferte Schrift.
- L11: Shuffle-Angaben importierter Dateien (Guitar Pro, MusicXML `<swing>`) nicht gelesen.
- E6: Schwelle nirgends angezeigt; Kabel-Messung (Loopback) behält Standard-Schwelle.
- #202 Bestand: SetupPage zeigt bei 0 ms „noch nicht gemessen“.

### Aufräumen
- Nach dem Merge von #220: Worktree `~/.cache/claude/kontra-wt/gt-cli` löschen.
- `~/.cache/claude` belegt ~73 GB (09.11.), davon 52 GB (31.10.) in Ordnern `studium-test-*` (Chrome-Profile einer anderen
  Sitzung, nicht angefasst): löschen?
