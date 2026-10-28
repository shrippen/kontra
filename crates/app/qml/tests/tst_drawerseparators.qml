import QtQuick
import QtTest
import QtQuick.Templates as T
import org.kde.kirigami as Kirigami
import ".."

// The sidebar's separators are silent for screen readers (QA 28.10., P7). Run by
// scripts/check-video.sh.
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

    DrawerSeparators { drawer: drawer }

    TestCase {
        name: "DrawerSeparators"
        when: windowShown

        // Kirigami's delegate of the first action `test` accepts: a button carrying it as tAction.
        function entry(item, test) {
            if (item.tAction && item instanceof T.AbstractButton && test(item.tAction)) {
                return item
            }
            for (var i = 0; i < item.children.length; i++) {
                var found = entry(item.children[i], test)
                if (found) {
                    return found
                }
            }
            return null
        }

        function test_separator_silent() {
            drawer.open()
            var line = entry(drawer.contentItem, function(action) { return action.separator })
            verify(line !== null)
            tryCompare(line.Accessible, "ignored", true)
            var home = entry(drawer.contentItem, function(action) { return action.text === "home" })
            compare(home.Accessible.ignored, false)
        }
    }
}
