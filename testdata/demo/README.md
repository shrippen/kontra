# Demo material

Own, MIT-licensed material for tests, QA journeys and screenshots.

- `etude-1.mid`: eight bars of eighth notes in E minor at 100 BPM (bass track "Bass").
- `clicks.mid`: eight open A strings on the beat at 100 BPM, rendered as a calibration take (`--lead-in 3.0 --offset-ms 40`, after the count-in).
- `songsterr-answer.json`: an invented answer of the Songsterr search (three songs, two with a bass track) for `--songsterr-answer`, so screenshots of J-B6 never use the network.
- `groove-techniken.musicxml`: two bars at 120 BPM with a hammer-on, a pull-off, a slide, two dead notes (x) and a ghost note ( ) (LH-BEW-06), the groove of J-C5; rendered with `--techniques`.

Performances for `--input-file` are rendered, not recorded: `kontra-cli render etude-1.mid take.wav --lead-in 2.4` (one 4/4 count-in bar at 100 BPM is 2.4 s).
