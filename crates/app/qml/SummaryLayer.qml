import QtQuick
import Kante

// The summary over the notes, covering them completely (QA P1/P4: the page showed through the
// card). The sheet loads in the background after the page; one asked for earlier opens when it
// is there.
Item {
    id: layer

    // What the sheet hands the keyboard back to, and what it hides meanwhile.
    property Item returnTo
    property var behind
    // How far the dimming reaches beyond this item, to the page's edge.
    property real bleed: 0
    readonly property bool shown: loader.item ? loader.item.visible : wanted
    property bool wanted: false

    function open() {
        if (loader.item) {
            loader.item.open()
            return
        }
        wanted = true
    }

    function close() {
        wanted = false
        if (loader.item) {
            loader.item.close()
        }
    }

    z: 5

    Rectangle {
        anchors.fill: parent
        anchors.margins: -layer.bleed
        visible: layer.shown
        color: Qt.rgba(KanteStyle.backgroundColor.r, KanteStyle.backgroundColor.g, KanteStyle.backgroundColor.b, 0.92)
        MouseArea { anchors.fill: parent; onClicked: layer.close() }
    }

    Loader {
        id: loader
        anchors.fill: parent
        asynchronous: true
        onLoaded: if (layer.wanted) item.open()
        sourceComponent: SummarySheet {
            anchors.centerIn: parent
            width: Math.min(layer.width - KanteStyle.unit(40), KanteStyle.unit(640))
            height: Math.min(contentHeight, layer.height - KanteStyle.unit(24))
            // Closed, the keyboard goes on at play; it stayed on the hidden "close" (QA 28.10., P7).
            returnTo: layer.returnTo
            behind: layer.behind
            onPracticeBar: function(bar) { practiceBars(bar, bar) }
            onPracticeBars: function(from, to) {
                ctl.changeMode("workshop")
                ctl.loopBars(from, to)
                close()
            }
        }
    }
}
