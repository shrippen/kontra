import QtQuick
import "."

/**
 * Classic tablature (Kante 1.23, for Kontra): one line per string, fret numbers on the
 * lines, bar lines, repeat signs and simple rhythm below the staff (stems, flags, beams
 * within a beat, dots). It pages line by line: the top line holds the current bar, the
 * `systems - 1` lines below show what comes next, and the lines jump (no scrolling)
 * when the current bar leaves the top line. The current note sits on a selection band;
 * `cursor` adds a thin line at `position`.
 *
 *   strings, stringNames, lowStringOnTop   as KanteTabLane (string 0 = lowest; the
 *                                          highest string on top by default)
 *   notes        [{time, duration, beats, string, fret, state, label}], sorted by time;
 *                time and duration in seconds, beats = duration in quarter notes (1 a
 *                quarter, 0.5 an eighth, 1.5 a dotted quarter). Notes with the same time
 *                form a chord (one stem)
 *   bars         bar start times in seconds, numbers or {time, repeatStart, repeatEnd}
 *   position     the current time in seconds
 *   barsPerSystem  bars per line, 0 = as many as fit (`minBarWidth` each)
 *   systems      lines shown
 *
 * The state of a note shows above the staff as a KanteNoteMark and in the colour of
 * its fret number. The layout is computed once per page (not per frame): only the
 * band and the cursor follow `position`. Rhythm glyphs (flags) come from Bravura
 * (SMuFL, SIL OFL, fonts/OFL-Bravura.txt).
 */
Item {
    id: staff

    property int strings: 4
    property var stringNames: ["E", "A", "D", "G"]
    property bool lowStringOnTop: false
    property var notes: []
    property var bars: []
    property real position: 0
    property int barsPerSystem: 0
    property int systems: 1
    property bool rhythm: true
    property bool cursor: true
    property real lineSpacing: KanteStyle.unit(14)
    property real minBarWidth: KanteStyle.unit(200)
    /** Ground behind the fret numbers (they interrupt the string line). */
    property color groundColor: KanteStyle.backgroundColor
    property string accessibleName: "Tabulatur"
    /** Text of the bar in the screen-reader description; %1 bar, %2 of. */
    property string barText: "Takt %1 von %2"

    /** Last note with time ≤ position, -1 before the first. */
    readonly property int currentIndex: lastAtOrBefore(position)
    readonly property var barList: normalizeBars(bars, notes)
    readonly property int currentBar: barAt(position)
    readonly property real ls: Math.max(6, lineSpacing)
    readonly property int perSystem: barsPerSystem > 0 ? barsPerSystem
        : Math.max(1, Math.floor((width - headerWidth) / Math.max(1, minBarWidth)))
    readonly property int firstBar: Math.max(0, Math.floor(currentBar / perSystem) * perSystem)

    readonly property real markRow: ls * 1.3
    readonly property real staffTop: markRow + ls * 0.5
    readonly property real staffHeight: (Math.max(1, strings) - 1) * ls
    readonly property real systemHeight: staffTop + staffHeight + (rhythm ? ls * 3.2 : ls * 0.8) + ls * 0.4
    readonly property real headerWidth: Math.round(nameMetrics.width + ls * 2.6)
    readonly property real barWidth: (width - headerWidth) / perSystem

    readonly property var layout: computeLayout(firstBar, perSystem, Math.max(1, systems), width, ls, notes, barList,
                                                strings, lowStringOnTop, rhythm)

    implicitWidth: KanteStyle.unit(720)
    implicitHeight: Math.ceil(systemHeight * Math.max(1, systems))

    Accessible.role: Accessible.Chart
    Accessible.name: accessibleName
    Accessible.description: {
        var d = barText.arg(currentBar + 1).arg(barList.length)
        if (currentIndex >= 0) {
            var n = notes[currentIndex]
            d += ", " + nameOf(Number(n.string) || 0) + " " + n.fret + " (" + KanteStyle.noteStateName(n.state || "pending") + ")"
        }
        return d
    }

    FontLoader { id: music; source: Qt.resolvedUrl("fonts/Bravura.otf") }
    TextMetrics {
        id: nameMetrics
        font: KanteStyle.monoFont(KanteStyle.labelFont().pointSize, true)
        text: {
            var w = "M"
            for (var s = 0; s < staff.strings; s++) {
                if (staff.nameOf(s).length > w.length) {
                    w = staff.nameOf(s)
                }
            }
            return w
        }
    }

    function nameOf(s) {
        return stringNames && s < stringNames.length ? String(stringNames[s]) : String(s + 1)
    }
    function lastAtOrBefore(t) {
        var a = 0, b = notes ? notes.length : 0
        while (a < b) {
            var m = (a + b) >> 1
            if (Number(notes[m].time) <= t + 1e-6) {
                a = m + 1
            } else {
                b = m
            }
        }
        return a - 1
    }
    /** Bars as [{time, end, repeatStart, repeatEnd}]; one bar over all notes when none are given. */
    function normalizeBars(list, ns) {
        var out = []
        for (var i = 0; list && i < list.length; i++) {
            var b = list[i]
            var o = typeof b === "object" && b !== null ? b : { time: b }
            out.push({ time: Number(o.time) || 0, repeatStart: !!o.repeatStart, repeatEnd: !!o.repeatEnd })
        }
        var lastEnd = 0
        for (var k = 0; ns && k < ns.length; k++) {
            lastEnd = Math.max(lastEnd, Number(ns[k].time) + Math.max(0, Number(ns[k].duration) || 0))
        }
        if (out.length === 0) {
            out.push({ time: ns && ns.length ? Math.min(0, Number(ns[0].time)) : 0, repeatStart: false, repeatEnd: false })
        }
        for (var j = 0; j < out.length; j++) {
            if (j + 1 < out.length) {
                out[j].end = out[j + 1].time
            } else {
                var len = j > 0 ? out[j].time - out[j - 1].time : lastEnd - out[j].time
                out[j].end = out[j].time + Math.max(len > 0 ? len : 2, lastEnd - out[j].time)
            }
        }
        return out
    }
    function barAt(t) {
        var a = 0, b = barList.length
        while (a < b) {
            var m = (a + b) >> 1
            if (barList[m].time <= t + 1e-6) {
                a = m + 1
            } else {
                b = m
            }
        }
        return Math.max(0, a - 1)
    }
    function rowOf(s) {
        return lowStringOnTop ? s : strings - 1 - s
    }
    /** x of time t in bar b on its line (padding left and right of the bar). */
    function xOf(t, b) {
        var bar = barList[b]
        var col = (b - firstBar) % perSystem
        var padL = ls * 1.2, padR = ls * 0.8
        var f = (t - bar.time) / Math.max(1e-6, bar.end - bar.time)
        return headerWidth + col * barWidth + padL + Math.max(0, Math.min(1, f)) * (barWidth - padL - padR)
    }
    function sysOf(b) {
        return Math.floor((b - firstBar) / perSystem)
    }
    /** Rhythm of a duration in quarters: {level: beams/flags (0 none), stem: 0 none / 1 short / 2 full, dot}. */
    function rhythmOf(beats) {
        var b = Number(beats) || 0
        var bases = [[4, 0, 0], [2, 0, 1], [1, 0, 2], [0.5, 1, 2], [0.25, 2, 2], [0.125, 3, 2]]
        for (var i = 0; i < bases.length; i++) {
            var base = bases[i][0]
            if (Math.abs(b - base * 1.5) < 1e-3) {
                return { level: bases[i][1], stem: bases[i][2], dot: true }
            }
            if (b >= base - 1e-3) {
                return { level: bases[i][1], stem: bases[i][2], dot: false }
            }
        }
        return { level: 3, stem: 2, dot: false }
    }

    function computeLayout(first, per, nsys, w, ls, ns, bl, nstr, lowTop, withRhythm) {
        var L = { lines: [], frets: [], marks: [], rects: [], flags: [], heads: [], boxes: {} }
        if (w <= 0 || bl.length === 0) {
            return L
        }
        var last = Math.min(bl.length - 1, first + per * nsys - 1)
        for (var s = 0; s < nsys; s++) {
            var sy = s * systemHeight
            var top = sy + staffTop
            var used = Math.min(per, Math.max(0, last - (first + s * per) + 1))
            var right = headerWidth + used * barWidth
            if (used <= 0) {
                continue
            }
            for (var l = 0; l < nstr; l++) {
                L.lines.push({ x: headerWidth - ls * 0.4, y: top + l * ls, w: right - headerWidth + ls * 0.4, h: 1, kind: "line" })
            }
            L.heads.push({ y: top, sys: s })
            L.lines.push({ x: headerWidth - ls * 0.4, y: top, w: 1, h: staffHeight + 1, kind: "bar" })
            for (var c = 0; c < used; c++) {
                var bi = first + s * per + c
                var bx = headerWidth + c * barWidth
                var bar = bl[bi]
                L.lines.push({ x: bx + barWidth - 1, y: top, w: 1, h: staffHeight + 1, kind: "bar" })
                L.heads.push({ y: sy, x: bx, number: bi + 1 })
                if (bar.repeatStart) {
                    L.lines.push({ x: bx, y: top, w: Math.max(2, ls * 0.25), h: staffHeight + 1, kind: "bar" })
                    L.rects.push({ x: bx + ls * 0.5, y: 0, sys: s, kind: "dots" })
                }
                if (bar.repeatEnd) {
                    L.lines.push({ x: bx + barWidth - Math.max(2, ls * 0.25), y: top, w: Math.max(2, ls * 0.25), h: staffHeight + 1, kind: "bar" })
                    L.rects.push({ x: bx + barWidth - ls * 0.75, y: 0, sys: s, kind: "dots" })
                }
            }
        }
        // Notes of the page.
        var from = bl[first].time, to = bl[last].end
        var onsets = []
        for (var i = 0; ns && i < ns.length; i++) {
            var n = ns[i]
            var t = Number(n.time)
            if (t < from - 1e-6 || t >= to - 1e-6) {
                continue
            }
            var b = barAt(t)
            var x = xOf(t, b)
            var sys = sysOf(b)
            var y = sys * systemHeight + staffTop + rowOf(Math.max(0, Math.min(nstr - 1, Number(n.string) || 0))) * ls
            L.frets.push({ x: x, y: y, text: n.fret !== undefined ? String(n.fret) : "", state: n.state || "pending", index: i, label: n.label || "" })
            L.boxes[i] = { x: x, sys: sys }
            var prev = onsets.length ? onsets[onsets.length - 1] : null
            if (prev && Math.abs(prev.t - t) < 1e-3) {
                if (!prev.marked && (n.state || "pending") !== "pending") {
                    prev.state = n.state
                }
                continue
            }
            var dur = Math.max(0, Number(n.duration) || 0)
            var beats = Number(n.beats) || 0
            var spq = dur > 0 && beats > 0 ? dur / beats : (bl[b].end - bl[b].time) / 4
            onsets.push({ t: t, x: x, sys: sys, bar: b, beat: Math.floor((t - bl[b].time) / spq + 1e-3),
                          r: rhythmOf(beats > 0 ? beats : dur / spq), state: n.state || "pending" })
        }
        for (var o = 0; o < onsets.length; o++) {
            var on = onsets[o]
            if (on.state !== "pending") {
                L.marks.push({ x: on.x, y: on.sys * systemHeight + markRow / 2, state: on.state })
            }
        }
        if (!withRhythm) {
            return L
        }
        // Stems, dots, beams within a beat, flags for single short notes.
        var stemW = Math.max(1, Math.round(ls * 0.1))
        var beamH = Math.max(2, Math.round(ls * 0.28))
        for (var k = 0; k < onsets.length; k++) {
            var e = onsets[k]
            var y0 = e.sys * systemHeight + staffTop + staffHeight + ls * 0.6
            var y1 = y0 + ls * 2.1
            e.y1 = y1
            if (e.r.stem > 0) {
                L.lines.push({ x: e.x - stemW / 2, y: e.r.stem === 1 ? y1 - ls * 1.0 : y0, w: stemW, h: e.r.stem === 1 ? ls * 1.0 : y1 - y0, kind: "stem" })
            }
            if (e.r.dot) {
                L.lines.push({ x: e.x + ls * 0.35, y: y1 - ls * 0.45, w: Math.max(2, ls * 0.24), h: Math.max(2, ls * 0.24), kind: "stem" })
            }
        }
        var g = 0
        while (g < onsets.length) {
            var h = g
            if (onsets[g].r.level > 0) {
                while (h + 1 < onsets.length && onsets[h + 1].r.level > 0 && onsets[h + 1].bar === onsets[g].bar
                       && onsets[h + 1].beat === onsets[g].beat && onsets[h + 1].sys === onsets[g].sys) {
                    h++
                }
            }
            if (h > g) {
                var gx0 = onsets[g].x - stemW / 2, gx1 = onsets[h].x + stemW / 2
                L.lines.push({ x: gx0, y: onsets[g].y1 - beamH, w: gx1 - gx0, h: beamH, kind: "beam" })
                for (var lv = 2; lv <= 3; lv++) {
                    var off = (lv - 1) * (beamH + Math.max(2, ls * 0.18))
                    for (var q = g; q <= h; q++) {
                        if (onsets[q].r.level < lv) {
                            continue
                        }
                        var r = q
                        while (r + 1 <= h && onsets[r + 1].r.level >= lv) {
                            r++
                        }
                        if (r > q) {
                            L.lines.push({ x: onsets[q].x - stemW / 2, y: onsets[q].y1 - beamH - off, w: onsets[r].x - onsets[q].x + stemW, h: beamH, kind: "beam" })
                        } else {
                            var stub = ls * 0.8
                            var left = q === h
                            L.lines.push({ x: left ? onsets[q].x - stub : onsets[q].x - stemW / 2, y: onsets[q].y1 - beamH - off, w: stub + stemW / 2, h: beamH, kind: "beam" })
                        }
                        q = r
                    }
                }
            } else if (onsets[g].r.level > 0) {
                L.flags.push({ x: onsets[g].x - stemW / 2, y: onsets[g].y1, glyph: ["", "", "", ""][onsets[g].r.level] })
            }
            g = h + 1
        }
        return L
    }

    // Selection band behind the current note.
    Rectangle {
        readonly property var box: staff.currentIndex >= 0 ? staff.layout.boxes[staff.currentIndex] : undefined
        visible: box !== undefined
        x: visible ? box.x - staff.ls * 0.8 : 0
        y: visible ? box.sys * staff.systemHeight + staff.staffTop - staff.ls * 0.6 : 0
        width: staff.ls * 1.6
        height: staff.staffHeight + staff.ls * 1.2
        color: KanteStyle.selectionColor
        Rectangle { anchors.bottom: parent.bottom; width: parent.width; height: 2; color: KanteStyle.focusColor }
    }

    Repeater {
        model: staff.layout.lines
        delegate: Rectangle {
            required property var modelData
            x: modelData.x
            y: modelData.y
            width: modelData.w
            height: modelData.h
            color: modelData.kind === "line" ? KanteStyle.mutedTextColor
                 : modelData.kind === "bar" ? KanteStyle.textColor : KanteStyle.textColor
            opacity: modelData.kind === "line" ? 0.7 : 1
        }
    }

    // Repeat dots: in the spaces next to the middle of the staff.
    Repeater {
        model: staff.layout.rects
        delegate: Item {
            required property var modelData
            readonly property int spaces: Math.max(1, staff.strings - 1)
            readonly property real mid: staff.staffHeight / 2
            readonly property real d: spaces % 2 === 1 ? staff.ls : staff.ls / 2
            x: modelData.x
            y: modelData.sys * staff.systemHeight + staff.staffTop
            Repeater {
                model: 2
                delegate: Rectangle {
                    required property int index
                    width: Math.max(3, staff.ls * 0.32)
                    height: width
                    x: -width / 2
                    y: (index === 0 ? parent.mid - parent.d : parent.mid + parent.d) - height / 2
                    color: KanteStyle.textColor
                }
            }
        }
    }

    // Line heads: string names and TAB; bar numbers.
    Repeater {
        model: staff.layout.heads
        delegate: Item {
            id: hd
            required property var modelData
            readonly property bool line: modelData.sys !== undefined
            Repeater {
                model: hd.line ? staff.strings : 0
                delegate: Text {
                    required property int index
                    x: KanteStyle.unit(2)
                    y: hd.modelData.y + staff.rowOf(index) * staff.ls - height / 2
                    text: staff.nameOf(index)
                    color: KanteStyle.mutedTextColor
                    font: nameMetrics.font
                }
            }
            Column {
                visible: hd.line
                x: nameMetrics.width + staff.ls * 0.8
                y: hd.line ? hd.modelData.y + (staff.staffHeight - height) / 2 : 0
                spacing: -staff.ls * 0.25
                Repeater {
                    model: ["T", "A", "B"]
                    delegate: Text {
                        required property string modelData
                        text: modelData
                        color: KanteStyle.strongTextColor
                        font.family: KanteStyle.headingFont(10).family
                        font.weight: Font.Bold
                        font.pixelSize: Math.round(Math.min(staff.ls * 1.0, (staff.staffHeight + staff.ls) / 3))
                    }
                }
            }
            Text {
                visible: !hd.line
                x: hd.line ? 0 : hd.modelData.x + KanteStyle.unit(2)
                y: hd.line ? 0 : hd.modelData.y
                text: hd.line ? "" : hd.modelData.number
                color: KanteStyle.mutedTextColor
                font: KanteStyle.monoFont(KanteStyle.labelFont().pointSize * 0.85, false)
            }
        }
    }

    Repeater {
        model: staff.layout.frets
        delegate: Rectangle {
            required property var modelData
            readonly property bool current: modelData.index === staff.currentIndex
            x: modelData.x - width / 2
            y: modelData.y - height / 2
            width: fret.implicitWidth + staff.ls * 0.3
            height: Math.round(staff.ls * 0.95)
            color: current ? "transparent" : staff.groundColor
            Text {
                id: fret
                anchors.centerIn: parent
                text: parent.modelData.text
                color: parent.modelData.state === "pending" ? (parent.current ? KanteStyle.strongTextColor : KanteStyle.textColor)
                     : KanteStyle.noteStateColor(parent.modelData.state)
                font.family: nameMetrics.font.family
                font.weight: parent.current ? Font.Bold : Font.Medium
                font.pixelSize: Math.round(staff.ls * 0.9)
            }
            Text {
                visible: parent.modelData.label !== ""
                anchors.bottom: parent.top
                anchors.horizontalCenter: parent.horizontalCenter
                text: parent.modelData.label
                color: KanteStyle.mutedTextColor
                font.family: nameMetrics.font.family
                font.pixelSize: Math.round(staff.ls * 0.6)
            }
        }
    }

    Repeater {
        model: staff.layout.marks
        delegate: KanteNoteMark {
            required property var modelData
            width: Math.round(staff.ls * 0.95)
            height: width
            x: modelData.x - width / 2
            y: modelData.y - height / 2
            noteState: modelData.state
        }
    }

    Repeater {
        model: staff.layout.flags
        delegate: Text {
            required property var modelData
            x: modelData.x
            y: modelData.y - baselineOffset
            text: modelData.glyph
            color: KanteStyle.textColor
            font.family: music.font.family
            font.pixelSize: Math.round(staff.ls * 2.0)
        }
    }

    // Cursor at `position`.
    Rectangle {
        readonly property int bar: staff.currentBar
        readonly property int sys: staff.sysOf(bar)
        visible: staff.cursor && sys >= 0 && sys < staff.systems && staff.barList.length > 0
        x: visible ? Math.round(staff.xOf(staff.position, bar)) : 0
        y: sys * staff.systemHeight + staff.staffTop - staff.ls * 0.8
        width: Math.max(2, KanteStyle.unit(2))
        height: staff.staffHeight + staff.ls * 1.6
        color: KanteStyle.accentColor
    }
}
