import QtQuick
import QtTest
import org.kde.kirigami as Kirigami
import ".."

// The scroll area of a page has the focus after a page switch (QA 28.10., B8, P7): it must be
// announced with the page title, not as an unnamed area. Run by scripts/check-video.sh.
Item {
    id: root
    width: 640
    height: 480

    Kirigami.ScrollablePage {
        id: page
        anchors.fill: parent
        title: "Lessons"

        PageFocusName { page: page }

        Column {
            Repeater { model: 40; Text { text: "row " + index } }
        }
    }

    TestCase {
        name: "PageFocusName"
        when: windowShown

        function test_focused_area_has_title() {
            verify(page.flickable !== null)
            page.flickable.forceActiveFocus(Qt.TabFocusReason)
            var focused = root.Window.activeFocusItem
            verify(focused !== null)
            compare(focused.Accessible.name, "Lessons")
        }

        function test_name_follows_title() {
            page.title = "Glossary"
            compare(page.flickable.Accessible.name, "Glossary")
        }
    }
}
