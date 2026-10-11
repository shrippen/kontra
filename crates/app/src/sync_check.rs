//! Guards the bridge against stale settings in the UI: every property that `new` fills from the
//! settings must also be set again in `sync_settings` (loading a backup with "replace" changes
//! them all). Reads the bridge's source, because the bridge itself needs a running Qt.

#[cfg(test)]
mod tests {
    const BRIDGE: &str = include_str!("bridge.rs");

    /// Text of the function starting at `start` up to the next one at `end`.
    fn block<'a>(start: &str, end: &str) -> &'a str {
        let from = BRIDGE.find(start).unwrap_or(0);
        let to = BRIDGE[from..].find(end).map_or(BRIDGE.len(), |i| from + i);
        &BRIDGE[from..to]
    }

    #[test]
    fn sync_settings_sets_every_settings_property() {
        // In `new`: `left_handed: s.left_handed,` (struct field from the settings `s`).
        let init = block("fn default() -> Self", "\n    }\n}");
        let synced = block("fn sync_settings", "\n    fn ");
        let missing: Vec<&str> = init
            .lines()
            .filter_map(|line| {
                let (field, value) = line.trim().split_once(": ")?;
                let from_settings =
                    value.starts_with("s.") || value.contains("(&s.") || value.contains("(s.");
                from_settings.then_some(field)
            })
            .filter(|field| !synced.contains(&format!("set_{field}(")))
            .collect();
        assert!(
            missing.is_empty(),
            "not refreshed in sync_settings: {missing:?}"
        );
    }
}
