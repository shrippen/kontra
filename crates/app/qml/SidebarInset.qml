import QtQuick
import Kante

// The page's left margin beside the sidebar. When the sidebar opens or closes the page takes
// its new width at once, one relayout, and follows the sidebar's edge by moving only: a
// relayout on every frame of the slide stuttered (QA 11.11., P4), waiting for the slide's end
// left the page still, then jumping (owner 12.11.).
//
//   start:  margin = target (new width)   offset = edge - target
//   slide:  offset → 0 with the edge      (a move, no relayout)
QtObject {
    id: inset

    /// How far the sidebar reaches into the window now.
    required property real edge
    /// The sidebar (Kirigami.GlobalDrawer): where its edge ends up once it slid or changed
    /// width. Its width grows or shrinks on collapse; the page takes the end width at once.
    property var drawer: null
    readonly property real collapsedWidth: drawer ? drawer.collapsedSize + drawer.leftPadding + drawer.rightPadding : 0
    // The widest seen open: the end of an expanding animation.
    property real openWidth: 0
    readonly property real target: !drawer || drawer.modal || !drawer.drawerOpen ? 0
        : drawer.collapsed ? collapsedWidth : Math.max(openWidth, drawer.width)
    Component.onCompleted: if (drawer && !drawer.collapsed) { openWidth = drawer.width }
    readonly property Connections drawerWidth: Connections {
        target: inset.drawer
        function onWidthChanged() {
            if (!inset.drawer.collapsed) {
                inset.openWidth = Math.max(inset.openWidth, inset.drawer.width)
            }
        }
    }
    /// At start: the sidebar has its width and place (no change for `restMs`), the window may
    /// show. It found them in steps (none, collapsed, open), each one moving the page (owner
    /// 12.11.). Until then the page follows the edge itself.
    property bool ready: false
    readonly property int restMs: 150
    /// The margin the page is laid out with, and how far it is moved from there.
    readonly property real margin: ready && KanteStyle.animate ? target : edge
    readonly property real offset: edge - margin

    onEdgeChanged: {
        if (!ready) {
            rest.restart()
        }
    }

    readonly property Timer rest: Timer {
        interval: inset.restMs
        running: true
        onTriggered: inset.ready = true
    }
}
