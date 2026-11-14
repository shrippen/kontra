import QtQuick
import QtTest
import org.kde.kirigami as Kirigami
import ".."

// After a page switch the focus is on the page's content item, the scroll view of a
// ScrollablePage (QA 28.10., B8, P7): it must be announced with the page title, not as an
// unnamed area. Run by scripts/check-video.sh.
Item {
    id: root
    width: 640
    height: 480

    Kirigami.PageRow {
        id: row
        anchors.fill: parent
    }

    Component {
        id: pageComponent
        Kirigami.ScrollablePage {
            title: "Lessons"
            Column {
                Repeater { model: 40; Text { text: "row " + index } }
            }
        }
    }
    Component { id: nameComponent; PageFocusName {} }

    TestCase {
        name: "PageFocusName"
        when: windowShown

        // As Main.qml shows a page: created, named, pushed; the page row focuses it.
        function shown() {
            var page = createTemporaryObject(pageComponent, row)
            nameComponent.createObject(page, { page: page })
            row.clear()
            row.push(page)
            page.forceActiveFocus()
            return page
        }

        function test_focusedAreaHasTheTitle() {
            shown()
            var focused = root.Window.activeFocusItem
            verify(focused !== null)
            compare(focused.Accessible.name, "Lessons")
        }

        function test_nameFollowsTheTitle() {
            var page = shown()
            page.title = "Glossary"
            compare(root.Window.activeFocusItem.Accessible.name, "Glossary")
        }
    }
}
