import QtQuick
import QtQuick.Controls as QQC2
import Kante

// In-place help (LH-ALL-05): an info button next to a technical term. The text shows on
// hover, on keyboard focus and on a click (touch), and is the button's description for
// screen readers.
KanteToolButton {
    id: tip

    property string term: ""
    property string info: ""

    // A question mark, not an icon: it shows with every icon theme.
    display: QQC2.AbstractButton.TextOnly
    text: "?"
    checkable: true
    Accessible.name: applicationWindow().texts.t("help.about", { term: tip.term })
    Accessible.description: tip.info
    QQC2.ToolTip.text: tip.info
    QQC2.ToolTip.visible: hovered || activeFocus || checked
    QQC2.ToolTip.delay: hovered && !checked ? 300 : 0
}
