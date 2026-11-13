import QtQuick
import QtQuick.Controls as QQC2

// Vertical scroll bar of a Flickable whose top lies under a sticky header (settings page): the
// whole bar runs below the header, so the header never hides it (review round 24).
//
// Not attached: an attached bar takes the Flickable's full height, and org.kde.desktop draws
// track and handle over that height whatever the padding (reviews of #321). So it is a sibling
// that syncs both ways. Position is assigned, not bound: the desktop style assigns it on clicks,
// which would drop a binding.
QQC2.ScrollBar {
    id: bar

    required property Flickable flickable
    // Height of the header over the Flickable's top edge.
    property real headerHeight: 0

    // Guards the Flickable → bar update against echoing back.
    property bool syncing: false

    orientation: Qt.Vertical
    x: flickable.x + flickable.width - width
    y: flickable.y + headerHeight
    height: Math.max(0, flickable.height - headerHeight)
    size: flickable.visibleArea.heightRatio
    active: hovered || pressed || flickable.moving

    function follow() {
        syncing = true
        position = flickable.visibleArea.yPosition
        syncing = false
    }

    onPositionChanged: {
        if (syncing) {
            return
        }
        flickable.contentY = flickable.originY + position * flickable.contentHeight
    }

    Connections {
        target: bar.flickable
        function onContentYChanged() { bar.follow() }
        function onContentHeightChanged() { bar.follow() }
        function onHeightChanged() { bar.follow() }
    }

    Component.onCompleted: follow()
}
