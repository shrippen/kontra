//! The one place that writes `library.json` and `settings.json`.

use crate::{store, Core};

/// File names in the data directory.
pub(crate) const LIBRARY_FILE: &str = "library.json";
pub(crate) const SETTINGS_FILE: &str = "settings.json";

impl Core {
    pub(crate) fn save_library(&self) {
        let _ = store::save(&self.data_dir.join(LIBRARY_FILE), &self.library);
    }

    pub(crate) fn save_settings(&self) {
        let _ = store::save(&self.data_dir.join(SETTINGS_FILE), &self.settings);
    }

    /// What `new` changed while loading: pieces moved (the next start must not find the old
    /// paths in the file) and plugins switched off after a crash.
    pub(crate) fn save_after_start(&self, moved_pieces: bool) {
        if moved_pieces {
            self.save_library();
        }
        if !self.plugins.crashed.is_empty() {
            self.save_settings();
        }
    }
}
