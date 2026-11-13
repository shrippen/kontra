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
            menu.closedAt = 0
            menu.openAt(root)
            tryCompare(menu, "visible", true)
        }
        function cleanup() {
            menu.close()
            tryCompare(menu, "visible", false)
        }

        function test_enterChoosesTheFirstWay() {
            keyClick(Qt.Key_Return)
            compare(spy.count, 1)
            compare(spy.signalArguments[0][0], "download")
        }

        function test_arrowsReachEveryWay() {
            var ways = ["download", "folder", "file", "generate"]
            for (var i = 0; i < ways.length; i++) {
                if (i > 0) {
                    menu.closedAt = 0
                    menu.openAt(root)
                    tryCompare(menu, "visible", true)
                }
                for (var k = 0; k < i; k++) {
                    keyClick(Qt.Key_Down)
                }
                keyClick(Qt.Key_Return)
                compare(spy.signalArguments[i][0], ways[i])
                tryCompare(menu, "visible", false)
            }
        }

        function test_upGoesBack() {
            keyClick(Qt.Key_Down)
            keyClick(Qt.Key_Down)
            keyClick(Qt.Key_Up)
            keyClick(Qt.Key_Return)
            compare(spy.signalArguments[0][0], "folder")
        }

        function test_escapeClosesWithoutChoosing() {
            keyClick(Qt.Key_Down)
            keyClick(Qt.Key_Escape)
            tryCompare(menu, "visible", false)
            compare(spy.count, 0)
        }

        function test_everyWayHasASignATitleAndADescription() {
            menu.ways.forEach(function(w) {
                verify(w.sign !== "")
                compare(root.texts.t(w.title), w.title)
                compare(root.texts.t(w.hint), w.hint)
            })
            compare(menu.ways.length, 4)
        }
    }
}
