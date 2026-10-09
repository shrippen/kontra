# Demo material

Own, MIT-licensed material for tests, QA journeys and screenshots.

- `etude-1.mid`: eight bars of eighth notes in E minor at 100 BPM (bass track "Bass").
- `clicks.mid`: eight open A strings on the beat at 100 BPM, rendered as a calibration take (`--lead-in 1.0 --offset-ms 40`).

Performances for `--input-file` are rendered, not recorded: `kontra-cli render etude-1.mid take.wav --lead-in 2.4` (one 4/4 count-in bar at 100 BPM is 2.4 s).
