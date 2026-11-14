//! Guards the bridge against stale settings in the UI: every property that `new` fills from the
//! settings must also be set again in `sync_settings` (loading a backup with "replace" changes
//! them all). Reads the bridge's source, because the bridge itself needs a running Qt.

#[cfg(test)]
mod tests {
    const BRIDGE: &str = include_str!("bridge.rs");
    /// `sync_settings` lives with the other settings functions.
    const SETTINGS: &str = include_str!("bridge/settings.rs");

    /// Fields filled from the core that `sync_settings` refreshes through a helper, not a setter.
    /// `staff_json` follows the loaded piece, not the settings.
    const VIA_HELPER: [(&str, &str); 7] = [
        ("tuning", "refresh_tuning("),
        ("tuning_json", "refresh_tuning("),
        ("profile_tuning_json", "refresh_tuning("),
        ("note_choices_json", "refresh_tuning("),
        ("setlists_json", "refresh_setlists("),
        ("effects_json", "refresh_effects("),
        ("staff_json", ""),
    ];

    /// Text of `source` from `start` up to the next `end`.
    fn block<'a>(source: &'a str, start: &str, end: &str) -> &'a str {
        let from = source.find(start).unwrap_or(0);
        let to = source[from..].find(end).map_or(source.len(), |i| from + i);
        &source[from..to]
    }

    #[test]
    fn sync_settings_sets_every_settings_property() {
        // In `new`: `left_handed: s.left_handed,` (from the settings `s`) or
        // `tuning_json: q(&core.tuning_json()),` (from the core).
        let init = block(BRIDGE, "fn default() -> Self", "\n    }\n}");
        let synced = block(SETTINGS, "fn sync_settings", "\n    }\n");
        let missing: Vec<&str> = init
            .lines()
            .filter_map(|line| {
                let (field, value) = line.trim().split_once(": ")?;
                let from_settings = value.starts_with("s.")
                    || value.contains("(&s.")
                    || value.contains("(s.")
                    || value.contains("core.");
                from_settings.then_some(field)
            })
            .filter(|field| {
                let helper = VIA_HELPER.iter().find(|(name, _)| name == field);
                match helper {
                    Some((_, call)) => !synced.contains(call),
                    None => !synced.contains(&format!("set_{field}(")),
                }
            })
            .collect();
        assert!(
            missing.is_empty(),
            "not refreshed in sync_settings: {missing:?}"
        );
    }
}
