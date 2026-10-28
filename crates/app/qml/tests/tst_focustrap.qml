import QtQuick
import QtQuick.Controls as QQC2
import QtTest
import ".."

// The run summary keeps the keyboard while it is open (QA 28.10., P7: Tab left it into the
// sidebar). Run by scripts/check-video.sh.
Item {
    id: root
    width: 640
    height: 480

    QQC2.Button { id: outsideBefore; text: "before" }

    Item {
        id: area
        y: 60
        width: 300
        height: 200
        QQC2.Button { id: first; text: "first" }
        QQC2.Button { id: last; y: 60; text: "last" }
    }

    QQC2.Button {
        id: outsideAfter
        y: 300
        text: "after"
        // Each time the focus touched it.
        property int touched: 0
        onActiveFocusChanged: {
            if (activeFocus) {
                touched++
            }
        }
    }

    Loader {
        id: loader
        Component.onCompleted: setSource(Qt.resolvedUrl("../FocusTrap.qml"), { area: area, first: first })
    }

    TestCase {
        name: "FocusTrap"
        when: windowShown

        function test_tab_wraps_to_first() {
            outsideAfter.touched = 0
            last.forceActiveFocus(Qt.TabFocusReason)
            keyClick(Qt.Key_Tab)
            verify(first.activeFocus, "focus on " + root.Window.activeFocusItem)
            // Not over a stop outside (it kept a focus frame in the sidebar).
            compare(outsideAfter.touched, 0)
        }

        function test_backtab_wraps_to_last() {
            first.forceActiveFocus(Qt.TabFocusReason)
            keyClick(Qt.Key_Backtab, Qt.ShiftModifier)
            verify(last.activeFocus, "focus on " + root.Window.activeFocusItem)
        }

        // Focus outside the area moves as before.
        function test_outside_left_alone() {
            outsideAfter.forceActiveFocus(Qt.TabFocusReason)
            keyClick(Qt.Key_Tab)
            verify(outsideBefore.activeFocus, "focus on " + root.Window.activeFocusItem)
        }

        // Closed, the area keeps the focus on a hidden item; Tab leaves it then.
        function test_off_when_hidden() {
            last.forceActiveFocus(Qt.TabFocusReason)
            area.visible = false
            keyClick(Qt.Key_Tab)
            area.visible = true
            verify(!first.activeFocus && !last.activeFocus, "focus on " + root.Window.activeFocusItem)
        }
    }
}
