import QtQuick
import QtQuick.Layouts
import QtQuick.Controls as QQC2
import Kante

// "Add pieces" (LH-BIB-17): the four ways in, in a popover at a button. Each way is its sign, a
// title and one line of what it does. Up and Down move, Enter chooses, Esc closes. The page
// takes the choice and opens the matching dialog.
//
//   ↓ Download …              Songsterr oder MuseScore durchsuchen
//   □ Ordner hinzufügen …     Ordner auf der Festplatte verknüpfen
//   ● Datei öffnen …          MIDI, Guitar Pro, MusicXML; oder auf die Seite ziehen
//   ─────────────────────────
//   ◇ Neues Stück generieren …  Stil, Tonart, Länge und Variante wählen
//   Tipp: Dateien und Ordner lassen sich jederzeit auf die Bibliothek ziehen.
KantePopover {
    id: menu

    required property var texts

    // "download", "folder", "file" or "generate".
    signal chosen(string way)

    readonly property var ways: [
        { id: "download", sign: "↓", title: "library.add.download", hint: "library.add.download.hint" },
        { id: "folder", sign: "□", title: "library.add.folder", hint: "library.add.folder.hint" },
        { id: "file", sign: "●", title: "library.open", hint: "library.add.file.hint" },
        { id: "generate", sign: "◇", title: "library.add.generate", hint: "library.add.generate.hint" }
    ]

    function pick(way) {
        close()
        chosen(way)
    }

    onOpened: rows.itemAt(0).row.forceActiveFocus()

    Accessible.role: Accessible.PopupMenu
    Accessible.name: texts.t("library.add")

    ColumnLayout {
        width: KanteStyle.unit(380)
        spacing: 0

        Repeater {
            id: rows
            model: menu.ways
            delegate: ColumnLayout {
                id: way
                required property int index
                required property var modelData
                readonly property alias row: wayRow
                Layout.fillWidth: true
                spacing: 0

                // The separator before the way that makes something new.
                Rectangle {
                    visible: way.modelData.id === "generate"
                    Layout.fillWidth: true
                    Layout.topMargin: KanteStyle.unit(4)
                    Layout.bottomMargin: KanteStyle.unit(4)
                    height: 1
                    color: KanteStyle.ruleColor
                }

                // Arrows between the rows (KanteListRow only takes Tab).
                KanteListRow {
                    id: wayRow
                    Layout.fillWidth: true
                    rule: false
                    density: KanteListRow.Density.Comfortable
                    leadingText: way.modelData.sign
                    text: menu.texts.t(way.modelData.title)
                    subtitle: menu.texts.t(way.modelData.hint)
                    onClicked: menu.pick(way.modelData.id)
                    Keys.onDownPressed: {
                        var next = rows.itemAt(way.index + 1)
                        if (next) {
                            next.row.forceActiveFocus()
                        }
                    }
                    Keys.onUpPressed: {
                        var before = rows.itemAt(way.index - 1)
                        if (before) {
                            before.row.forceActiveFocus()
                        }
                    }
                }
            }
        }

        QQC2.Label {
            Layout.fillWidth: true
            Layout.topMargin: KanteStyle.unit(8)
            Layout.leftMargin: KanteStyle.unit(12)
            wrapMode: Text.WordWrap
            text: menu.texts.t("library.add.tip")
            color: KanteStyle.mutedTextColor
        }
    }
}
