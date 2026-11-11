//! Mono gain plugin file.

use clack_plugin::prelude::*;
use kontra_clap_gain::Gain;

clack_plugin::clack_export_entry!(SinglePluginEntry<Gain<1>>);
