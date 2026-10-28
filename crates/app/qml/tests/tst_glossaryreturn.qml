import QtQuick
import QtQuick.Layouts
import QtTest
import org.kde.kirigami as Kirigami
import Kante
import ".."

// Back from a glossary entry opened by a "?" in the exercise list (QA 28.10. B5, P7): the focus
// returns to that "?", and Tab and Shift+Tab still reach every row. Run by scripts/check-video.sh.
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
    function applicationWindow() { return root }

    // As Main.qml: remember, then open the entry as a layer.
    function glossary(id) {
        back.save()
        stack.layers.push(entryPage)
    }

    Kirigami.PageRow {
        id: stack
        anchors.fill: parent
        initialPage: Kirigami.ScrollablePage {
            ColumnLayout {
                KanteButton { id: before; text: "before" }
                // As in the exercise list.
                KanteListRow {
                    id: first
                    Layout.fillWidth: true
                    text: "first"
                    trailing: Row {
                        TermTip { id: tip; topic: "beat" }
                    }
                }
                KanteListRow { id: second; Layout.fillWidth: true; text: "second" }
            }
        }
    }

    GlossaryReturn { id: back; pageStack: stack }

    Component {
        id: entryPage
        Kirigami.Page {
            KanteButton { id: backButton; text: "back"; Component.onCompleted: forceActiveFocus(Qt.TabFocusReason) }
        }
    }

    TestCase {
        name: "GlossaryReturn"
        when: windowShown

        function focused() { return root.Window.activeFocusItem }

        function test_focus_back_on_the_tip() {
            before.forceActiveFocus(Qt.TabFocusReason)
            keyClick(Qt.Key_Tab)
            keyClick(Qt.Key_Tab)
            verify(focused() === tip, "the ?, focus on " + focused())

            keyClick(Qt.Key_Space)
            tryCompare(stack.layers, "depth", 2)
            verify(focused() !== tip, "the entry has the focus")

            stack.layers.pop()
            tryVerify(function() { return focused() === tip }, 2000, "back on the ?, focus on " + focused())

            // Every row stays reachable both ways.
            keyClick(Qt.Key_Tab)
            verify(focused() === second, "row second, focus on " + focused())
            keyClick(Qt.Key_Backtab, Qt.ShiftModifier)
            verify(focused() === tip, "the ? again, focus on " + focused())
            keyClick(Qt.Key_Backtab, Qt.ShiftModifier)
            verify(focused() === first, "row first by Shift+Tab, focus on " + focused())
            keyClick(Qt.Key_Backtab, Qt.ShiftModifier)
            verify(focused() === before, "before, focus on " + focused())
            keyClick(Qt.Key_Tab)
            verify(focused() === first, "row first by Tab, focus on " + focused())
        }
    }
}
