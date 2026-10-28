import QtQuick
import QtQuick.Templates as T
import org.kde.kirigami as Kirigami
import Kante

// Kante's colours on a drawer's ground, and its separators silent for screen readers.
// The entries took Kante's colours, the ground behind them the platform's: on a light system
// white below the entries, "Close Sidebar" at 1.4:1 (QA 28.10., P1). Kirigami gives a disabled
// item the platform's colours whatever its parents set, so disabled entries (separators, "Practise"
// without a piece) drop their own ground and show the drawer's; their line takes Kante's rule.
// Kirigami makes each separator an unnamed list entry (P7). The System style keeps the platform.
Item {
    id: skin

    required property T.Popup drawer

    visible: false

    KanteScope { target: skin.drawer.background }

    // The first item below `item` (itself included) that `test` accepts.
    function find(item, test) {
        if (!item || test(item)) {
            return item
        }
        for (var i = 0; i < item.children.length; i++) {
            var found = find(item.children[i], test)
            if (found) {
                return found
            }
        }
        return null
    }

    // Kirigami's entry delegates below `item`: buttons that carry their action as tAction.
    function dress(item) {
        if (!item) {
            return
        }
        if (!(item.tAction && item instanceof T.AbstractButton)) {
            for (var i = 0; i < item.children.length; i++) {
                dress(item.children[i])
            }
            return
        }
        var entry = item
        if (entry.background) {
            entry.background.visible = Qt.binding(function() { return entry.enabled || !KanteStyle.themed })
        }
        if (!entry.tAction.separator) {
            return
        }
        entry.Accessible.ignored = true
        // Kirigami's separator line (Separator: weight and color); in System its own colour.
        var line = find(entry, function(it) { return it.weight !== undefined && it.color !== undefined })
        if (line) {
            line.color = Qt.binding(function() {
                var theme = line.Kirigami.Theme
                return KanteStyle.themed ? KanteStyle.ruleColor
                     : Kirigami.ColorUtils.linearInterpolation(theme.backgroundColor, theme.textColor, theme.frameContrast)
            })
        }
    }

    // The delegates come after the drawer; again on each opening, the list may have changed.
    Component.onCompleted: Qt.callLater(dress, drawer.contentItem)
    Connections {
        target: skin.drawer
        function onAboutToShow() { skin.dress(skin.drawer.contentItem) }
    }
}
