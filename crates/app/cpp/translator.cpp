#include "translator.h"

#include <QCoreApplication>
#include <QStandardPaths>
#include <QTranslator>

bool install_kirigami_translator(rust::Str language)
{
    const auto lang = QString::fromUtf8(language.data(), static_cast<qsizetype>(language.size()));
    const auto subdir = QStringLiteral("locale/%1/LC_MESSAGES").arg(lang);
    const auto dirs = QStandardPaths::locateAll(QStandardPaths::GenericDataLocation, subdir,
                                                QStandardPaths::LocateDirectory);
    for (const auto &dir : dirs) {
        auto *translator = new QTranslator(QCoreApplication::instance());
        if (translator->load(QStringLiteral("libkirigami6_qt"), dir)) {
            QCoreApplication::installTranslator(translator);
            return true;
        }
        delete translator;
    }
    return false;
}
