import QtQuick
import QtQuick.Shapes
import "."

/**
 * Note highway (Kante 1.23, for Kontra): one lane per string, the notes run from the
 * right onto the play line. Purely presentational: the app moves `position` (seconds)
 * and sets the note states; nothing here animates on its own.
 *
 *   strings, stringNames      lanes; string 0 is the lowest. `lowStringOnTop` false
 *                             (default) puts the highest string on top, as in a tab
 *   stringColors              optional colour per string (0 = lowest); the string
 *                             palette below where missing
 *   notes                     [{time, duration, string, fret, state, label}], sorted by
 *                             time; seconds. state: "pending", "hit", "wrong", "missed",
 *                             "early", "late". Reassign the list to show a new state
 *   bars                      bar start times in seconds (numbers or {time})
 *   position                  the current time in seconds (sits on the play line)
 *   pixelsPerSecond           speed of the highway
 *   playLine                  where the play line is, 0..1 of the width from the left
 *   loopStart, loopEnd        loop range in seconds, -1 for none
 *
 * A note is a square head with the fret number at its onset and a tail as long as it
 * lasts. The state shows by shape and colour: pending filled in the string colour,
 * hit filled + check, wrong filled + cross, missed hollow and dashed, early / late
 * filled + arrow (left: before the beat, right: after it). KanteNoteMark draws the signs.
 *
 * Speed (chosen over a Canvas, which would repaint every note on every frame): the
 * notes are items on one strip at x = time × pixelsPerSecond, and only the strip moves
 * with `position` (one binding per frame, no per-note work). Each note has a light
 * placeholder (about 0.05 ms per note at load); its visuals are created asynchronously
 * when it comes within a view and a half of the play line and dropped behind it. The
 * window moves in steps of half a view, so the per-note bindings run about twice per
 * view width, not per frame. Measured with 3000 notes, about 300 in view: under 2 ms of
 * binding work per frame; 65-70 fps on llvmpipe (CPU rendering), a GPU is not the limit.
 */
Item {
    id: lane

    property int strings: 4
    property var stringNames: ["E", "A", "D", "G"]
    property bool lowStringOnTop: false
    property var stringColors: []
    property var notes: []
    property var bars: []
    property real position: 0
    property real pixelsPerSecond: KanteStyle.unit(160)
    property real playLine: 0.2
    property real loopStart: -1
    property real loopEnd: -1
    /** Bar numbers above the lanes. */
    property bool barNumbers: true
    /** Name for screen readers. */
    property string accessibleName: "Tab-Lane"

    /** Index of the next note to play (first with time ≥ position), -1 after the last. */
    readonly property int nextIndex: firstAtOrAfter(position)

    // ── Geometry ─────────────────────────────────────────────────────
    readonly property real topPad: barNumbers ? Math.round(labelMetrics.height + KanteStyle.unit(4)) : KanteStyle.unit(2)
    readonly property real gutter: Math.round(nameMetrics.width + KanteStyle.unit(24))
    readonly property real plotWidth: Math.max(1, width - gutter)
    readonly property real laneHeight: (height - topPad) / Math.max(1, strings)
    readonly property real head: Math.max(KanteStyle.unit(12), Math.round(laneHeight * 0.72))
    readonly property real playX: Math.round(plotWidth * Math.max(0, Math.min(1, playLine)))
    readonly property real pps: Math.max(1, pixelsPerSecond)
    // Seconds before and after the play line that are in view (plus a head).
    readonly property real viewBefore: (playX + head) / pps
    readonly property real viewAfter: (plotWidth - playX + head) / pps
    readonly property real chunk: Math.max(0.25, plotWidth / pps / 2)
    readonly property real chunkStart: Math.floor(position / chunk) * chunk
    readonly property real windowFrom: chunkStart - viewBefore
    readonly property real windowTo: chunkStart + chunk + viewAfter

    implicitWidth: KanteStyle.unit(640)
    implicitHeight: topPad + Math.max(1, strings) * KanteStyle.unit(38)

    Accessible.role: Accessible.Chart
    Accessible.name: accessibleName
    Accessible.description: summary(nextIndex, notes)

    function firstAtOrAfter(t) {
        var a = 0, b = notes ? notes.length : 0
        while (a < b) {
            var m = (a + b) >> 1
            if (Number(notes[m].time) < t) {
                a = m + 1
            } else {
                b = m
            }
        }
        return notes && a < notes.length ? a : -1
    }
    function nameOf(s) {
        return stringNames && s < stringNames.length ? String(stringNames[s]) : String(s + 1)
    }
    /** Next note and the count per state, for screen readers. */
    function summary(next, list) {
        var counts = {}
        for (var i = 0; list && i < list.length; i++) {
            var st = list[i].state || "pending"
            counts[st] = (counts[st] || 0) + 1
        }
        var parts = []
        if (next >= 0) {
            var n = list[next]
            parts.push(KanteStyle.noteStateName("pending") + ": " + nameOf(n.string) + " " + n.fret)
        }
        ["hit", "wrong", "missed", "early", "late"].forEach(function (st) {
            if (counts[st]) {
                parts.push(KanteStyle.noteStateName(st) + " " + counts[st])
            }
        })
        return parts.join(", ")
    }
    /** The string palette: kept apart from the state colours (hit, wrong, early/late). */
    function stringColor(s) {
        if (stringColors && s < stringColors.length && stringColors[s] !== undefined) {
            return stringColors[s]
        }
        var p = [KanteStyle.tagColor, KanteStyle.focusColor, KanteStyle.accentTextColor,
                 KanteStyle.strongTextColor, KanteStyle.mutedTextColor, KanteStyle.infoColor]
        return p[s % p.length]
    }
    function rowOf(s) {
        return lowStringOnTop ? s : strings - 1 - s
    }
    function rowCenter(s) {
        return topPad + (rowOf(s) + 0.5) * laneHeight
    }
    function barTime(b) {
        return typeof b === "object" && b !== null ? Number(b.time) : Number(b)
    }

    TextMetrics {
        id: nameMetrics
        font: KanteStyle.monoFont(KanteStyle.defaultFont.pointSize, true)
        text: {
            var w = "M"
            for (var s = 0; s < lane.strings; s++) {
                if (lane.nameOf(s).length > w.length) {
                    w = lane.nameOf(s)
                }
            }
            return w
        }
    }
    TextMetrics {
        id: labelMetrics
        font: KanteStyle.monoFont(KanteStyle.labelFont().pointSize * 0.85, false)
        text: "128"
    }

    // Lanes: name, colour key and the string line.
    Repeater {
        model: lane.strings
        delegate: Item {
            required property int index
            readonly property color tone: lane.stringColor(index)
            y: lane.rowCenter(index) - lane.laneHeight / 2
            width: lane.width
            height: lane.laneHeight
            Rectangle {
                visible: lane.rowOf(index) % 2 === 1
                x: lane.gutter
                width: lane.plotWidth
                height: parent.height
                color: KanteStyle.tint1Color
                opacity: 0.5
            }
            Rectangle {
                x: KanteStyle.unit(4)
                anchors.verticalCenter: parent.verticalCenter
                width: KanteStyle.unit(4)
                height: Math.min(parent.height * 0.6, KanteStyle.unit(16))
                color: parent.tone
            }
            Text {
                x: KanteStyle.unit(12)
                anchors.verticalCenter: parent.verticalCenter
                text: lane.nameOf(parent.index)
                color: KanteStyle.strongTextColor
                font: nameMetrics.font
            }
            Rectangle {
                x: lane.gutter
                width: lane.plotWidth
                anchors.verticalCenter: parent.verticalCenter
                height: Math.max(1, Math.round(KanteStyle.unit(1) + (lane.strings - 1 - parent.index) * KanteStyle.unit(1) / 2))
                color: KanteStyle.tint(parent.tone, 0.45)
            }
        }
    }

    // The highway: one strip that moves with `position`.
    Item {
        id: plot
        x: lane.gutter
        width: lane.plotWidth
        height: lane.height
        clip: true

        Item {
            id: strip
            x: lane.playX - lane.position * lane.pps
            height: plot.height

            // Loop range: a tint with bracket edges ([ and ]), not colour alone.
            Item {
                visible: lane.loopStart >= 0 && lane.loopEnd > lane.loopStart
                x: lane.loopStart * lane.pps
                width: (lane.loopEnd - lane.loopStart) * lane.pps
                y: lane.topPad
                height: plot.height - lane.topPad
                Rectangle { anchors.fill: parent; color: KanteStyle.tintHighlightColor }
                Repeater {
                    model: 2
                    delegate: Item {
                        required property int index
                        x: index === 0 ? 0 : parent.width - width
                        width: KanteStyle.unit(8)
                        height: parent.height
                        Rectangle { x: parent.index === 0 ? 0 : parent.width - width; width: 2; height: parent.height; color: KanteStyle.focusColor }
                        Rectangle { width: parent.width; height: 2; color: KanteStyle.focusColor }
                        Rectangle { y: parent.height - 2; width: parent.width; height: 2; color: KanteStyle.focusColor }
                    }
                }
            }

            Repeater {
                model: lane.bars ? lane.bars.length : 0
                delegate: Item {
                    required property int index
                    readonly property real t: lane.barTime(lane.bars[index])
                    visible: t >= lane.windowFrom && t <= lane.windowTo
                    x: Math.round(t * lane.pps)
                    height: plot.height
                    Rectangle { y: lane.topPad; width: 1; height: plot.height - lane.topPad; color: KanteStyle.frameColor }
                    Text {
                        visible: lane.barNumbers
                        x: KanteStyle.unit(3)
                        text: parent.index + 1
                        color: KanteStyle.mutedTextColor
                        font: labelMetrics.font
                    }
                }
            }

            Repeater {
                model: lane.notes ? lane.notes.length : 0
                delegate: Item {
                    id: note
                    required property int index
                    readonly property var n: lane.notes[index] || ({})
                    readonly property real t: Number(n.time) || 0
                    readonly property real d: Math.max(0, Number(n.duration) || 0)
                    readonly property int str: Math.max(0, Math.min(lane.strings - 1, Number(n.string) || 0))
                    readonly property string st: n.state ? String(n.state) : "pending"
                    readonly property bool hollow: st === "missed"
                    readonly property color fill: st === "pending" ? lane.stringColor(str) : KanteStyle.noteStateColor(st)
                    readonly property bool near: t + d >= lane.windowFrom - lane.chunk && t <= lane.windowTo + lane.chunk
                    visible: t + d >= lane.windowFrom && t <= lane.windowTo
                    x: t * lane.pps
                    y: lane.rowCenter(str)

                    Accessible.ignored: true

                    // The note itself is made only near the window, a few frames ahead
                    // (asynchronous), so a long song costs little at load and nothing per frame.
                    Loader {
                        active: note.near
                        asynchronous: true
                        sourceComponent: Component {
                            Item {
                                // Tail: as long as the note lasts.
                                Rectangle {
                                    visible: note.d * lane.pps > lane.head / 2
                                    y: -height / 2
                                    width: note.d * lane.pps
                                    height: Math.round(lane.head * 0.3)
                                    color: KanteStyle.tint(note.fill, note.hollow ? 0.2 : 0.5)
                                }
                                Rectangle {
                                    id: headBox
                                    x: -lane.head / 2
                                    y: -lane.head / 2
                                    width: lane.head
                                    height: lane.head
                                    color: note.hollow ? KanteStyle.backgroundColor : note.fill
                                }
                                Loader {
                                    active: note.hollow
                                    anchors.fill: headBox
                                    sourceComponent: Shape {
                                        antialiasing: true
                                        ShapePath {
                                            id: dash
                                            readonly property real w: Math.max(2, Math.round(lane.head * 0.08))
                                            readonly property real a: w / 2
                                            readonly property real b: lane.head - w / 2
                                            strokeColor: note.fill
                                            strokeWidth: w
                                            fillColor: "transparent"
                                            strokeStyle: ShapePath.DashLine
                                            dashPattern: [2, 1.5]
                                            joinStyle: ShapePath.MiterJoin
                                            PathSvg { path: "M " + dash.a + " " + dash.a + " H " + dash.b + " V " + dash.b + " H " + dash.a + " Z" }
                                        }
                                    }
                                }
                                Text {
                                    anchors.centerIn: headBox
                                    text: note.n.fret !== undefined ? note.n.fret : ""
                                    color: note.hollow ? KanteStyle.textColor : KanteStyle.inkOn(note.fill)
                                    font.family: nameMetrics.font.family
                                    font.weight: Font.Medium
                                    font.pixelSize: Math.max(8, Math.round(lane.head * 0.56))
                                }
                                Loader {
                                    active: note.st !== "pending" && !note.hollow
                                    x: headBox.x + headBox.width - width * 0.6
                                    y: headBox.y - height * 0.4
                                    width: Math.round(lane.head * 0.55)
                                    height: width
                                    sourceComponent: KanteNoteMark { noteState: note.st; badge: true }
                                }
                                Text {
                                    visible: !!note.n.label
                                    x: headBox.x
                                    y: headBox.y - height
                                    text: note.n.label || ""
                                    color: KanteStyle.mutedTextColor
                                    font: labelMetrics.font
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Play line with a diamond on top.
    Rectangle {
        x: lane.gutter + lane.playX - width / 2
        y: lane.topPad
        width: Math.max(2, KanteStyle.unit(3))
        height: lane.height - lane.topPad
        color: KanteStyle.accentColor
    }
    Rectangle {
        x: lane.gutter + lane.playX - width / 2
        y: lane.topPad - height / 2
        width: KanteStyle.unit(9)
        height: width
        rotation: 45
        color: KanteStyle.accentColor
    }
}
