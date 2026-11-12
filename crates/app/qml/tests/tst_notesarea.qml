import QtQuick
import QtQuick.Layouts
import QtTest
import Kante
import ".."

// A row comes above the notes (owner 12.11.: from "Frei spielen" to "Werkstatt" the notes
// jumped down, then slid to their new lines). They glide there in one motion instead: right
// after the change they still stand where they stood. Run by scripts/check-video.sh.
Item {
    id: root
    width: 800
    height: 600

    property bool resizing: false
    function applicationWindow() { return root }

    ColumnLayout {
        id: column
        anchors.fill: parent
        spacing: 0
        Rectangle {
            id: row
            visible: false
            Layout.fillWidth: true
            Layout.preferredHeight: 80
        }
        NotesArea {
            id: area
            Layout.fillWidth: true
            Layout.fillHeight: true
            Rectangle {
                id: notes
                anchors.fill: parent
            }
        }
    }

    // A practice page just opened, its notes under a row.
    Component {
        id: opened
        ColumnLayout {
            anchors.fill: parent
            spacing: 0
            property alias notes: freshNotes
            Rectangle {
                Layout.fillWidth: true
                Layout.preferredHeight: 200
            }
            NotesArea {
                Layout.fillWidth: true
                Layout.fillHeight: true
                Rectangle {
                    id: freshNotes
                    anchors.fill: parent
                }
            }
        }
    }

    TestCase {
        name: "NotesArea"
        when: windowShown

        // Kante's own style animates; the system style (the test's default) does not.
        property int kindBefore: KanteStyle.Kind.System
        function initTestCase() {
            kindBefore = KanteStyle.kind
            KanteStyle.kind = KanteStyle.Kind.Kante
        }
        function cleanupTestCase() { KanteStyle.kind = kindBefore }

        function screenY() { return notes.mapToItem(root, 0, 0).y }

        // The first place comes from the layout, not from a row coming: no glide.
        function test_firstPlaceAtOnce() {
            var page = createTemporaryObject(opened, root)
            waitForRendering(page)
            compare(page.notes.mapToItem(root, 0, 0).y, 200)
        }

        function test_glidesToTheNewPlace() {
            verify(KanteStyle.animate)
            waitForRendering(column)
            compare(screenY(), 0)
            row.visible = true
            waitForRendering(column)
            verify(screenY() < 40, "jumped to " + screenY())
            tryVerify(function() { return screenY() === 80 }, KanteStyle.durationSlow * 4)
            row.visible = false
            waitForRendering(column)
            tryVerify(function() { return screenY() === 0 }, KanteStyle.durationSlow * 4)
        }

        // A window being resized moves nothing slowly: the notes follow at once.
        function test_followsResizeAtOnce() {
            root.resizing = true
            row.visible = true
            waitForRendering(column)
            compare(screenY(), 80)
            row.visible = false
            waitForRendering(column)
            compare(screenY(), 0)
            root.resizing = false
        }
    }
}
