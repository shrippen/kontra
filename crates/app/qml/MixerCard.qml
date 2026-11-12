import QtQuick
import QtQuick.Controls as QQC2
import QtQuick.Window
import Kante

// The mixer of the practice page: levels and tone shaping on a glass layer that slides down
// over the notes (owner 12.11.). As a row of the page it pushed the notes down and shrank
// them on every frame of its slide, a relayout each.
KanteGlassPanel {
    id: mixerCard

    // At most this high, the rest scrolls: tone shaping is tall, and the strike line below stays
    // in sight (QA 14.10.: as a row it squeezed the notes to a strip).
    property real maxHeight: 0

    edge: KanteGlassPanel.Edge.Top
    panelSize: Math.min(mixerLoader.implicitHeight + 2 * padding, maxHeight)
    barColor: KanteStyle.accentColor
    accessibleName: applicationWindow().texts.t("mixer.title")

    Flickable {
        id: mixerFlick
        anchors.fill: parent
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
