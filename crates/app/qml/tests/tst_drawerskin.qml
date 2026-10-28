import QtQuick
import QtTest
import QtQuick.Templates as T
import org.kde.kirigami as Kirigami
import Kante
import ".."

// The sidebar in Kante's colours (QA 28.10., P1 with a light system theme: white below the
// entries, "Close Sidebar" at 1.4:1), and its separators silent for screen readers (P7). Run by
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
            Kirigami.Action { text: "library" },
            Kirigami.Action { text: "locked"; enabled: false }
        ]
    }

    Loader {
        id: loader
        Component.onCompleted: setSource(Qt.resolvedUrl("../DrawerSkin.qml"), { drawer: drawer })
    }

    TestCase {
        name: "DrawerSkin"
        when: windowShown

        function cleanup() {
            KanteStyle.kind = KanteStyle.Kind.System
        }

        // The first item below `item` that `test` accepts.
        function find(item, test) {
            if (test(item)) {
                return item
            }
            for (var i = 0; i < item.children.length; i++) {
                var found = find(item.children[i], test)
                if (found) {
                    return found
                }
            }
            return null
        }

        // Kirigami's delegate of an action: a button carrying it as tAction.
        function entry(test) {
            return find(drawer.contentItem, function(it) { return it.tAction && it instanceof T.AbstractButton && test(it.tAction) })
        }
        function separator() {
            return entry(function(action) { return action.separator })
        }

        function test_kante_background() {
            KanteStyle.kind = KanteStyle.Kind.Kante
            drawer.open()
            tryCompare(drawer.background, "color", KanteStyle.dialogColor)
        }

        // Back to System: the platform's colour again.
        function test_system_background() {
            drawer.open()
            var platform = drawer.background.color
            KanteStyle.kind = KanteStyle.Kind.Kante
            KanteStyle.kind = KanteStyle.Kind.System
            tryCompare(drawer.background, "color", platform)
        }

        function test_separator_silent() {
            drawer.open()
            var line = separator()
            verify(line !== null)
            tryCompare(line.Accessible, "ignored", true)
        }

        // Kirigami paints disabled entries in the platform's colours, whatever Kante sets: in
        // Kante they show the drawer's ground and Kante's rule.
        function test_disabled_entries_kante() {
            KanteStyle.kind = KanteStyle.Kind.Kante
            drawer.open()
            var locked = entry(function(action) { return action.text === "locked" })
            tryCompare(locked.background, "visible", false)
            var line = find(separator(), function(it) { return it.weight !== undefined && it.color !== undefined })
            tryCompare(line, "color", KanteStyle.ruleColor)
        }

        function test_disabled_entries_system() {
            drawer.open()
            var locked = entry(function(action) { return action.text === "locked" })
            tryCompare(locked.background, "visible", true)
        }
    }
}
