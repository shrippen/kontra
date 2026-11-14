import QtQuick
import org.kde.kirigami as Kirigami

// Names the content item of its page with the page title. After a page switch the focus is
// there (the scroll view of a ScrollablePage), and unnamed the screen reader said nothing
// useful (QA 28.10., B8, P7). Main.qml adds one to every page it shows.
//
//   page (title "Lessons")
//     └─ contentItem (has the focus) ── Accessible.name = "Lessons"
Item {
    id: root
    visible: false

    required property Kirigami.Page page

    Binding {
        when: root.page.contentItem !== null
        target: root.page.contentItem ? root.page.contentItem.Accessible : null
        property: "name"
        value: root.page.title
    }
}
