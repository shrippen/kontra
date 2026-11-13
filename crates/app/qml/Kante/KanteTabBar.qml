import QtQuick
import QtQuick.Layouts
import org.kde.kirigami as Kirigami
import "."

/**
 * Tabs as notched tabs (top-right cut). The yellow tab slides to the selected
 * one. `model` is a list of labels (it may change at run time), `icons` an optional
 * list of icon names (empty label = icon only), `counts` an optional list of counters
 * and `countKinds` their kinds (KanteCounter.Kind: 0 normal, 1 info, 2 error, e.g. overdue).
 * With `badges` the counters are filled badges instead of mono text. When the tabs do not
 * fit they scroll sideways and the selected tab is kept in view.
 * Left/Right move the selection while the bar has focus; by default (`activation` Automatic)
 * they activate it at once, like a tap. With `activation: KanteTabBar.Activation.Manual` the
 * arrows only move the selection (nothing opens, the bar keeps focus); Return, Enter, Space or
 * a tap activate it (KanteStickyHeader's tabs: the page should not jump on every arrow).
 * `look` (Underline): no notched tabs, only text and a 2 px accent line under the selected one,
 * for a bar in a page header (KanteStickyHeader); the line and the accent text colour carry
 * the selection, not colour alone.
 *
 *   Notched:    [ Alle ]  Prüfen   Fertig         Underline:  Alle   Prüfen   Fertig
 *   ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━                      ▔▔▔▔▔
 */
FocusScope {
    id: tabs

    enum Look {
        Notched,
        Underline
    }

    enum Activation {
        /** The arrows activate at once, like a tap (the default, every earlier app relies on it). */
        Automatic,
        /** The arrows only move the selection; Return, Enter, Space or a tap activate it. */
        Manual
    }

    property int look: KanteTabBar.Look.Notched
    property int activation: KanteTabBar.Activation.Automatic
    /** Screen readers hear this as the tab list's name. */
    property string accessibleName: ""
    property var model: []
    property var icons: []
    property var counts: []
    property var countKinds: []
    property bool badges: false
    property int currentIndex: 0
    signal activated(int index)
    /** Manual arrows only (`move`): the selection changed without activating it. */
    signal moved(int index)

    implicitHeight: KanteStyle.heightMedium
    implicitWidth: row.implicitWidth
    activeFocusOnTab: true
    Accessible.role: Accessible.PageTabList
    Accessible.name: accessibleName

    // Automatic (default): the arrows select and activate at once, like a tap. Manual: they
    // only move the selection (the bar keeps focus, nothing opens or scrolls); Return, Enter
    // and Space activate the selected tab then, as a tap does.
    Keys.onLeftPressed: moveOrSelect(Math.max(0, currentIndex - 1))
    Keys.onRightPressed: moveOrSelect(Math.min(model.length - 1, currentIndex + 1))
    Keys.onReturnPressed: activate(currentIndex)
    Keys.onEnterPressed: activate(currentIndex)
    Keys.onSpacePressed: activate(currentIndex)

    function moveOrSelect(i) {
        if (activation === KanteTabBar.Activation.Manual) {
            move(i)
        } else {
            select(i)
        }
    }

    /** Moves the selection without activating it (Manual arrows): no `activated`, nothing opens. */
    function move(i) {
        currentIndex = i
        moved(i)
    }

    /** Selects and activates, as a tap does: fires `activated` only when `i` changes the
        selection (tapping the already-active tab twice does nothing the second time). */
    function select(i) {
        if (i !== currentIndex) {
            currentIndex = i
            activated(i)
        }
    }

    /** Activates `i` unconditionally (Return, Enter, Space): the key press itself is the
        request to act, even when the arrows already left the selection there. */
    function activate(i) {
        currentIndex = i
        activated(i)
    }

    function ensureVisible() {
        var t = rep.count > 0 ? rep.itemAt(currentIndex) : null
        if (!t) {
            return
        }
        if (t.x < flick.contentX) {
            flick.contentX = t.x
        } else if (t.x + t.width > flick.contentX + flick.width) {
            flick.contentX = t.x + t.width - flick.width
        }
    }
    onCurrentIndexChanged: ensureVisible()

    Rectangle {
        anchors.bottom: parent.bottom
        width: parent.width
        height: 2
        color: KanteStyle.frameColor
    }

    Flickable {
        id: flick
        anchors.fill: parent
        contentWidth: row.implicitWidth
        contentHeight: height
        flickableDirection: Flickable.HorizontalFlick
        boundsBehavior: Flickable.StopAtBounds
        interactive: contentWidth > width
        clip: true
        Behavior on contentX { NumberAnimation { duration: KanteStyle.duration; easing.type: Easing.OutCubic } }

        // The sliding yellow tab.
        KantePolygon {
            id: slider
            // rep.count makes the binding re-run once the tabs exist.
            readonly property Item target: rep.count > 0 ? rep.itemAt(tabs.currentIndex) : null
            x: target ? target.x : 0
            width: target ? target.width : 0
            height: tabs.height
            fillColor: KanteStyle.accentColor
            visible: tabs.look === KanteTabBar.Look.Notched
            cutTopRight: KanteStyle.chamferSmall
            Behavior on x { NumberAnimation { duration: KanteStyle.animate ? 260 : 0; easing.type: Easing.OutCubic } }
            Behavior on width { NumberAnimation { duration: KanteStyle.animate ? 260 : 0; easing.type: Easing.OutCubic } }
        }

        Row {
            id: row
            spacing: 2

            Repeater {
                id: rep
                model: tabs.model
                delegate: Item {
                    id: tab
                    required property int index
                    required property var modelData
                    readonly property bool current: tabs.currentIndex === index
                    readonly property string iconName: tabs.icons && tabs.icons.length > index ? String(tabs.icons[index]) : ""
                    readonly property string countText: tabs.counts && tabs.counts.length > index ? String(tabs.counts[index]) : ""
                    readonly property color ink: current ? (tabs.look === KanteTabBar.Look.Underline ? KanteStyle.accentTextColor : KanteStyle.accentForegroundColor) : (hover.hovered ? KanteStyle.strongTextColor : KanteStyle.mutedTextColor)
                    width: content.implicitWidth + KanteStyle.unit(iconName.length > 0 && String(modelData).length === 0 ? 24 : tabs.look === KanteTabBar.Look.Underline ? 24 : 36)
                    height: tabs.height

                    KantePolygon {
                        anchors.fill: parent
                        fillColor: hover.hovered ? KanteStyle.sunkenColor : KanteStyle.cardColor
                        cutTopRight: KanteStyle.chamferSmall
                        // Shapes entirely outside the flickable are still painted by the software
                        // renderer (it ignores the clip for them): draw only tabs in view.
                        visible: tabs.look === KanteTabBar.Look.Notched && !tab.current && tab.x + tab.width > flick.contentX && tab.x < flick.contentX + flick.width
                    }

                    RowLayout {
                        id: content
                        anchors.centerIn: parent
                        spacing: KanteStyle.unit(8)
                        KanteIcon {
                            visible: tab.iconName.length > 0
                            Layout.preferredWidth: Kirigami.Units.iconSizes.small
                            Layout.preferredHeight: Kirigami.Units.iconSizes.small
                            iconSource: tab.iconName
                            color: tab.ink
                        }
                        Text {
                            visible: String(tab.modelData).length > 0
                            text: tab.modelData
                            font: KanteStyle.labelFont()
                            color: tab.ink
                            Behavior on color { ColorAnimation { duration: KanteStyle.duration } }
                        }
                        Text {
                            visible: tab.countText.length > 0 && !tabs.badges
                            text: tab.countText
                            font: KanteStyle.monoFont(KanteStyle.labelFont().pointSize, false)
                            color: tab.ink
                            opacity: 0.7
                        }
                        KanteCounter {
                            visible: tab.countText.length > 0 && tabs.badges
                            text: tab.countText
                            kind: tabs.countKinds && tabs.countKinds.length > tab.index ? tabs.countKinds[tab.index] : KanteCounter.Kind.Normal
                        }
                    }

                    HoverHandler { id: hover }
                    TapHandler { onTapped: tabs.select(tab.index) }
                    Accessible.role: Accessible.PageTab
                    Accessible.name: String(tab.modelData).length > 0 ? tab.modelData : tab.iconName
                    Accessible.selected: tab.current
                }
            }
        }

        // Underline look: the selected tab's accent line, over the bar's rule.
        Rectangle {
            visible: tabs.look === KanteTabBar.Look.Underline && slider.width > 0
            x: slider.x
            width: slider.width
            y: tabs.height - height
            height: KanteStyle.unit(2)
            color: KanteStyle.accentColor
        }

        // Focus: a 3 px cyan line under the selected tab.
        Rectangle {
            visible: tabs.activeFocus
            x: slider.x
            width: slider.width
            y: tabs.height - 3
            height: 3
            color: KanteStyle.focusColor
        }
    }
}
