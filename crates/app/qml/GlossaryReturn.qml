import QtQuick
import org.kde.kirigami as Kirigami

// Where the player was when a glossary entry opened as a layer over a page (J-G2): the page
// loses its scroll position under the layer; it is put back when the last layer closes, and the
// control that opened it ("?" or link) gets the focus back (QA 28.10. B5, P7: it was gone).
QtObject {
    id: back

    required property Kirigami.PageRow pageStack
    property real contentY: -1
    property Item opener: null
    property int reason: Qt.OtherFocusReason

    // Before the first layer opens.
    function save() {
        if (pageStack.layers.depth !== 1) {
            return
        }
        var under = pageStack.currentItem
        if (under && under.flickable) {
            contentY = under.flickable.contentY
        }
        opener = pageStack.Window.activeFocusItem
        reason = opener ? opener.focusReason : Qt.OtherFocusReason
    }

    // Another page: nothing to put back.
    function forget() {
        contentY = -1
        opener = null
    }

    readonly property Connections layers: Connections {
        target: back.pageStack.layers
        function onDepthChanged() {
            if (back.pageStack.layers.depth > 1) {
                return
            }
            var under = back.pageStack.currentItem
            var y = back.contentY
            var opener = back.opener
            var reason = back.reason
            back.forget()
            // Scroll first, then focus: the opener lies in view at the old position.
            Qt.callLater(function() {
                if (y >= 0 && under && under.flickable) {
                    under.flickable.contentY = y
                }
                if (opener) {
                    opener.forceActiveFocus(reason)
                }
            })
        }
    }
}
