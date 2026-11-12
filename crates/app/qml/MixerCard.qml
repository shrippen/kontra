import QtQuick
import QtQuick.Layouts
import QtQuick.Controls as QQC2
import QtQuick.Window
import Kante

// The mixer card of the practice page: levels and tone shaping above the transport.
KanteCard {
    id: mixerCard

    property bool open: false
    // At most this high, the rest scrolls: open with tone shaping it squeezed the notes to a
    // strip (QA 14.10.).
    property real maxHeight: 0
    readonly property real fullHeight: Math.min(mixerLoader.implicitHeight + KanteStyle.unit(16), maxHeight)
    // It grows to its height and shrinks away instead of popping in.
    property real openness: open ? 1 : 0
    Behavior on openness { NumberAnimation { duration: KanteStyle.duration; easing.type: Easing.OutCubic } }
    Layout.preferredHeight: fullHeight * openness
    visible: openness > 0
    clip: true
    barColor: KanteStyle.accentColor
    borderColor: KanteStyle.frameColor
    Flickable {
        id: mixerFlick
        x: KanteStyle.unit(12)
        y: KanteStyle.unit(8)
        width: parent.width - KanteStyle.unit(24)
        height: mixerCard.fullHeight - KanteStyle.unit(16)
        contentWidth: width
        contentHeight: mixerLoader.implicitHeight
        clip: true
        boundsBehavior: Flickable.StopAtBounds
        QQC2.ScrollBar.vertical: QQC2.ScrollBar { id: mixerBar; policy: mixerFlick.contentHeight > mixerFlick.height ? QQC2.ScrollBar.AlwaysOn : QQC2.ScrollBar.AsNeeded }
        // Tab into a row below the fold scrolls the whole row into view (P7, keyboard only).
        Window.onActiveFocusItemChanged: {
            let item = Window.activeFocusItem
            while (item && item.parent !== mixerLoader.item) {
                item = item.parent
            }
            if (!item) {
                return
            }
            const top = item.y
            if (top < contentY) {
                contentY = top
            } else if (top + item.height > contentY + height) {
                contentY = Math.min(top + item.height - height, contentHeight - height)
            }
        }
        // Built after the page unless it opens at once: the mixer is many controls.
        Loader {
            id: mixerLoader
            asynchronous: !mixerCard.open
            width: mixerFlick.width - (mixerFlick.contentHeight > mixerFlick.height ? mixerBar.width + KanteStyle.unit(8) : 0)
            sourceComponent: Mixer { practice: true }
        }
    }
}
