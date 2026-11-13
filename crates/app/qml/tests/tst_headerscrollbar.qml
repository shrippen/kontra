import QtQuick
import QtQuick.Controls as QQC2
import QtTest
import ".."

// The settings page's scroll bar under its sticky header: the handle starts below the header and,
// scrolled to the end, still ends inside the Flickable (review of #321: it ended 78 px below).
// Run by scripts/check-video.sh.
Item {
    id: root
    width: 400
    height: 300

    Flickable {
        id: flick
        anchors.fill: parent
        clip: true
        contentHeight: 2000
        QQC2.ScrollBar.vertical: HeaderScrollBar {
            id: bar
            headerHeight: root.headerHeight
            policy: QQC2.ScrollBar.AlwaysOn
        }
    }

    property real headerHeight: 80

    TestCase {
        name: "HeaderScrollBar"
        when: windowShown

        function init() {
            root.headerHeight = 80
            flick.contentY = 0
        }

        // Handle edges in the Flickable's coordinates.
        function handleTop() { return bar.contentItem.mapToItem(flick, 0, 0).y }
        function handleBottom() { return handleTop() + bar.contentItem.height }

        function test_starts_below_the_header() {
            waitForRendering(root)
            verify(handleTop() >= 80, "handle top " + handleTop())
        }

        function test_ends_inside_at_the_end() {
            flick.contentY = flick.contentHeight - flick.height
            waitForRendering(root)
            verify(handleBottom() <= flick.height, "handle bottom " + handleBottom())
        }

        function test_follows_a_smaller_header() {
            root.headerHeight = 50
            flick.contentY = flick.contentHeight - flick.height
            waitForRendering(root)
            verify(handleBottom() <= flick.height, "handle bottom " + handleBottom())
            flick.contentY = 0
            waitForRendering(root)
            verify(handleTop() >= 50 && handleTop() < 80, "handle top " + handleTop())
        }
    }
}
