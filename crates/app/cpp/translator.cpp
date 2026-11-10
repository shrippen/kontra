#include "translator.h"

#include <QCoreApplication>
#include <QPointer>
#include <QStandardPaths>
#include <QTranslator>

namespace {
// Gives every text back as it is written in the source, which is English. Kirigami loads its
// translation for the system's language by itself and never lets go of it; for an English
// Kontra on a German system this goes in front of it (10.11.: "Seitenleiste schließen" beside
// "Hear the bass").
class SourceTexts : public QTranslator
{
public:
    using QTranslator::QTranslator;

    QString translate(const char *, const char *sourceText, const char *, int) const override
    {
        return QString::fromUtf8(sourceText);
    }

    bool isEmpty() const override
    {
        return false;
    }
};

const char *const SOURCE_LANGUAGE = "en";

// The translator installed last; a new language replaces it.
QTranslator *current = nullptr;
// The app's engine: a removed translator alone leaves Kirigami's texts as they were (10.11.).
QPointer<QQmlApplicationEngine> engine;
}

void translate_engine(QQmlApplicationEngine &app_engine)
{
    engine = &app_engine;
}

bool install_kirigami_translator(rust::Str language)
{
    if (current != nullptr) {
        QCoreApplication::removeTranslator(current);
        delete current;
        current = nullptr;
    }
    const auto lang = QString::fromUtf8(language.data(), static_cast<qsizetype>(language.size()));
    const auto subdir = QStringLiteral("locale/%1/LC_MESSAGES").arg(lang);
    const auto dirs = QStandardPaths::locateAll(QStandardPaths::GenericDataLocation, subdir,
                                                QStandardPaths::LocateDirectory);
    if (lang == QLatin1String(SOURCE_LANGUAGE)) {
        current = new SourceTexts(QCoreApplication::instance());
        QCoreApplication::installTranslator(current);
    }
    for (const auto &dir : dirs) {
        if (current != nullptr) {
            break;
        }
        auto *translator = new QTranslator(QCoreApplication::instance());
        if (translator->load(QStringLiteral("libkirigami6_qt"), dir)) {
            QCoreApplication::installTranslator(translator);
            current = translator;
            break;
        }
        delete translator;
    }
    // qsTr inside a larger expression (Kirigami: `collapsed ? "" : qsTr(…)`) is evaluated again
    // only when the engine's language changes; retranslate() covers plain qsTr bindings.
    if (engine) {
        engine->setUiLanguage(lang);
        engine->retranslate();
    }
    return current != nullptr;
}
