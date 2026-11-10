// Environment variables as Qt reads them (LH-NF-17).
#pragma once
#include "rust/cxx.h"

// Sets `name` for Qt. On Windows Qt reads the C runtime's copy of the environment, which
// Rust's `std::env::set_var` does not change; `qputenv` reaches it on every system.
void set_qt_env(rust::Str name, rust::Str value);
