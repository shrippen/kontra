//! The reasons a corpus set fails `korpus check`.

/// Why a set failed, one indented line per missed requirement ("" if it met them all).
pub(crate) fn miss_lines(set: &str, misses: &[String]) -> String {
    misses.iter().map(|m| format!("    {set}: {m}\n")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn misses_print_one_line_each() {
        let misses = vec![
            "recall 80.0% < 90.0%".to_string(),
            "pitch 70.0% < 90.0%".to_string(),
        ];
        assert_eq!(
            miss_lines("clean", &misses),
            "    clean: recall 80.0% < 90.0%\n    clean: pitch 70.0% < 90.0%\n"
        );
        assert_eq!(miss_lines("clean", &[]), "");
    }
}
