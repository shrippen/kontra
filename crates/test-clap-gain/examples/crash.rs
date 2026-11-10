//! A plugin file that aborts the process when it is loaded.

use clack_plugin::entry::{Entry, EntryFactories, EntryLoadError};
use std::ffi::CStr;

struct Crash;

impl Entry for Crash {
    fn new(_bundle_path: Option<&CStr>) -> Result<Self, EntryLoadError> {
        std::process::abort()
    }

    fn declare_factories<'a>(&'a self, _builder: &mut EntryFactories<'a>) {}
}

clack_plugin::clack_export_entry!(Crash);
