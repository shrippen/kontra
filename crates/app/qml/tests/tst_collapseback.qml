import QtQuick
import QtTest
import org.kde.kirigami as Kirigami
import ".."

// The sidebar after the presentation (QA 28.10., B8, P4): Kirigami turns `collapsible` off when
// the drawer turns modal and never back on, so after Esc the sidebar had no "Close Sidebar"
// button and less width. Run by scripts/check-video.sh.
Item {
    id: root
    width: 900
    height: 600

    Kirigami.GlobalDrawer {
        id: drawer
        modal: false
        isMenu: false
        collapsible: true
        actions: [ Kirigami.Action { text: "home" } ]
    }

    CollapseBack { drawer: drawer }

    TestCase {
        name: "CollapseBack"
        when: windowShown

        function test_collapsibleAfterModal() {
            verify(drawer.collapsible, "collapsible at start")
            drawer.modal = true
            wait(50)
            drawer.modal = false
            tryVerify(function () { return drawer.collapsible }, 1000, "collapsible after modal")
        }
    }
}
