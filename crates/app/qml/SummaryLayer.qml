import QtQuick
import Kante

// The summary over the notes, covering them completely (QA P1/P4: the page showed through the
// card). The sheet loads in the background after the page; one asked for earlier opens when it
// is there.
Item {
    // Not "layer": every Item has a property of that name, and inside the loaded sheet it
    // hid this id (review #303).
    id: summaryLayer

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
        anchors.margins: -summaryLayer.bleed
        visible: summaryLayer.shown
        color: Qt.rgba(KanteStyle.backgroundColor.r, KanteStyle.backgroundColor.g, KanteStyle.backgroundColor.b, 0.92)
        MouseArea { anchors.fill: parent; onClicked: summaryLayer.close() }
    }

    // Centred and sized by the sheet: a loader with a size of its own stretches what it loads.
    Loader {
        id: loader
        anchors.centerIn: parent
        asynchronous: true
        onLoaded: if (summaryLayer.wanted) item.open()
        sourceComponent: SummarySheet {
            width: Math.min(summaryLayer.width - KanteStyle.unit(40), KanteStyle.unit(640))
            height: Math.min(contentHeight, summaryLayer.height - KanteStyle.unit(24))
            // Closed, the keyboard goes on at play; it stayed on the hidden "close" (QA 28.10., P7).
            returnTo: summaryLayer.returnTo
            behind: summaryLayer.behind
            onPracticeBar: function(bar) { practiceBars(bar, bar) }
            onPracticeBars: function(from, to) {
                ctl.changeMode("workshop")
                ctl.loopBars(from, to)
                close()
            }
        }
    }
}
