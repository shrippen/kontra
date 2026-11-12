import QtQuick
import QtQuick.Layouts
import QtTest
import Kante
import ".."

// The notes take the room the practice page's column leaves (owner 12.11.: one tab line in
// the middle of an empty page). Call and response is a layout itself; a nested layout with
// no child shown fills by default and took half the room. The song map stands beside it.
// Run by scripts/check-video.sh.
Item {
    id: root
    width: 1200
    height: 900

    // The components read ctl and texts from the application window.
    readonly property QtObject texts: QtObject {
        function t(key) { return key }
    }
    readonly property QtObject ctl: QtObject {
        property bool playing: false
        property bool call: false
        property bool callHidden: false
        function changeCall(on) {}
        function songMap() {
            return JSON.stringify({ values: [1, 0.5], labels: ["1", "2"], tips: [], loop: null, played: true })
        }
    }
    function applicationWindow() { return root }

    ColumnLayout {
        id: column
        anchors.fill: parent
        spacing: 0
        SongMap {
            id: map
            Layout.fillWidth: true
        }
        CallResponse {
            id: call
            Layout.fillWidth: true
        }
        Item {
            id: notes
            Layout.fillWidth: true
            Layout.fillHeight: true
        }
    }

    TestCase {
        name: "PracticeColumn"
        when: windowShown

        function test_notesTakeTheRoom() {
            waitForRendering(column)
            verify(map.visible)
            compare(call.height, 0)
            compare(notes.height, column.height - map.implicitHeight)
        }
    }
}
