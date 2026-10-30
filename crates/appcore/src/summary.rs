//! Run summary (LH-BEW-01): note list, observations, pocket, intonation and probe texts.

use crate::{
    instrument_from, key_at, same_range, state_of, technique_key, technique_result_key, Core,
    PULSE_SPREAD_MS, WORKSHOP_MODE,
};
use kontra_model::{Judging, Technique};
use kontra_session::{
    Intonation, Leaning, NoteResult, Observation, Outcome, OutsideNote, Pocket, PocketVerdict,
    Side, Summary, TechniqueResult,
};
use serde_json::json;
use std::path::Path;

/// Tones of a double stop (R8): only its missed tone says "nur ein Ton des Griffs gehört".
const DOUBLE_STOP_TONES: usize = 2;
/// How many notes the evaluation lists as hardest.
const HARDEST_NOTES: usize = 5;
/// A probe this many points below the workshop or closer still counts as holding (I1).
const PROBE_SAME_POINTS: f64 = 5.0;

impl Core {
    /// Every judged note of the run (LH-BEW-01), the last attempt per pass and note, in order:
    /// `{index, pass, bar, state, offsetMs, cents, severity, technique, techniqueResult, text}`;
    /// `text` reads like "Takt 3 · G1 · 42 ms zu spät · +12 Cent · Slap: andere Technik"; a
    /// missed note in `outside` tells where it fell beside the window.
    pub(crate) fn note_details(
        &self,
        results: &[NoteResult],
        techniques: &[TechniqueResult],
        outside: &[OutsideNote],
    ) -> Vec<serde_json::Value> {
        let Some(a) = self.arrangement() else {
            return Vec::new();
        };
        let mut last = std::collections::BTreeMap::new();
        for r in results {
            if let Some(note) = r.note {
                last.insert((r.pass, note), *r);
            }
        }
        let mut technique_of = std::collections::BTreeMap::new();
        for t in techniques {
            technique_of.insert((t.pass, t.sign), *t);
        }
        // Double stop tones sounded per pass and start: a missed tone beside one is "nur ein Ton"
        // (R8). Not for larger chords: MIDI judges each tone alone, two of three are not "one".
        let double_stop = |start: u64| {
            a.notes.iter().filter(|n| n.start_ticks == start).count() == DOUBLE_STOP_TONES
        };
        let sounded: Vec<(u32, u64)> = last
            .iter()
            .filter(|(_, r)| r.outcome != Outcome::Missed)
            .filter_map(|((pass, index), _)| Some((*pass, a.notes.get(*index)?.start_ticks)))
            .filter(|(_, start)| double_stop(*start))
            .collect();
        last.into_iter()
            .filter_map(|((pass, index), r)| {
                let note = a.notes.get(index)?;
                let state = state_of(r.outcome, r.offset_ms.unwrap_or(0.0));
                let bar = r
                    .bar
                    .and_then(|b| a.bars.get(b))
                    .map_or(0, |b| b.source_index + 1);
                let key = key_at(a, note.start_ticks);
                let mut parts = vec![
                    self.tr("summary.note.bar", &[("bar", bar.to_string())]),
                    self.pitch_label(note.pitch, key),
                ];
                // Early and late say it with the milliseconds below.
                if !matches!(state, "early" | "late") {
                    parts.push(self.tr(&format!("result.{state}"), &[]));
                }
                match r.offset_ms {
                    Some(ms) if ms <= -1.0 => {
                        parts.push(self.tr("summary.note.early", &[("ms", format!("{:.0}", -ms))]))
                    }
                    Some(ms) if ms >= 1.0 => {
                        parts.push(self.tr("summary.note.late", &[("ms", format!("{ms:.0}"))]))
                    }
                    Some(_) => parts.push(self.tr("summary.note.ontime", &[])),
                    None => {}
                }
                if r.outcome == Outcome::Missed && sounded.contains(&(pass, note.start_ticks)) {
                    parts.push(self.tr("summary.note.one.tone", &[]));
                }
                // A missed note played beside the window: where it fell (M6).
                let beside = outside
                    .iter()
                    .find(|o| o.pass == pass && o.note == index)
                    .map(|o| o.offset_ms);
                match beside {
                    Some(ms) if ms < 0.0 => parts.push(self.tr(
                        "summary.note.outside.early",
                        &[("ms", format!("{:.0}", -ms))],
                    )),
                    Some(ms) => parts
                        .push(self.tr("summary.note.outside.late", &[("ms", format!("{ms:.0}"))])),
                    None => {}
                }
                // A wrong note names what was played; cents only for the right note's intonation.
                match (r.outcome, r.cents) {
                    (Outcome::WrongPitch { played_midi }, _) => {
                        let played =
                            kontra_model::Pitch(played_midi.round().clamp(0.0, 127.0) as u8);
                        let played = self.pitch_label(played, key);
                        parts.push(self.tr("summary.note.played", &[("played", played)]));
                    }
                    (_, Some(c)) => {
                        parts.push(self.tr("summary.note.cents", &[("cents", format!("{c:+.0}"))]));
                    }
                    _ => {}
                }
                let technique = technique_of.get(&(pass, index));
                let mark = technique.and_then(|t| t.mark());
                if let (Some(t), Some(_)) = (technique, mark) {
                    parts.push(self.tr(
                        "summary.note.technique",
                        &[
                            ("technique", self.technique_name(t.expected)),
                            (
                                "result",
                                self.tr(
                                    &format!("technique.result.{}", technique_result_key(mark)),
                                    &[],
                                ),
                            ),
                        ],
                    ));
                }
                Some(json!({
                    "index": index,
                    "pass": pass,
                    "bar": bar,
                    "state": state,
                    "offsetMs": r.offset_ms,
                    "cents": r.cents,
                    "severity": severity(state),
                    "technique": technique.map_or("", |t| technique_key(t.expected)),
                    "techniqueResult": technique_result_key(mark),
                    "text": parts.join(" · "),
                }))
            })
            .collect()
    }

    /// Notes on another string than the tab's (R7); a profile that cannot tell says so, also
    /// for some notes only (a five-string's low B, review #207). One line each.
    fn strings_text(&self, strings: kontra_session::StringTally) -> String {
        let mut parts = Vec::new();
        if strings.other > 0 {
            parts.push(self.tr(
                "summary.strings.other",
                &[
                    ("other", strings.other.to_string()),
                    ("told", strings.told.to_string()),
                ],
            ));
        }
        if strings.unsupported > 0 && strings.told == 0 {
            parts.push(self.tr("summary.strings.unsupported", &[]));
        }
        if strings.unsupported > 0 && strings.told > 0 {
            parts.push(self.tr(
                "summary.strings.unsupported.some",
                &[("count", strings.unsupported.to_string())],
            ));
        }
        parts.join("\n")
    }

    /// What this input did not judge: techniques, hints (slap and pop over a cable) and double
    /// stops at a microphone (Runde 17); one line each.
    fn untold_text(&self, techniques: &[String], hints: &[String], double_stops: u32) -> String {
        let mut parts = Vec::new();
        if !techniques.is_empty() {
            parts.push(self.tr(
                "summary.techniques.untold",
                &[("techniques", techniques.join(", "))],
            ));
        }
        if !hints.is_empty() {
            parts.push(self.tr(
                "summary.techniques.hint",
                &[("techniques", hints.join(", "))],
            ));
        }
        if double_stops > 0 {
            parts.push(self.tr("summary.doublestops.untold", &[]));
        }
        parts.join("\n")
    }

    /// Summary with ready-made texts for the evaluation sheet, and every note's result.
    pub fn summary_json(
        &self,
        s: &Summary,
        results: &[NoteResult],
        techniques: &[TechniqueResult],
    ) -> String {
        let notes = self.note_details(results, techniques, &s.outside_notes);
        // Techniques apart from the hit rate (round 9): "Slide: 8 von 10".
        let technique_list: Vec<_> = s
            .techniques
            .iter()
            .map(|t| {
                json!({
                    "technique": technique_key(t.technique),
                    "played": t.played,
                    "judged": t.judged,
                    "text": self.tr("summary.technique", &[
                        ("technique", self.technique_name(t.technique)),
                        ("played", t.played.to_string()),
                        ("judged", t.judged.to_string()),
                    ]),
                })
            })
            .collect();
        let names = |list: &[Technique]| -> Vec<String> {
            list.iter().map(|t| self.technique_name(*t)).collect()
        };
        let untold = names(&s.untold_techniques);
        let hints = names(&s.hint_techniques);
        let hardest_notes = hardest_notes(&notes);
        let arrangement = self.arrangement();
        let judging = arrangement.map_or(Judging::PitchAndTime, |a| a.judging);
        // Most notes missed the window but were played beside it: their offset is the tendency
        // (QA 27.10., M2: strict window, all 40 ms early read "0 % hit" without a word on time).
        let outside = s.outside_offset_ms.filter(|_| s.missed > s.hits);
        // Intonation only where it is scored: a fretless or double bass, or off-pitch notes.
        let intonation = s
            .intonation
            .filter(|_| s.off_pitch > 0 || self.scored_intonation());
        let tendency = match (outside, s.mean_offset_ms) {
            (Some(m), _) if m < 0.0 => {
                self.tr("summary.outside.early", &[("ms", format!("{:.0}", -m))])
            }
            (Some(m), _) => self.tr("summary.outside.late", &[("ms", format!("{m:.0}"))]),
            (None, Some(m)) if m < -5.0 => {
                self.tr("summary.early", &[("ms", format!("{:.0}", -m))])
            }
            (None, Some(m)) if m > 5.0 => self.tr("summary.late", &[("ms", format!("{m:.0}"))]),
            (None, Some(_)) => self.tr("summary.ontime", &[]),
            (None, None) => String::new(),
        };
        let hardest: Vec<_> = s
            .hardest_bars
            .iter()
            .map(|b| {
                let number = arrangement
                    .and_then(|a| a.bars.get(*b))
                    .map_or(b + 1, |bar| bar.source_index as usize + 1);
                json!({ "index": b, "number": number })
            })
            .collect();
        // A bundled exercise that now sits: the next one is one click away.
        let next = self
            .loaded_path()
            .filter(|p| p.parent().and_then(Path::parent) == Some(Self::exercises_dir().as_path()))
            .filter(|_| Self::run_sits(judging, s.hit_rate, s.spread_ms))
            .and_then(|p| self.next_exercise(Some(p)));
        json!({
            "next": next,
            "probe": self.probe_json(s),
            "hitRate": s.hit_rate,
            "hitRateText": self.tr("summary.hitrate", &[("percent", format!("{:.0}", s.hit_rate * 100.0))]),
            "countsText": self.tr("summary.counts", &[
                ("hits", s.hits.to_string()),
                ("judged", s.judged.to_string()),
                ("wrong", s.wrong.to_string()),
                ("missed", s.missed.to_string()),
            ]),
            "observation": self.observation_json(s),
            "recalibrateText": self.recalibrate_text(s),
            "tendencyText": tendency,
            // The few hits beside the missed notes tell no steadiness.
            "spreadText": s
                .spread_ms
                .filter(|_| outside.is_none())
                .map_or(String::new(), |v| self.spread_text(v, judging)),
            "pocketText": self.pocket_text(s),
            "intonationText": intonation.map_or(String::new(), |i| self.tr("summary.intonation", &[
                ("offpitch", s.off_pitch.to_string()),
                ("text", self.intonation_text(i)),
            ])),
            "intonationParts": intonation.map_or(Vec::new(), |_| self.intonation_parts(s)),
            // Feedback beside the score (R7): notes on another string than the tab's.
            "stringsText": self.strings_text(s.strings),
            "meanOffsetMs": s.mean_offset_ms.unwrap_or(0.0),
            "counts": s.histogram.counts,
            "firstBinMs": s.histogram.first_bin_ms,
            "binMs": s.histogram.bin_ms,
            "hardestBars": hardest,
            "notes": notes,
            "hardestNotes": hardest_notes,
            "techniques": technique_list,
            "untoldText": self.untold_text(&untold, &hints, s.untold_double_stops),
        })
        .to_string()
    }

    /// The one observation of the run (LH-BEW-09), or null below the minimum of notes (or of
    /// hits, for timing):
    /// `{ solid: false, text: "Am meisten bringt jetzt: Slide, 2 von 6. …" }`.
    fn observation_json(&self, s: &Summary) -> serde_json::Value {
        let pocket = Pocket::from_key(&self.settings.pocket).unwrap_or_default();
        let Some(observation) = s.observation(pocket) else {
            return serde_json::Value::Null;
        };
        // "Solid" would contradict the latency hint next to it (review #195): that hint alone.
        if observation == Observation::Solid && self.latency_hint(s, pocket).is_some() {
            return serde_json::Value::Null;
        }
        let percent = |share: f64| format!("{:.0}", share * 100.0);
        let text = match observation {
            Observation::Technique {
                technique,
                played,
                judged,
            } => self.tr(
                "summary.observe.technique",
                &[
                    ("technique", self.technique_name(technique)),
                    ("played", played.to_string()),
                    ("judged", judged.to_string()),
                ],
            ),
            Observation::Beside { offset_ms } if offset_ms < 0.0 => self.tr(
                "summary.observe.beside.early",
                &[("ms", format!("{:.0}", -offset_ms))],
            ),
            Observation::Beside { offset_ms } => self.tr(
                "summary.observe.beside.late",
                &[("ms", format!("{offset_ms:.0}"))],
            ),
            Observation::Missed { notes, judged } => self.tr(
                "summary.observe.missed",
                &[("notes", notes.to_string()), ("judged", judged.to_string())],
            ),
            Observation::Lean {
                side: Side::Early,
                share,
            } => self.tr("summary.observe.early", &[("percent", percent(share))]),
            Observation::Lean {
                side: Side::Late,
                share,
            } => self.tr("summary.observe.late", &[("percent", percent(share))]),
            Observation::Aim {
                side: Side::Early,
                off_ms,
            } => self.tr(
                "summary.observe.aim.early",
                &[("ms", format!("{off_ms:.0}"))],
            ),
            Observation::Aim {
                side: Side::Late,
                off_ms,
            } => self.tr(
                "summary.observe.aim.late",
                &[("ms", format!("{off_ms:.0}"))],
            ),
            Observation::Solid => self.tr("summary.observe.solid", &[]),
        };
        json!({ "solid": observation == Observation::Solid, "text": text })
    }

    /// The latency hint (LH-EIN-14), else "". After an observation of the same lean it follows
    /// on ("Liegen die Treffer so beständig hinter dem Schlag (im Mittel 25 ms), kann auch …"),
    /// otherwise it stands alone ("Die Treffer liegen beständig hinter dem Schlag …").
    fn recalibrate_text(&self, s: &Summary) -> String {
        let pocket = Pocket::from_key(&self.settings.pocket).unwrap_or_default();
        let Some(side) = self.latency_hint(s, pocket) else {
            return String::new();
        };
        let follows = matches!(
            s.observation(pocket),
            Some(Observation::Lean { side: lean, .. } | Observation::Aim { side: lean, .. })
                if lean == side
        );
        let key = match (side, follows) {
            (Side::Early, true) => "summary.recalibrate.early",
            (Side::Late, true) => "summary.recalibrate.late",
            (Side::Early, false) => "summary.recalibrate.early.alone",
            (Side::Late, false) => "summary.recalibrate.late.alone",
        };
        let ms = format!("{:.0}", s.mean_offset_ms.unwrap_or_default().abs());
        self.tr(key, &[("ms", ms)])
    }

    /// A blind probe next to the workshop before it (DIDAKTIK I1), or null:
    /// `{ with: "88 % · ±11 ms" | "", without: "71 % · ±19 ms", text: "17 Punkte weniger …" }`.
    fn probe_json(&self, s: &Summary) -> serde_json::Value {
        let (Some(_), Some(range)) = (self.probe, self.loop_range()) else {
            return serde_json::Value::Null;
        };
        let row = |rate: f64, spread: Option<f64>| {
            let percent = format!("{:.0}", rate * 100.0);
            match spread {
                Some(ms) => self.tr(
                    "probe.row.spread",
                    &[("percent", percent), ("ms", format!("{ms:.0}"))],
                ),
                None => self.tr("probe.row", &[("percent", percent)]),
            }
        };
        let without = row(s.hit_rate, s.spread_ms);
        let runs = self.loaded_runs();
        let practised = runs
            .iter()
            .rev()
            .find(|r| r.mode == WORKSHOP_MODE && same_range(r.section, range));
        let Some(practised) = practised else {
            return json!({ "with": "", "without": without, "text": self.tr("probe.first", &[]) });
        };
        let points = (practised.hit_rate - s.hit_rate) * 100.0;
        let text = if points > PROBE_SAME_POINTS {
            self.tr("probe.less", &[("points", format!("{points:.0}"))])
        } else {
            self.tr("probe.holds", &[])
        };
        json!({
            "with": row(practised.hit_rate, practised.spread_ms),
            "without": without,
            "text": text,
        })
    }

    /// A technique's name in the UI's language ("Hammer-on").
    fn technique_name(&self, technique: Technique) -> String {
        self.tr(&format!("technique.{}", technique_key(technique)), &[])
    }

    /// Intonation is part of the score for fretless and double basses (pitch limit below 50 ct).
    fn scored_intonation(&self) -> bool {
        let kind = self.arrangement().map_or_else(
            || instrument_from(&self.settings).kind,
            |a| a.instrument.kind,
        );
        self.score_settings(kind).pitch_cents < 50.0
    }

    /// Direction and spread, e.g. "im Schnitt 9 Cent zu tief, Streuung ±6"; without a direction
    /// only the spread (LH-BEW-12).
    fn intonation_text(&self, i: Intonation) -> String {
        let key = match i.leaning() {
            Some(Leaning::Flat) => "summary.intonation.flat",
            Some(Leaning::Sharp) => "summary.intonation.sharp",
            None => "summary.intonation.centred",
        };
        self.tr(
            key,
            &[
                ("cents", format!("{:.0}", i.mean_cent.abs())),
                ("spread", format!("{:.0}", i.spread_cent)),
            ],
        )
    }

    /// Per string and position, e.g. "E-Saite, Bund 5 bis 9: …" (LH-BEW-12); none for a single
    /// part, it would repeat the line above.
    fn intonation_parts(&self, s: &Summary) -> Vec<String> {
        if s.intonation_parts.len() < 2 {
            return Vec::new();
        }

        let strings = self
            .arrangement()
            .map(|a| a.instrument.tuning.strings.clone())
            .unwrap_or_default();
        s.intonation_parts
            .iter()
            .map(|p| {
                let string = strings
                    .get(usize::from(p.string))
                    .map_or_else(|| (p.string + 1).to_string(), |s| s.name().to_string());
                self.tr(
                    "summary.intonation.part",
                    &[
                        ("string", string),
                        ("from", p.first_fret.to_string()),
                        ("to", p.last_fret.to_string()),
                        ("text", self.intonation_text(p.intonation)),
                    ],
                )
            })
            .collect()
    }

    /// The spread, e.g. "Streuung ±18 ms"; judged by time only with the aim for the frets
    /// (DIDAKTIK I4), e.g. "Streuung ±31 ms · Bünde kommen unter ±25 ms dazu".
    fn spread_text(&self, spread_ms: f64, judging: Judging) -> String {
        let spread = self.tr("summary.spread", &[("ms", format!("{spread_ms:.0}"))]);
        match judging {
            Judging::TimeOnly => self.tr(
                "summary.spread.pulse",
                &[("spread", spread), ("ms", format!("{PULSE_SPREAD_MS:.0}"))],
            ),
            Judging::PitchAndTime => spread,
        }
    }

    /// Whether the run held the chosen aim against the beat (LH-BEW-08), e.g. "Ziel laid-back
    /// (10 bis 30 ms dahinter): verfehlt, 6 ms vor dem Zielbereich"; empty without an aim or
    /// timed notes.
    fn pocket_text(&self, s: &Summary) -> String {
        let pocket = Pocket::from_key(&self.settings.pocket).unwrap_or_default();
        // Whole ms, as the text shows them: +9.6 is laid-back, not "missed by 0 ms".
        let Some(verdict) = s.mean_offset_ms.and_then(|m| pocket.judge(m.round())) else {
            return String::new();
        };
        let Some((a, b)) = pocket.band_ms() else {
            return String::new();
        };
        let (low, high) = (a.abs().min(b.abs()), a.abs().max(b.abs()));
        let aim = self.tr(
            &format!("pocket.{}.aim", pocket.key()),
            &[("low", format!("{low:.0}")), ("high", format!("{high:.0}"))],
        );
        let (key, ms) = match verdict {
            PocketVerdict::Held => ("summary.pocket.held", 0.0),
            PocketVerdict::Early(ms) => ("summary.pocket.early", ms),
            PocketVerdict::Late(ms) => ("summary.pocket.late", ms),
        };
        self.tr(key, &[("aim", aim), ("ms", format!("{ms:.0}"))])
    }
}

/// Weight of a state for "hardest notes": a missed or wrong note before an off-pitch one,
/// before one off in time; a clean hit is 0.
fn severity(state: &str) -> u8 {
    match state {
        "missed" | "wrong" => 3,
        "offpitch" => 2,
        "early" | "late" => 1,
        _ => 0,
    }
}

/// The hardest notes of `notes_details`: worst state first, then the largest timing offset.
pub(crate) fn hardest_notes(notes: &[serde_json::Value]) -> Vec<serde_json::Value> {
    let key = |n: &serde_json::Value| {
        let severity = n["severity"].as_u64().unwrap_or(0);
        let offset = n["offsetMs"].as_f64().map_or(0.0, f64::abs);
        (severity, offset)
    };
    let mut hard: Vec<&serde_json::Value> = notes.iter().filter(|n| key(n).0 > 0).collect();
    hard.sort_by(|a, b| {
        let (a, b) = (key(a), key(b));
        b.0.cmp(&a.0).then(b.1.total_cmp(&a.1))
    });
    hard.into_iter().take(HARDEST_NOTES).cloned().collect()
}
