// Qt translator for Kirigami's own texts (round 12): KDE apps get it through KI18n, Kontra not.
#pragma once
#include "rust/cxx.h"

#include <QQmlApplicationEngine>

// Installs libkirigami6_qt for `language` ("de", "en") from the XDG data folders, replacing the
// one installed before (a language change at runtime). English puts the source texts in front of
// what Kirigami loaded for the system's language. False if the language has no translation.
bool install_kirigami_translator(rust::Str language);

// The engine whose texts a later language change translates again.
void translate_engine(QQmlApplicationEngine &engine);
