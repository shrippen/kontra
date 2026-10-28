import QtQuick
import QtTest
import Kante
import ".."

// A "?" inside a list row (exercise list, QA 28.10. B5, P7): once the "?" had the focus, Tab
// and Shift+Tab skipped its row, because the row (a focus scope) handed its focus on to the
// "?". Every row stays reachable. Run by scripts/check-video.sh.
Item {
    id: root
    width: 640
    height: 480

    // TermTip reads ctl and texts from the application window; keys stand for the texts.
    readonly property QtObject texts: QtObject {
        readonly property var map: ({})
        function t(key) { return key }
    }
    readonly property QtObject ctl: QtObject {
        function termJson(topic) { return JSON.stringify({ id: topic, title: topic, text: topic }) }
    }
    property string opened: ""
    function glossary(id) { opened = id; before.forceActiveFocus(Qt.OtherFocusReason) }
    function applicationWindow() { return root }

    Column {
        width: parent.width

        KanteButton { id: before; text: "before" }
        KanteListRow {
            id: first
            width: parent.width
            text: "first"
            // As in the exercise list.
            trailing: Row {
                TermTip { id: tip; topic: "beat" }
            }
        }
        KanteListRow { id: second; width: parent.width; text: "second" }
    }

    TestCase {
        name: "TermTip"
        when: windowShown

        function focused() { return root.Window.activeFocusItem }

        function test_row_reachable_after_tip() {
            before.forceActiveFocus(Qt.TabFocusReason)

            keyClick(Qt.Key_Tab)
            verify(focused() === first, "row first")
            keyClick(Qt.Key_Tab)
            verify(focused() === tip, "the ?")
            keyClick(Qt.Key_Tab)
            verify(focused() === second, "row second")

            // Back: the "?", then its row; the next round still stops at the row, not at the "?".
            keyClick(Qt.Key_Backtab)
            verify(focused() === tip, "the ? again")
            keyClick(Qt.Key_Backtab)
            verify(focused() === first, "row first by Shift+Tab, focus on " + focused())
            before.forceActiveFocus(Qt.TabFocusReason)
            keyClick(Qt.Key_Tab)
            verify(focused() === first, "row first again, focus on " + focused())
        }

        // The glossary takes the focus away; afterwards Tab still reaches the row.
        function test_row_reachable_after_glossary() {
            tip.forceActiveFocus(Qt.TabFocusReason)
            tip.clicked()
            compare(root.opened, "beat")

            before.forceActiveFocus(Qt.TabFocusReason)
            keyClick(Qt.Key_Tab)
            verify(focused() === first, "row first, focus on " + focused())
        }
    }
}
