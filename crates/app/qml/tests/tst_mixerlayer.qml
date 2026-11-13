import QtQuick
import QtQuick.Layouts
import QtTest
import Kante
import ".."

// The mixer used the notes area's room: 60 % of 250 px, two rows of a mixer with many (owner
// 13.11.). It now reaches over what lies under the notes, to 2/3 of the page, and stops above the
// transport. Run by scripts/check-video.sh.
Item {
    id: root
    width: 1000
    height: 900

    property bool resizing: false
    function applicationWindow() { return root }

    ColumnLayout {
        id: column
        anchors.fill: parent
        spacing: 0
        Item { Layout.fillWidth: true; Layout.preferredHeight: 200 }
        NotesArea {
            id: notes
            Layout.fillWidth: true
            Layout.fillHeight: true
        }
        Item { id: fretboard; Layout.fillWidth: true; Layout.preferredHeight: 150 }
        Item { id: transport; Layout.fillWidth: true; Layout.preferredHeight: 60 }
    }

    MixerLayer {
        id: layer
        column: column
        startRow: notes
        stopRow: transport
        open: true
    }

    TestCase {
        name: "MixerLayer"
        when: windowShown

        function init() { root.height = 900 }

        function test_reaches_over_what_lies_under_the_notes() {
            // 900 page: notes 200..690 (490), fretboard 690..840, transport 840..900.
            compare(layer.y, 200)
            compare(layer.height, 640)
            compare(layer.children[0].maxHeight, 600)
            verify(layer.children[0].maxHeight > notes.height * 0.6)
        }

        function test_stops_above_the_transport() {
            // A low window: 2/3 of the page is more than the room above the transport.
            root.height = 500
            compare(layer.height, 240)
            compare(layer.children[0].maxHeight, 240)
            verify(layer.y + layer.height <= transport.y + column.y)
        }
    }
}
