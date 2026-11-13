import QtQuick
import QtQuick.Controls as QQC2
import QtQuick.Layouts
import Kante

// One setlist (LH-BIB-04, J-B1): an order of pieces for a gig; playing one offers the next.
// Add a piece of the library without opening it, move, remove, delete the list.
ColumnLayout {
    id: setlist

    required property var ctl
    required property var texts
    required property string name

    readonly property var lists: JSON.parse(ctl.setlistsJson)
    readonly property var current: lists.filter(function(l) { return l.name === name })[0] || ({ name: name, pieces: [] })

    signal openRequested(string name, int index)
    // The list is gone; the page shows another source.
    signal deleted()

    spacing: KanteStyle.unit(8)

    RowLayout {
        Layout.fillWidth: true
        QQC2.Label {
            Layout.fillWidth: true
            text: setlist.current.name + " · " + setlist.texts.t(setlist.current.pieces.length === 1 ? "setlist.count.one" : "setlist.count", { n: setlist.current.pieces.length })
            color: KanteStyle.strongTextColor
            font.bold: true
            elide: Text.ElideRight
        }
        // Any piece of the library, without opening it first (QA 14.10., P2).
        KanteComboBox {
            readonly property var pieces: JSON.parse(setlist.ctl.recentJson)
            visible: pieces.length > 0
            Layout.preferredWidth: KanteStyle.unit(240)
            Accessible.name: setlist.texts.t("setlist.add") + ": " + setlist.current.name
            displayText: setlist.texts.t("setlist.add")
            model: pieces.map(function(p) { return p.title })
            currentIndex: -1
            onActivated: function(index) {
                setlist.ctl.setlistAdd(setlist.current.name, pieces[index].path)
                currentIndex = -1
            }
        }
        KanteButton {
            visible: setlist.ctl.loaded
            text: setlist.texts.t("setlist.addloaded")
            emphasis: KanteButton.Emphasis.Quiet
            onClicked: setlist.ctl.setlistAddLoaded(setlist.current.name)
        }
        // Two steps: a setlist is gone for good (the pieces stay in the library).
        KanteButton {
            property bool armed: false
            text: armed ? setlist.texts.t("setlist.delete.sure") : setlist.texts.t("setlist.delete")
            emphasis: armed ? KanteButton.Emphasis.Primary : KanteButton.Emphasis.Quiet
            onClicked: {
                if (!armed) {
                    armed = true
                    return
                }
                setlist.ctl.setlistDelete(setlist.current.name)
                setlist.deleted()
            }
            onActiveFocusChanged: if (!activeFocus) armed = false
            Timer { running: parent.armed; interval: 4000; onTriggered: parent.armed = false }
        }
    }

    QQC2.Label {
        visible: setlist.current.pieces.length === 0
        text: setlist.texts.t("library.setlist.empty")
        color: KanteStyle.mutedTextColor
    }

    ListView {
        Layout.fillWidth: true
        Layout.fillHeight: true
        clip: true
        model: setlist.current.pieces
        QQC2.ScrollBar.vertical: QQC2.ScrollBar {}
        delegate: RowLayout {
            id: entry
            required property var modelData
            required property int index
            width: ListView.view.width
            KanteListRow {
                Layout.fillWidth: true
                leadingText: String(entry.index + 1)
                text: entry.modelData.title
                subtitle: entry.modelData.artist
                disabledReason: entry.modelData.missing ? setlist.texts.t("setlist.missing") : ""
                enabled: !entry.modelData.missing
                onClicked: setlist.openRequested(setlist.current.name, entry.index)
            }
            KanteButton {
                Layout.preferredWidth: KanteStyle.unit(44)
                text: "↑"; emphasis: KanteButton.Emphasis.Quiet; enabled: entry.index > 0
                Accessible.name: setlist.texts.t("setlist.up")
                onClicked: setlist.ctl.setlistMove(setlist.current.name, entry.index, -1)
            }
            KanteButton {
                Layout.preferredWidth: KanteStyle.unit(44)
                text: "↓"; emphasis: KanteButton.Emphasis.Quiet; enabled: entry.index < setlist.current.pieces.length - 1
                Accessible.name: setlist.texts.t("setlist.down")
                onClicked: setlist.ctl.setlistMove(setlist.current.name, entry.index, 1)
            }
            KanteButton {
                Layout.preferredWidth: KanteStyle.unit(44)
                text: "×"; emphasis: KanteButton.Emphasis.Quiet
                Accessible.name: setlist.texts.t("setlist.remove")
                onClicked: setlist.ctl.setlistRemove(setlist.current.name, entry.index)
            }
        }
    }
}
