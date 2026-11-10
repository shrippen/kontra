#include "translator.h"

#include <QCoreApplication>
#include <QEvent>
#include <QLibrary>
#include <QPointer>
#include <QStandardPaths>
#include <QTranslator>

#include <string>

namespace {
// Gives every text back as it is written in the source, which is English. Kirigami loads its
// translation for the system's language by itself and never lets go of it; for an English
// Kontra on a German system this goes in front of it (10.11.: "Seitenleiste schließen" beside
// "Hear the bass").
class SourceTexts : public QTranslator
{
public:
    using QTranslator::QTranslator;

    QString translate(const char *, const char *sourceText, const char *, int n) const override
    {
        // A text with a count ("%n day(s)") is left to the English plural catalogs.
        if (n >= 0) {
            return QString();
        }
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

// Keeps `current` in front. Qt asks the translator installed last first, and Kirigami's
// libraries install their own for the system's language whenever one of them loads (with the
// first page, and with later ones). Each install tells the app (LanguageChange); this puts
// ours behind it again in the list, which is in front for Qt.
class KeepInFront : public QObject
{
public:
    using QObject::QObject;

protected:
    bool eventFilter(QObject *object, QEvent *event) override
    {
        const bool changed = event->type() == QEvent::LanguageChange
            && object == QCoreApplication::instance();
        if (changed && current != nullptr && !moving) {
            // Removing and installing tell the app again: not to be answered.
            moving = true;
            QCoreApplication::removeTranslator(current);
            QCoreApplication::installTranslator(current);
            moving = false;
        }
        return QObject::eventFilter(object, event);
    }

private:
    bool moving = false;
};

KeepInFront *watcher = nullptr;
// The app's engine: a removed translator alone leaves Kirigami's texts as they were (10.11.).
QPointer<QQmlApplicationEngine> engine;
}

namespace {
QTranslator *source_texts()
{
    return new SourceTexts(QCoreApplication::instance());
}

// Kirigami's catalog for `lang` from the XDG data folders; nullptr if there is none.
QTranslator *catalog_of(const QString &lang)
{
    const auto subdir = QStringLiteral("locale/%1/LC_MESSAGES").arg(lang);
    const auto dirs = QStandardPaths::locateAll(QStandardPaths::GenericDataLocation, subdir,
                                                QStandardPaths::LocateDirectory);
    for (const auto &dir : dirs) {
        auto *translator = new QTranslator(QCoreApplication::instance());
        if (translator->load(QStringLiteral("libkirigami6_qt"), dir)) {
            return translator;
        }
        delete translator;
    }
    return nullptr;
}
}

void translate_engine(QQmlApplicationEngine &app_engine)
{
    engine = &app_engine;
}

bool install_kirigami_translator(rust::Str language)
{
    // Forgotten first: removing tells the app, and the watcher must not put it back.
    QTranslator *old = current;
    current = nullptr;
    if (old != nullptr) {
        QCoreApplication::removeTranslator(old);
        delete old;
    }
    if (watcher == nullptr) {
        watcher = new KeepInFront(QCoreApplication::instance());
        QCoreApplication::instance()->installEventFilter(watcher);
    }
    const auto lang = QString::fromUtf8(language.data(), static_cast<qsizetype>(language.size()));
    current = lang == QLatin1String(SOURCE_LANGUAGE) ? source_texts() : catalog_of(lang);
    if (current != nullptr) {
        QCoreApplication::installTranslator(current);
    }
    // qsTr inside a larger expression (Kirigami: `collapsed ? "" : qsTr(…)`) is evaluated again
    // only when the engine's language changes; retranslate() covers plain qsTr bindings.
    if (engine) {
        engine->setUiLanguage(lang);
        engine->retranslate();
    }
    return current != nullptr;
}

rust::String translated(rust::Str context, rust::Str source)
{
    const std::string where(context.data(), context.size());
    const std::string what(source.data(), source.size());
    const auto text = QCoreApplication::translate(where.c_str(), what.c_str());
    return rust::String(text.toUtf8().constData());
}

bool load_kirigami()
{
    QLibrary library(QStringLiteral("Kirigami"), 6);
    return library.load();
}

