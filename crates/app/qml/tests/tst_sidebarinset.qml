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

    Component {
        id: freshInset
        SidebarInset { edge: 0 }
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

        function test_laterAfterTheSlide() {
            var fresh = createTemporaryObject(freshInset, root)
            fresh.edge = 180
            tryCompare(fresh, "ready", true, fresh.restMs * 4)
            fresh.edge = 0
            compare(fresh.margin, 180)
            tryCompare(fresh, "margin", 0, KanteStyle.durationSlow * 3)
        }
    }
}
