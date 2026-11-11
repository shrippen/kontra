//! A plugin file that never returns from loading.

use clack_plugin::entry::{Entry, EntryFactories, EntryLoadError};
use std::ffi::CStr;
use std::time::Duration;

struct Hang;

impl Entry for Hang {
    fn new(_bundle_path: Option<&CStr>) -> Result<Self, EntryLoadError> {
        loop {
            std::thread::sleep(Duration::from_secs(1));
        }
    }

    fn declare_factories<'a>(&'a self, _builder: &mut EntryFactories<'a>) {}
}

clack_plugin::clack_export_entry!(Hang);
