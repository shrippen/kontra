# Forschung zur Audio-Engine: Messberichte (29.10.2026)

Je Punkt ein Prototyp, gemessen vorher/nachher auf denselben Daten. Zusammenfassungen der Berichte; die Prototyp-Branches `exp-*` blieben lokal. Plan und Stand: [ROADMAP.md](ROADMAP.md), Abschnitt „Forschung zur Audio-Engine“.

## pitch-fast (Punkte 1, 2), 29.10.2026 — Zusammenfassung
Prototyp: Branch exp-pitch-fast (nur lokal), Env-Schalter KX_*, Rohdaten out/, outw/.

Punkt 1 wachsendes YIN-Fenster (L=1,06 + Bestätigung, volle Schätzung revidiert beide Richtungen):
IDMT p50 78,9 → 61,0 ms (E2–D#3 79 → 52), korpus check p95 line-4 81 → 69, line-5 100 → 80 ms.
Endgenauigkeit gleich (91,6 → 91,3 %), erste Meldung 1–2,5 Punkte öfter falsch, FiloBass kein Gewinn,
nsynth Präzision 99,4 → 95,7 % (Vorläufer/Zweitburst), fretless-5 Cent-Grenze FAILED, 4 Tests rot.
Nötig: estimate_short ohne Allokation, Revision nach unten + nur Cent, Ereignis „Note zurückgenommen“,
vorläufiges Urteil in der Session. Aufwand M. Unbewiesen: Fingertipp-Vorläufer am Bass des Eigentümers (~5 min).

Punkt 2 erwartete Note: Gewinn zum großen Teil Bias. Locker: IDMT 95 % aber falsche Treffer bei +1 von
16 auf 47 (FiloBass −1: 53 → 97). Mit Bestätigung durch das volle Fenster: Endurteil unverändert (keine
zusätzlichen falschen Treffer), erste Meldung IDMT +3,2, FiloBass +8,3 Punkte; kaum schneller (58 vs 61 ms).
Nur als Torwächter der ersten Meldung, nie als Entscheider. Aufwand M–L (Tab-Zeit → Sample-Zeit, hängt an 3).
Toleranzen auf denselben Daten gewählt; echte Spielfehler unbewiesen.

## timing (Punkte 3, 4), 29.10.2026 — Zusammenfassung
Prototyp: Branch exp-timing (1042985, nur lokal). Simulation sim3.py/sim3.out, Lean-Versuch exp_lean.rs, lean.out.

Punkt 3 Gerätestempel + DLL: bringt etwas. Heute exakt bei festem Quantum mit Echtzeit (p95 0,08 ms);
ohne Echtzeit unter Last nur 70 % ≤5 ms; Quantum 128→1024 zwischen Kalibrierung und Lauf 37 ms Fehler
(passt zur Messung 14.10.); zwei Geräte ±50 ppm nach 5 min p95 24 ms. DLL: überall ≤0,12 ms, sofern
capture-Semantik (erster Frame) stimmt; sonst 1 Quantum daneben. cpal: PipeWire capture = now−delay
(CLOCK_MONOTONIC, delay ohne queued/buffered); ALSA htstamp ab Prepare je Stream; Abbildung über Stream::now().
Offen: Loopback am Solo (~15 min, 10×check loopback, mit/ohne RT, force-quantum 1024). Aufwand M.

Punkt 4 Klick-Kalibrierung schluckt Lean: bringt klar. Mit echtem Code: Fehler = −Lean (bzw. Latenz bei
max(0)). 15 ms Früh-Spieler gilt bei Ziel „Auf den Punkt“ in 83–99 % als gehalten. LH-EIN-14 kommt bei
echtem Lean nie, ohne Fehler in 25–36 % (σ10) wegen ±4 ms Median-Fehler aus 8 Klicks; Begleitung vs.
nackter Klick → Schleife „zu spät“. Mit Loopback: richtig, aber LH-EIN-14 rät dann bei echtem Lean
falsch → LH-EIN-14/LH-BEW-09 mit umbauen. Nebenbefund ON_TIME_MS 10 → Lean ±10 ms nur 10 % erkannt.
Aufwand S–M. Eigentümer ~10 min: loopback R, 3× calibrate K, Lean = K−R, ein Lied mit K und R.

## pyin (Punkt 5), 29.10.2026 — Zusammenfassung
Prototyp: Branch exp-pyin (nur lokal) (cca9f60, Basis ba3bdb5 vor #193). Env-Schalter KONTRA_PYIN*, KONTRA_BASE_GATE.

Urteil: bringt nichts. Gleiche Meldezeit: IDMT 91,6 → 89,9–90,5 %, mehr Halbtonfehler (19 → 30–35),
FiloBass gleich/schlechter, idle-line 41 → 48/min. Thump-Regel nicht ersetzbar.
Held-out (Dateien geteilt): Vorteil nur durch Warten; main mit Konfidenz ≥0,9 für die Übereinstimmung
ist gleich gut (FiloBass 66,9 → 72,3 %, Median-Meldezeit +~35 ms).
Voicing bringt keine neue Information (hängt an derselben CMNDF-Mulde); trennt Dead Notes schlechter.
Nebenbefund für 1/2: adaptiv früh melden bei klarer Mulde, sonst warten; erwartete Note könnte das Warten verkürzen.
idle-room: 26 tonlose Falschnoten sind ein Onset-Problem (vor #193 gemessen).

## techstring (Punkte 6, 7), 29.10.2026 — Zusammenfassung
Prototyp: Branch exp-techstring (8763ca4, nur lokal): Konverter idmt-sb, KONTRA_STRIKE_LOG.
Daten: IDMT-SMT-Bass 1,58 GB in testdata/korpus/local/dl/idmt-sb (gitignored). Achtung: conv/idmt-st ist
veraltet (ohne Technik-Feld), `korpus techniques` darauf zählt alles als plain.

Punkt 6: Schwellen stabil über Dateien (LOFO kaum Abfall), aber an den einen Bass angepasst: auf SB
(3 Bässe) Slap 42 %, Pop 15 % Recall (ST in-sample 80/66). Vibrato 15,6 % als Muted. Abeßer-Merkmale +
logistische Regression: mit Training über mehrere Bässe Slap/Pop stark besser (z. B. Pop 12→75 %),
aber Plain-Fehlalarme bis 26 %. Ein Bass trägt kein Modell. Sofort (S): Konverter + SB als 2. Satz,
Schwellen auf ST+SB. Echt (M–L): eigene Aufnahmen mehrerer Bässe/Spieler.

Punkt 7: Saitenerkennung über Inharmonizität (log10 B = a_Saite + Bund·log10(2)/6), held-out je Bass:
93,8 % (Zufall 41,5 %, tiefster Bund 44,2 %); IDMT-ST-Linien 76 % (>0,5 s Noten 90 %, <0,2 s 60 %);
Palm-Mute ≈ Zufall. Urteil ~280 ms nach Anschlag. Risiko: andere Mensur/Saiten/5-Saiter verschieben
a_Saite. Aufwand M, Kandidaten aus der erwarteten Note.

## dyadstretch (Punkte 8, 9), 29.10.2026 — Zusammenfassung
Prototyp: Branch exp-dyadstretch (1fdf151, nur lokal), Code in crates/exp.

Punkt 8 Doppelgriffe: heute bei Doppelgriff 21–54 % falscher Ton, nie „zwei Töne“. Klapuri 2006
informiert (Tab gibt beide Töne vor), ohne Oktaven: IDMT-Mischungen 95 % erkannt, Fehlalarm 2,1 %;
NSynth 87 % / 0,4 %; synthetischer Korpus 0 Fehlalarme. Oktaven 0 %. Blind 45 % / 5,7 %: bringt nichts.
Urteil ~160 ms nach Anschlag, 0,43 ms je Einsatz. Aufwand M (erwarteter Doppelgriff von der Session).
Unbewiesen: echte Doppelgriffe (Eigentümer ~15 min Aufnahmen).

Punkt 9 Stretch: Bandteilung bringt nichts (Fine-Latenz oder verwaschene Anschläge). Nebenbefund:
`lead` win/2 → win/16 spart 37 ms (Standard 133→96) bzw. 75 ms (Fine 267→192), Messwerte gleich,
Tests grün; Aufwand S. Doku-Fehler: Fine kostet 133 ms mehr, nicht 85. B0 bei 50 %: Standard 6,8 dB
Wackeln, Fine 0,5 dB. Hörtest: wav/ (b0-synth, b0-nsynth, e1-idmt, plucks-synth, kicks, line-idmt), ~10 min.

## Recherche (Quellen und Ausgangslage)

Nine points, ranked by benefit for the player. Lines refer to `main` as of 29.10. (may have shifted slightly).

### 1. Report time: YIN window tied to the lowest note of the range, not the played note (M)
- `tracker.rs:441` sets `window_s = 1.0 / config.min_hz`; `Yin::needed()` = window + `tau_max`, i.e. two periods of `min_hz` (`yin.rs:79-82`).
- `min_hz` is two semitones below the lowest string (`instrument.rs:157-161`): 4-string D1 36.7 Hz → 54.6 ms; 5-string A0 27.5 Hz → 72.8 ms.
- Plus attack skip 12 ms (`tracker.rs:147`) and a second agreeing estimate 5 ms later → about 72 / 90 ms; measured 81 / 100 ms (target LH-NF-15: 70 ms).
- The second estimate overlaps the first by ~90 %, little independent information.
- Research: Fonseca & Tavares (SBCM 2019, RITA 2020): f0 at the finger pluck of the electric bass with the Absolute Difference Function, latency 1.1 fundamental periods (~27 ms on low E), half YIN's error rate at equal latency. Derrien (DAFx-14, ESPRIT) near one period.
- Proposal: YIN from the attack with a growing window (tau_max limited by available samples); report early when a clear dip lies well below tau_max; or a ~1-period method. Risk: octave errors upward (sub-octave invisible in short window); remedy: expected note (2) and revisions both ways (today only upward, `Steps`).
- Gain: A1 ~36 ms of audio instead of 55, notes from D2 under 30 ms.

### 2. Tracker does not know the expected note (M)
- `NoteTracker` only gets instrument and input (`TrackerConfig::for_instrument`, `tracker.rs:121`). The session knows note and time from the tab but uses them only when comparing. Detection decides blind with heuristics (`THUMP_*`, `OCTAVE_TOLERANCE` in `yin.rs:13-35`).
- Research: score-informed assessment (Benetos, Klapuri & Dixon EUSIPCO 2012; Wang, Ewert & Dixon TASLP 2017; Songs2See).
- Proposal: session passes the expected notes of the next window down (neighbour layer only). Detection first checks "is it the expected note?" (CMNDF at its period and at the octave below); free estimate keeps running and has the last word, so an invented tone never becomes a hit.
- Gain: earlier, surer report (basis for 1); fewer octave/semitone errors in the weak register (C3–B3 68.2 % right, C2–B2 13 semitones off; ROADMAP A5).

### 3. Device timestamps dropped; timeline starts at the first callback (M)
- Input and output ignore `InputCallbackInfo`/`OutputCallbackInfo` (`drivers/src/audio.rs:662`, `:815`). (#186 now passes the capture stamp for input gaps.)
- Time = `clock.now_s()` at first block, then counting samples (`input.rs:437-440`, `player.rs:262-264`), "calibration absorbs the rest" — only if the error is the same at every start; it varies with thread wake-up and quantum (PipeWire default 1024 frames = 21 ms). Another program changing the quantum invalidates calibration.
- Drift between input and output clock not corrected; only the monitor buffer is capped (`player.rs:35`, `:480`). Matters for separate devices (laptop jack in, USB/HDMI out).
- Practice: cpal 0.18.2 PipeWire host gives capture/playback as `now ∓ pw_time.delay` on CLOCK_MONOTONIC (`cpal-0.18.2/src/host/pipewire/stream.rs:329-348`), same clock as `Instant`. Map sample counters to system time with a DLL (Adriaensen, "Using a DLL to filter time"; JACK, PipeWire).
- Gain: same timeline at every start, device latency known without calibration, no drift with separate devices; "on the point ±5 ms" (LH-BEW-08) measurable.

### 4. "Play on the click" calibration folds the player's lean into latency (S–M)
- `session/src/calibration.rs:80-106` takes the median of attack minus click as system latency. Loopback only in `kontra-cli check` (`session/src/check.rs`).
- A player habitually 15 ms early gets 15 ms too little latency and then counts as on time — what LH-BEW-08 and LH-EIN-14 want to show. `latency.max(0.0)` (line 105) hides a lean larger than the latency.
- Research: negative mean asynchrony, typically tens of ms, smaller for musicians but not zero (Repp 2005; Repp & Su 2013). DAWs measure latency by loopback (`jack_iodelay`) or take it from the driver.
- Proposal: latency from device (3) or loopback in the app; "on the click" only without cable, result shown separately as "your lean".

### 5. Fixed YIN threshold and hard decisions instead of pYIN (M)
- Fixed threshold 0.15 and "first dip" (`yin.rs:13`, `:230-251`), then hand-picked octave and thump rules; confidence limit 0.6 (`tracker.rs:150`); after 150 ms timeout best estimate reported without agreement (`tracker.rs:718`); attack without pitch → dead note (`tracker.rs:770-775`).
- Research: pYIN (Mauch & Dixon, ICASSP 2014): prior over the threshold → several candidates with probabilities plus voicing probability; HMM/Viterbi picks the path; real time: fixed-lag Viterbi over the 5-ms estimates.
- Gain: octave/semitone decisions by probability instead of rules tuned on FiloBass/IDMT; voicing probability tells "no tone" from "dead note" (helps F3).
- Licence: Vamp reference is GPL → paper only. librosa `pyin` is ISC, may be read.

### 6. Technique thresholds chosen and measured on the same 17 lines (M)
- `strike.rs:12-13`: thresholds chosen on IDMT-SMT-Bass-Single-Track and measured on it; ROADMAP admits overfitting (one bass, one player). Pop 32/46, slides 3/20; one-fret slides not separable from hammer-on (IDMT has no hammer-on recordings).
- `testdata/korpus/QUELLEN.md:15`: IDMT-SMT-Bass (~4300 single notes) not loaded, converter missing.
- Research: Abeßer, Lukashevich & Schuller (ICASSP 2010) on this data: plucking styles (incl. slap thumb, slap pop, muted) 93.25 %, expression styles 95.61 %; features + feature selection (+27 points).
- Proposal: load single-note set as held-out test set (local only, CC BY-NC-ND); leave-one-file-out for thresholds; features like Abeßer (spectral centroid over time, decay per partial, inharmonicity).

### 7. No string detection: right note on wrong string goes unnoticed (L)
- Scoring knows only pitch; string assumed from tab. LH-BEW-12 wants intonation per string and position; LH-ANZ-05, LH-LRN-02 rely on positions. `intonation.rs` already measures fundamental apart from sharp partials.
- Research: Abeßer (CMMR 2012) string detection on bass F = 0.93 (4 classes), humans F = 0.26; Kehling et al. (DAFx-14) tab transcription with inharmonicity-robust partial tracking: string 82 %, plucking 93 %.
- With the expected note (2) it becomes a yes/no between 2–3 candidates.

### 8. Double stops: Kontra hears one tone (M)
- Tracker monophonic; M1 and LH-BEW-11 planned, M4 (NNLS) after 1.0.
- Research: Klapuri (ISMIR 2006) harmonic summation with iterative estimate-and-cancel. For LH-BEW-11 enough: "second fundamental with notable salience after removing the first?", limited to reachable string pairs (M3).
- Licence: Essentia `MultiPitchKlapuri` is AGPL → paper only.

### 9. Time-stretch with one resolution (L, least benefit)
- Already Laroche-Dolson phase locking and phase reset only in transient bins (`stretch/src/lib.rs`, `vocoder.rs`, `onset.rs`). Single window size; `Resolution::Fine` 8192 samples costs 133 ms more latency (Standard 133, Fine 267 ms; since R9, #200: 96 and 192 ms) and softer attacks (`lib.rs:41-48`).
- Research/practice: Průša & Holighaus "Phase Vocoder Done Right" (RTPGHI, 2022); Signalsmith Stretch (MIT, C++) as reading model (Rubber Band is GPL); multi-resolution (long window low bins, short high bins).
- Gain: B0 clean at 50 % without 8192 latency, hard attacks (LH-WIE-01).

### Sources
- Fonseca & Tavares: https://sol.sbc.org.br/index.php/sbcm/article/view/10433, https://seer.ufrgs.br/index.php/rita/article/view/RITA_VOL27_NR4_79
- Derrien DAFx-14: https://www.dafx14.fau.de/papers/dafx14_olivier_derrien_a_very_low_latency_pitch_.pdf
- Benetos, Klapuri & Dixon 2012: https://webspace.eecs.qmul.ac.uk/s.e.dixon/pub/2012/Benetos-Klapuri-Dixon-EUSIPCO2012a.pdf
- Wang, Ewert & Dixon 2017: https://webspace.eecs.qmul.ac.uk/s.e.dixon/pub/2017/WangEwertDixon-TASLP2017.pdf
- Adriaensen DLL: http://www.kokkinizita.net/papers/usingdll.pdf
- Repp & Su 2013: https://link.springer.com/article/10.3758/s13423-012-0371-2
- pYIN: http://matthiasmauch.de/_pdf/mauch_pyin_2014.pdf
- Abeßer 2010: https://publica.fraunhofer.de/entities/publication/34571752-4e2c-46fe-91de-f868ddf9e374
- IDMT-SMT-Bass: https://zenodo.org/record/7188892
- Abeßer string detection 2012: https://publica.fraunhofer.de/handle/publica/378743
- Kehling DAFx-14: https://www.dafx14.fau.de/papers/dafx14_christian_kehling_automatic_tablature_trans.pdf
- Klapuri ISMIR 2006: https://archives.ismir.net/ismir2006/paper/000125.pdf
- Průša & Holighaus: https://arxiv.org/abs/2202.07382
- Signalsmith Stretch: https://signalsmith-audio.co.uk/code/stretch
