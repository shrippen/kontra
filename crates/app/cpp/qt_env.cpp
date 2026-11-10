#include "qt_env.h"

#include <QByteArray>
#include <QtEnvironmentVariables>

void set_qt_env(rust::Str name, rust::Str value)
{
    const QByteArray key(name.data(), static_cast<qsizetype>(name.size()));
    qputenv(key.constData(), QByteArray(value.data(), static_cast<qsizetype>(value.size())));
}
