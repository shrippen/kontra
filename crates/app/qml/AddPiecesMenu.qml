import QtQuick
import QtQuick.Controls as QQC2
import Kante

// "Add pieces" (LH-BIB-17): the four ways in, a menu at a button. Arrow keys move, Enter chooses,
// Esc closes (the platform's menu). Each way opens its own dialog; the page takes the choice.
//
//   Download …                 songs from the web
//   Ordner hinzufügen …        a folder on disk, linked or imported
//   Datei öffnen …             one file (it can also be dragged onto the page)
//   ───────────────
//   Neues Stück generieren …   from a style
//   Tip: files and folders can be dragged onto the library any time.
QQC2.Menu {
    id: menu

    required property var texts

    // "download", "folder", "file" or "generate".
    signal chosen(string way)

    KantePopupSkin { popup: menu }

    KanteMenuItem {
        text: menu.texts.t("library.add.download")
        iconRoom: true
        onTriggered: menu.chosen("download")
    }
    KanteMenuItem {
        text: menu.texts.t("library.add.folder")
        iconRoom: true
        onTriggered: menu.chosen("folder")
    }
    KanteMenuItem {
        text: menu.texts.t("library.open")
        valueText: menu.texts.t("library.add.file.hint")
        iconRoom: true
        onTriggered: menu.chosen("file")
    }
    QQC2.MenuSeparator {}
    KanteMenuItem {
        text: menu.texts.t("library.add.generate")
        iconRoom: true
        onTriggered: menu.chosen("generate")
    }
    // Not an item: the keyboard passes over it, a screen reader reads it after the items.
    KanteMenuItem {
        text: menu.texts.t("library.add.tip")
        enabled: false
        iconRoom: true
    }
}
