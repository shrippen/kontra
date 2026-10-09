//! Progress report of one piece as a self-contained HTML file for teachers (LH-FOR-04):
//! key figures, hit rate and tempo per run as a chart, every run in a table.
//!
//! Styled with Kante's web stylesheet, embedded unchanged (`report/shrippen.css`, refreshed by
//! `scripts/sync-kante.sh`), so the file opens anywhere without a network.

use crate::Core;
use kontra_session::{Run, SongProgress};
use std::fmt::Write;
use std::path::Path;

const KANTE_CSS: &str = include_str!("../report/shrippen.css");
/// Chart canvas in SVG units: runs left to right, 0–100 % bottom to top.
const CHART_W: f64 = 600.0;
const CHART_H: f64 = 200.0;
const CHART_PAD: f64 = 24.0;
/// The tempo the progress view calls "sitzt" (LH-FOR-01): best speed at ≥ 90 % hits.
const SOLID_HIT_RATE: f64 = 0.9;
const SECONDS_PER_DAY: u64 = 86_400;

impl Core {
    /// Writes the report of the piece with progress id `song_id` to `path`.
    pub fn export_report(&self, song_id: &str, path: &Path) -> Result<String, String> {
        let progress = self
            .progress_store()
            .load_id(song_id)
            .ok_or_else(|| self.tr("report.error.none", &[]))?;
        let html = self.report_html(&progress, crate::unix_now());
        std::fs::write(path, html)
            .map_err(|e| self.tr("data.error.write", &[("reason", e.to_string())]))?;
        Ok(self.tr("report.saved", &[("name", crate::file_name(path))]))
    }

    fn report_html(&self, p: &SongProgress, now_unix: u64) -> String {
        let percent = |v: f64| format!("{:.0} %", v * 100.0);
        let minutes = format!("{:.0}", p.practice_s() / 60.0);
        let facts = [
            (minutes, self.tr("report.minutes", &[])),
            (p.runs.len().to_string(), self.tr("report.runs", &[])),
            (
                p.best_hit_rate().map_or_else(|| "–".into(), percent),
                self.tr("report.best", &[]),
            ),
            (
                p.best_speed(SOLID_HIT_RATE)
                    .map_or_else(|| "–".into(), percent),
                self.tr("report.solid", &[]),
            ),
        ];
        let mut html = String::new();
        let lang = self.catalog.language().code();
        let title = self.tr("report.title", &[("title", p.title.clone())]);
        let _ = write!(
            html,
            "<!doctype html>\n<html lang=\"{lang}\" data-theme=\"light\">\n<head>\n<meta charset=\"utf-8\">\n\
             <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n\
             <title>{}</title>\n<style>\n{KANTE_CSS}\n</style>\n</head>\n<body>\n<main>\n",
            escape(&title)
        );
        let _ = write!(
            html,
            "<section class=\"section\">\n<h1>{}</h1>\n<p>{}</p>\n</section>\n<div class=\"facts\">\n",
            escape(&title),
            escape(&self.tr(
                "report.subtitle",
                &[("artist", p.artist.clone()), ("date", date(now_unix))]
            )),
        );
        for (value, label) in &facts {
            let _ = writeln!(
                html,
                "  <div class=\"fact\"><b>{}</b><span>{}</span></div>",
                escape(value),
                escape(label)
            );
        }
        html.push_str("</div>\n");
        let _ = write!(
            html,
            "<section class=\"section\">\n<h2>{}</h2>\n<p>{}</p>\n{}</section>\n",
            escape(&self.tr("report.history", &[])),
            escape(&self.tr("report.legend", &[])),
            chart(&p.runs, &self.tr("report.history", &[])),
        );
        let _ = write!(
            html,
            "<section class=\"section\">\n<h2>{}</h2>\n{}</section>\n</main>\n</body>\n</html>\n",
            escape(&self.tr("report.table", &[])),
            self.run_table(&p.runs),
        );
        html
    }

    fn run_table(&self, runs: &[Run]) -> String {
        let heads = ["date", "mode", "tempo", "hits", "time", "bars"]
            .map(|k| {
                format!(
                    "<th>{}</th>",
                    escape(&self.tr(&format!("report.col.{k}"), &[]))
                )
            })
            .join("");
        let mut out = format!(
            "<div class=\"table-wrap\"><table class=\"table\">\n<thead><tr>{heads}</tr></thead>\n<tbody>\n"
        );
        for r in runs.iter().rev() {
            let mode = self.tr(&format!("mode.{}", r.mode), &[]);
            let bars = r
                .hardest_bars
                .iter()
                .map(u32::to_string)
                .collect::<Vec<_>>()
                .join(", ");
            let _ = writeln!(
                out,
                "<tr><td>{}</td><td>{}</td><td>{:.0} %</td><td>{:.0} %</td><td>{:.1} min</td><td>{}</td></tr>",
                date(r.finished_unix),
                escape(&mode),
                r.speed * 100.0,
                r.hit_rate * 100.0,
                r.practice_s / 60.0,
                escape(&bars),
            );
        }
        out.push_str("</tbody>\n</table></div>\n");
        out
    }
}

/// Hit rate (line with marks) and tempo (dashed line) per run, oldest left; grid at 0/50/100 %.
fn chart(runs: &[Run], label: &str) -> String {
    let inner_w = CHART_W - 2.0 * CHART_PAD;
    let inner_h = CHART_H - 2.0 * CHART_PAD;
    let x = |i: usize| {
        let steps = runs.len().saturating_sub(1).max(1) as f64;
        CHART_PAD + inner_w * i as f64 / steps
    };
    let y = |v: f64| CHART_PAD + inner_h * (1.0 - v.clamp(0.0, 1.5) / 1.5);
    let mut svg = format!(
        "<svg class=\"chart\" viewBox=\"0 0 {CHART_W} {CHART_H}\" role=\"img\" aria-label=\"{}\">\n",
        escape(label)
    );
    for level in [0.0, 0.5, 1.0, 1.5] {
        let _ = writeln!(
            svg,
            "<line class=\"grid\" x1=\"{CHART_PAD}\" x2=\"{}\" y1=\"{1:.1}\" y2=\"{1:.1}\"/>\
             <text class=\"axis\" x=\"{2}\" y=\"{1:.1}\">{3:.0} %</text>",
            CHART_W - CHART_PAD,
            y(level),
            CHART_PAD - 4.0,
            level * 100.0
        );
    }
    let _ = writeln!(
        svg,
        "<line class=\"goal\" x1=\"{CHART_PAD}\" x2=\"{}\" y1=\"{1:.1}\" y2=\"{1:.1}\"/>",
        CHART_W - CHART_PAD,
        y(SOLID_HIT_RATE)
    );
    let points = |value: fn(&Run) -> f64| {
        runs.iter()
            .enumerate()
            .map(|(i, r)| format!("{:.1},{:.1}", x(i), y(value(r))))
            .collect::<Vec<_>>()
            .join(" ")
    };
    let _ = writeln!(
        svg,
        "<polyline class=\"line s2 bar is-prev\" points=\"{}\"/>",
        points(|r| r.speed)
    );
    let _ = writeln!(
        svg,
        "<polyline class=\"line s1\" points=\"{}\"/>",
        points(|r| r.hit_rate)
    );
    for (i, r) in runs.iter().enumerate() {
        let _ = writeln!(
            svg,
            "<circle class=\"mark s1\" cx=\"{:.1}\" cy=\"{:.1}\" r=\"3\"/>",
            x(i),
            y(r.hit_rate)
        );
    }
    svg.push_str("</svg>\n");
    svg
}

/// `2026-10-09` for a Unix time (UTC; days from the epoch to a civil date).
fn date(unix: u64) -> String {
    let days = (unix / SECONDS_PER_DAY) as i64 + 719_468;
    let era = days.div_euclid(146_097);
    let doe = days.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!("{year:04}-{month:02}-{day:02}")
}

/// HTML text escaping for titles and names from score files.
fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dates_and_escaping() {
        assert_eq!(date(0), "1970-01-01");
        assert_eq!(date(1_791_504_000), "2026-10-09");
        assert_eq!(date(951_782_400), "2000-02-29");
        assert_eq!(escape("<b>R&B \"x\""), "&lt;b&gt;R&amp;B &quot;x&quot;");
    }
}
