import QtQuick
import QtQuick.Templates as T

// A drawer's separators silent for screen readers: Kirigami makes each one an unnamed list
// entry (QA 28.10., P7).
QtObject {
    id: lines

    required property T.Popup drawer

    // Kirigami's entry delegates below `item`: buttons that carry their action as tAction.
    function silence(item) {
        if (!item) {
            return
        }
        if (item.tAction && item instanceof T.AbstractButton) {
            if (item.tAction.separator) {
                item.Accessible.ignored = true
            }
            return
        }
        for (var i = 0; i < item.children.length; i++) {
            silence(item.children[i])
        }
    }

    // The delegates come after the drawer; again on each opening, the list may have changed.
    Component.onCompleted: Qt.callLater(silence, drawer.contentItem)
    readonly property Connections opening: Connections {
        target: lines.drawer
        function onAboutToShow() { lines.silence(lines.drawer.contentItem) }
    }
}
