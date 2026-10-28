import QtQuick
import QtQuick.Templates as T
import Kante

// Kante's focus brackets on the focused entry of a drawer. The desktop style drew the focus like
// hover, in the selection colour: invisible on the current page's entry (QA 27.10., S3, P7).
KanteBrackets {
    id: brackets

    required property T.Popup drawer

    readonly property Item content: drawer.contentItem
    // The item with the keyboard focus, when it lies in the drawer.
    readonly property Item entry: inDrawer(content ? content.Window.activeFocusItem : null)

    function inDrawer(item) {
        for (var p = item; p; p = p.parent) {
            if (p === content) {
                return item
            }
        }
        return null
    }

    parent: entry
    anchors.fill: parent
    z: 1
    active: entry !== null
}
