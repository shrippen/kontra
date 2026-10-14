// Qt translator for Kirigami's own texts (round 12): KDE apps get it through KI18n, Kontra not.
#pragma once
#include "rust/cxx.h"

// Installs libkirigami6_qt for `language` ("de", "en") from the XDG data folders; false if none.
bool install_kirigami_translator(rust::Str language);
