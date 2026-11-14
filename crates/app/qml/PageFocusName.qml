import QtQuick
import org.kde.kirigami as Kirigami

// Names the scroll area of its page with the page title. After a page switch the screen reader
// lands on that area, and unnamed it said nothing useful (QA 28.10., B8, P7). Main.qml adds one
// to every page it shows.
//
//   page (title "Lessons")
//     └─ flickable ── Accessible.name = "Lessons"
Item {
    id: root
    visible: false

    required property Kirigami.Page page

    Binding {
        when: root.page !== null && root.page.flickable !== null
        target: root.page ? root.page.flickable.Accessible : null
        property: "name"
        value: root.page ? root.page.title : ""
    }
}
