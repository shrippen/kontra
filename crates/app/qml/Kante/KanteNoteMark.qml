import QtQuick
import QtQuick.Shapes
import "."

/**
 * The sign of a played note's state (`noteState`, Kante 1.23), so colour is never the only
 * sign: "hit" a check, "wrong" a cross, "missed" a hollow dashed square, "early" an
 * arrow to the left (before the beat, as on a time axis), "late" an arrow to the right;
 * "pending" (or anything else) draws nothing. Colour from KanteStyle.noteStateColor.
 * `badge` puts the sign on a small dialog-coloured square with a frame, so it reads on
 * top of a filled note. Used by KanteTabLane, KanteTabStaff and KanteBassStaff, and as
 * the key of a legend.
 */
Item {
    id: mark

    property string noteState: "pending"
    property color color: KanteStyle.noteStateColor(noteState)
    property bool badge: false
    readonly property bool drawn: noteState === "hit" || noteState === "wrong" || noteState === "missed"
                                  || noteState === "early" || noteState === "late"

    implicitWidth: KanteStyle.unit(14)
    implicitHeight: implicitWidth

    Accessible.role: Accessible.Graphic
    Accessible.name: KanteStyle.noteStateName(noteState)

    readonly property real s: Math.min(width, height)
    readonly property real stroke: Math.max(1.5, s * (badge ? 0.13 : 0.15))

    /** The path of a sign in a square of side `s` (SVG syntax). */
    function pathFor(st, s) {
        function p(x, y) { return (x * s).toFixed(2) + " " + (y * s).toFixed(2) }
        switch (st) {
        case "hit": return "M " + p(0.2, 0.52) + " L " + p(0.42, 0.74) + " L " + p(0.82, 0.28)
        case "wrong": return "M " + p(0.26, 0.26) + " L " + p(0.74, 0.74) + " M " + p(0.74, 0.26) + " L " + p(0.26, 0.74)
        case "early": return "M " + p(0.8, 0.5) + " L " + p(0.22, 0.5) + " M " + p(0.46, 0.26) + " L " + p(0.22, 0.5) + " L " + p(0.46, 0.74)
        case "late": return "M " + p(0.2, 0.5) + " L " + p(0.78, 0.5) + " M " + p(0.54, 0.26) + " L " + p(0.78, 0.5) + " L " + p(0.54, 0.74)
        case "missed": return "M " + p(0.22, 0.22) + " L " + p(0.78, 0.22) + " L " + p(0.78, 0.78) + " L " + p(0.22, 0.78) + " Z"
        default: return ""
        }
    }

    Rectangle {
        visible: mark.badge && mark.drawn
        anchors.centerIn: parent
        width: mark.s
        height: mark.s
        color: KanteStyle.dialogColor
        border.width: 1
        border.color: mark.color
    }

    Shape {
        visible: mark.drawn
        width: mark.s
        height: mark.s
        anchors.centerIn: parent
        antialiasing: true
        ShapePath {
            strokeColor: mark.color
            strokeWidth: mark.stroke
            fillColor: "transparent"
            capStyle: ShapePath.SquareCap
            joinStyle: ShapePath.MiterJoin
            strokeStyle: mark.noteState === "missed" ? ShapePath.DashLine : ShapePath.SolidLine
            dashPattern: [1.6, 1.4]
            PathSvg { path: mark.pathFor(mark.noteState, mark.s) }
        }
    }
}
