import QtQuick
import QtTest
import Kante
import ".."

// At start the sidebar opened and only then pushed the page aside, with a jerk (owner 12.11.).
// Until the sidebar rests the window stays hidden and the page takes the sidebar's room at
// once; later it waits for the slide's end (QA 11.11.). Run by scripts/check-video.sh.
Item {
    id: root
    width: 800
    height: 600

    // What the inset reads of Kirigami's drawer.
    QtObject {
        id: drawerMock
        property real width: 180
        property bool collapsed: false
        property bool drawerOpen: true
        property bool modal: false
        property real collapsedSize: 32
        property real leftPadding: 4
        property real rightPadding: 4
    }

    Component {
        id: freshInset
        SidebarInset { edge: 0; drawer: drawerMock }
    }

    TestCase {
        name: "SidebarInset"
        when: windowShown

        property int kindBefore: KanteStyle.Kind.System
        function initTestCase() {
            kindBefore = KanteStyle.kind
            KanteStyle.kind = KanteStyle.Kind.Kante
            verify(KanteStyle.animate)
        }
        function cleanupTestCase() { KanteStyle.kind = kindBefore }

        function test_atStartAtOnce() {
            var fresh = createTemporaryObject(freshInset, root)
            fresh.edge = 180
            compare(fresh.margin, 180)
        }

        // Shown once the sidebar stops changing, not at its first step.
        function test_readyWhenTheSidebarRests() {
            var fresh = createTemporaryObject(freshInset, root)
            fresh.edge = 60
            verify(!fresh.ready, "ready while the sidebar still moves")
            wait(fresh.restMs / 2)
            fresh.edge = 180
            wait(fresh.restMs / 2)
            verify(!fresh.ready, "ready before it rested")
            tryCompare(fresh, "ready", true, fresh.restMs * 4)
            fresh.edge = 0
            verify(fresh.ready, "hid again later")
        }

        // Opening or closing: the page takes its new width at once (one relayout) and follows
        // the sidebar's edge by moving only (owner 12.11.: it waited for the slide's end).
        function test_widthAtOnceThenFollows() {
            var fresh = createTemporaryObject(freshInset, root)
            tryCompare(fresh, "ready", true, fresh.restMs * 4)
            fresh.edge = 180
            drawerMock.drawerOpen = false
            fresh.edge = 170
            compare(fresh.margin, 0)
            compare(fresh.offset, 170)
            fresh.edge = 0
            compare(fresh.offset, 0)
            drawerMock.drawerOpen = true
            compare(fresh.margin, 180)
        }

        // Collapsing shrinks the drawer's width step by step: the page takes the collapsed
        // width at once, not each step's.
        function test_collapseAtOnce() {
            var fresh = createTemporaryObject(freshInset, root)
            tryCompare(fresh, "ready", true, fresh.restMs * 4)
            drawerMock.collapsed = true
            compare(fresh.margin, 40)
            drawerMock.width = 120
            compare(fresh.margin, 40)
            drawerMock.collapsed = false
            compare(fresh.margin, 180)
            drawerMock.width = 180
        }
    }
}
