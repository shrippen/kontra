import QtQuick
import QtTest
import org.kde.kirigami as Kirigami
import ".."

// The sidebar with the keyboard (P7, QA 27.10., S3): Tab skips the separators, and the focused
// entry carries Kante's brackets, also the entry of the current page. Run by scripts/check-video.sh.
Item {
    id: root
    width: 640
    height: 480

    Kirigami.GlobalDrawer {
        id: drawer
        modal: false
        isMenu: false
        actions: [
            Kirigami.Action { text: "home"; checked: true },
            NavSeparator {},
            Kirigami.Action { text: "library" }
        ]
    }

    Loader {
        id: loader
        Component.onCompleted: setSource(Qt.resolvedUrl("../DrawerFocus.qml"), { drawer: drawer })
    }

    TestCase {
        name: "DrawerFocus"
        when: windowShown

        // The drawer entry with this text (Kirigami's focusable delegate carries its action as tAction).
        function entry(item, text) {
            if (item.tAction && item.activeFocusOnTab && item.tAction.text === text) {
                return item
            }
            for (var i = 0; i < item.children.length; i++) {
                var found = entry(item.children[i], text)
                if (found) {
                    return found
                }
            }
            return null
        }

        function test_tab_skips_separator() {
            drawer.open()
            var home = entry(drawer.contentItem, "home")
            verify(home !== null)
            home.forceActiveFocus(Qt.TabFocusReason)
            keyClick(Qt.Key_Tab)
            var focused = root.Window.activeFocusItem
            verify(focused === entry(drawer.contentItem, "library"),
                   "focus on " + (focused && focused.tAction ? focused.tAction.text || "separator" : focused))
        }

        function test_brackets_on_focused_entry() {
            drawer.open()
            var brackets = loader.item
            var home = entry(drawer.contentItem, "home")
            home.forceActiveFocus(Qt.TabFocusReason)
            tryCompare(brackets, "active", true)
            verify(brackets.parent === home)

            var library = entry(drawer.contentItem, "library")
            library.forceActiveFocus(Qt.TabFocusReason)
            verify(brackets.parent === library)
        }
    }
}
