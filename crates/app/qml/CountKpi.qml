import QtQuick
import Kante

// A figure like KanteKpi whose leading whole number counts up on KanteOdometer when it shows:
// "87 % hits" rolls to 87, "12 min" to 12. A value without a leading number ("< 1 min") shows
// as text. One name for screen readers, as in KanteKpi: "Practised: 12 min".
Column {
    id: kpi

    property string value: ""
    property string label: ""
    // The whole figure as text, as one Text showed it before (tests, tools).
    readonly property string text: value
    // Set one tick after creation, so the reels exist at 0 and roll to the value.
    property bool armed: false
    readonly property var parts: /^(\d+)(.*)$/.exec(value)
    readonly property int number: parts ? parseInt(parts[1]) : 0
    readonly property string rest: parts ? parts[2] : value
    readonly property font figureFont: KanteStyle.headingFont(KanteStyle.defaultFont.pointSize * 2.6)

    spacing: KanteStyle.unit(2)
    Component.onCompleted: Qt.callLater(function() { kpi.armed = true })

    Accessible.role: Accessible.StaticText
    Accessible.name: label.length > 0 ? label + ": " + value : value

    Row {
        Accessible.ignored: true
        KanteOdometer {
            id: odometer
            visible: kpi.parts !== null
            digits: String(kpi.number).length
            font: kpi.figureFont
            // Hidden pages and the first tick show 0; the roll starts when the figure shows.
            value: kpi.armed && kpi.visible ? kpi.number : 0
        }
        Text {
            text: kpi.rest
            color: KanteStyle.strongTextColor
            font: kpi.figureFont
        }
    }
    Text {
        Accessible.ignored: true
        text: kpi.label
        color: KanteStyle.mutedTextColor
        font: KanteStyle.labelFont()
    }
}
