import QtQuick
import QtQuick.Controls as QQC2
import QtTest
import Kante
import ".."

// "Add pieces" (LH-BIB-17): the platform menu takes arrows, Enter and Esc; each way reports
// itself, the disabled tip is passed over. Run by scripts/check-video.sh.
Item {
    id: root
    width: 600
    height: 400

    readonly property QtObject texts: QtObject {
        function t(key, args) { return key }
    }

    AddPiecesMenu {
        id: menu
        texts: root.texts
    }
    SignalSpy { id: spy; target: menu; signalName: "chosen" }

    TestCase {
        name: "AddPiecesMenu"
        when: windowShown

        function init() {
            spy.clear()
            menu.popup(root, 10, 10)
            tryCompare(menu, "visible", true)
            menu.forceActiveFocus()
        }
        function cleanup() {
            menu.close()
            tryCompare(menu, "visible", false)
        }

        function test_enterChoosesTheHighlightedWay() {
            keyClick(Qt.Key_Down)
            keyClick(Qt.Key_Return)
            compare(spy.count, 1)
            compare(spy.signalArguments[0][0], "download")
        }

        function test_arrowsReachEveryWayAndSkipTheTip() {
            var ways = ["download", "folder", "file", "generate"]
            var chosen = []
            for (var i = 0; i < ways.length; i++) {
                menu.popup(root, 10, 10)
                tryCompare(menu, "visible", true)
                for (var k = 0; k <= i; k++) {
                    keyClick(Qt.Key_Down)
                }
                keyClick(Qt.Key_Return)
                chosen.push(spy.signalArguments[i][0])
                tryCompare(menu, "visible", false)
            }
            compare(chosen, ways)
        }

        function test_escapeClosesWithoutChoosing() {
            keyClick(Qt.Key_Down)
            keyClick(Qt.Key_Escape)
            tryCompare(menu, "visible", false)
            compare(spy.count, 0)
        }

        function test_itemsHaveAccessibleNames() {
            var named = 0
            for (var i = 0; i < menu.count; i++) {
                var item = menu.itemAt(i)
                if (item && item.Accessible && item.Accessible.name !== "") {
                    named++
                }
            }
            verify(named >= 4, "named items: " + named)
        }
    }
}
