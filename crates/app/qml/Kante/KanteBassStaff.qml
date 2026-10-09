import QtQuick
import QtQuick.Shapes
import "."

/**
 * Notation in the bass clef, one voice (Kante 1.23, for Kontra). Glyphs from Bravura
 * (SMuFL, SIL OFL, fonts/OFL-Bravura.txt): clef, key and time signatures, noteheads,
 * flags, rests, accidentals, dots. Drawn here: staff and ledger lines, stems (up below
 * the middle line, down from it), beams within a beat (eighths and shorter; a dotted
 * quarter in 6/8, 9/8, 12/8), ties and the triplet 3 over a beamed group.
 *
 *   notes      [{time, beats, midi, state, tied}], sorted by time; time in seconds,
 *              beats = duration in quarters (1.5 dotted quarter, 1/3 eighth triplet),
 *              tied: tied to the next note (same pitch), state as KanteTabLane
 *   rests      [{time, beats}]
 *   bars       [{time, numerator, denominator}] (numbers or a missing signature keep
 *              the previous one; 4/4 at first); the time signature shows on the first
 *              bar of the page and where it changes
 *   keyFifths  key signature: 1..7 sharps, -1..-7 flats; notes are spelled in the key
 *              (else sharps in sharp keys, flats in flat keys), an accidental holds to
 *              the end of the bar
 *   position   the current time in seconds
 *
 * Pages line by line as KanteTabStaff: the top line holds the current bar, `systems - 1`
 * lines preview. The current note sits on a selection band, states show above the staff
 * (KanteNoteMark) and in the note's colour. Spacing within a bar follows time. Sizes
 * come from `staffSpace` (KanteStyle.unit, so it scales with the platform), readable
 * at 200 %. The layout is computed once per page, not per frame.
 */
Item {
    id: staff

    property var notes: []
    property var rests: []
    property var bars: []
    property int keyFifths: 0
    property real position: 0
    property int barsPerSystem: 0
    property int systems: 1
    property bool cursor: true
    /** Distance of two staff lines. */
    property real staffSpace: KanteStyle.unit(10)
    property real minBarWidth: staffSpace * 18
    property string accessibleName: "Noten, Bassschlüssel"
    property string barText: "Takt %1 von %2"
    /** Note names for screen readers (C D E F G A B; German: H for B), and the accidental words. */
    property var letterNames: ["C", "D", "E", "F", "G", "A", "H"]
    property string sharpText: "is"
    property string flatText: "es"

    readonly property real sp: Math.max(4, staffSpace)
    readonly property int currentIndex: lastAtOrBefore(position)
    readonly property var barList: normalizeBars(bars, notes, rests)
    readonly property int currentBar: barAt(position)
    readonly property real headerWidth: sp * (4.2 + Math.abs(Math.max(-7, Math.min(7, keyFifths))) * 1.1 + 3.0)
    readonly property int perSystem: barsPerSystem > 0 ? barsPerSystem
        : Math.max(1, Math.floor((width - headerWidth) / Math.max(1, minBarWidth)))
    readonly property int firstBar: Math.max(0, Math.floor(currentBar / perSystem) * perSystem)
    readonly property real barWidth: (width - headerWidth) / perSystem
    readonly property real staffTop: sp * 5.5
    readonly property real systemHeight: staffTop + sp * 4 + sp * 5
    readonly property var layout: computeLayout(firstBar, perSystem, Math.max(1, systems), width, sp, notes, rests, barList, keyFifths)

    implicitWidth: KanteStyle.unit(720)
    implicitHeight: Math.ceil(systemHeight * Math.max(1, systems))

    Accessible.role: Accessible.Chart
    Accessible.name: accessibleName
    Accessible.description: {
        var d = barText.arg(currentBar + 1).arg(barList.length)
        if (currentIndex >= 0) {
            var n = notes[currentIndex]
            d += ", " + pitchName(Number(n.midi)) + " (" + KanteStyle.noteStateName(n.state || "pending") + ")"
        }
        return d
    }

    FontLoader { id: music; source: Qt.resolvedUrl("fonts/Bravura.otf") }

    // ── Pitch ────────────────────────────────────────────────────────
    readonly property var letterPc: [0, 2, 4, 5, 7, 9, 11]
    function keyAlter(letter, fifths) {
        var sharps = [3, 0, 4, 1, 5, 2, 6], flats = [6, 2, 5, 1, 4, 0, 3]
        var f = Math.max(-7, Math.min(7, fifths))
        for (var i = 0; i < Math.abs(f); i++) {
            if ((f > 0 ? sharps : flats)[i] === letter) {
                return f > 0 ? 1 : -1
            }
        }
        return 0
    }
    /** {letter 0..6 (C..B), alter -1/0/1, octave, pos} of a MIDI pitch in the key; pos 0 = bottom line (G2). */
    function spell(midi, fifths) {
        var pc = ((midi % 12) + 12) % 12
        var best = null
        for (var pass = 0; pass < 3 && !best; pass++) {
            for (var l = 0; l < 7 && !best; l++) {
                var a = pc - letterPc[l]
                if (a > 6) a -= 12
                if (a < -6) a += 12
                if (Math.abs(a) > 1) continue
                if ((pass === 0 && a === keyAlter(l, fifths) )
                        || (pass === 1 && a === 0)
                        || (pass === 2 && a === (fifths < 0 ? -1 : 1))) {
                    best = { letter: l, alter: a }
                }
            }
        }
        if (!best) {
            best = { letter: 0, alter: 0 }
        }
        best.octave = Math.floor((midi - best.alter) / 12) - 1
        best.pos = best.octave * 7 + best.letter - 18
        return best
    }
    function pitchName(midi) {
        if (isNaN(midi)) {
            return ""
        }
        var s = spell(midi, keyFifths)
        var name = String(letterNames[s.letter])
        return name + (s.alter > 0 ? sharpText : s.alter < 0 ? flatText : "") + s.octave
    }
    /** Note value of a duration in quarters: {type 0 whole .. 5 32nd, dots, triplet}. */
    function valueOf(beats) {
        var b = Number(beats) || 0
        var bases = [4, 2, 1, 0.5, 0.25, 0.125]
        for (var i = 0; i < bases.length; i++) {
            if (Math.abs(b - bases[i]) < 1e-3) return { type: i, dots: 0, triplet: false }
            if (Math.abs(b - bases[i] * 1.5) < 1e-3) return { type: i, dots: 1, triplet: false }
            if (Math.abs(b - bases[i] * 1.75) < 1e-3) return { type: i, dots: 2, triplet: false }
            if (Math.abs(b - bases[i] * 2 / 3) < 1e-3) return { type: i, dots: 0, triplet: true }
        }
        for (var k = 0; k < bases.length; k++) {
            if (b >= bases[k]) return { type: k, dots: 0, triplet: false }
        }
        return { type: 5, dots: 0, triplet: false }
    }

    // ── Time ─────────────────────────────────────────────────────────
    function lastAtOrBefore(t) {
        var a = 0, b = notes ? notes.length : 0
        while (a < b) {
            var m = (a + b) >> 1
            if (Number(notes[m].time) <= t + 1e-6) a = m + 1
            else b = m
        }
        return a - 1
    }
    function normalizeBars(list, ns, rs) {
        var out = [], num = 4, den = 4
        for (var i = 0; list && i < list.length; i++) {
            var o = typeof list[i] === "object" && list[i] !== null ? list[i] : { time: list[i] }
            var changed = i === 0
            if (o.numerator && o.denominator && (o.numerator !== num || o.denominator !== den)) {
                num = Number(o.numerator)
                den = Number(o.denominator)
                changed = true
            }
            out.push({ time: Number(o.time) || 0, num: num, den: den, changed: changed })
        }
        // Seconds per quarter, from the notes (median of time step / beats): sizes a bar
        // when none or only the start of the last one is given.
        var lastEnd = 0, steps = []
        for (var k = 0; ns && k < ns.length; k++) {
            var b = Number(ns[k].beats) || 0
            if (k + 1 < ns.length && b > 0 && Number(ns[k + 1].time) > Number(ns[k].time)) {
                steps.push((Number(ns[k + 1].time) - Number(ns[k].time)) / b)
            }
        }
        steps.sort(function (a, b) { return a - b })
        var spq = steps.length ? steps[Math.floor(steps.length / 2)] : 0.5
        for (var k2 = 0; ns && k2 < ns.length; k2++) lastEnd = Math.max(lastEnd, Number(ns[k2].time) + (Number(ns[k2].beats) || 0) * spq)
        for (var r = 0; rs && r < rs.length; r++) lastEnd = Math.max(lastEnd, Number(rs[r].time) + (Number(rs[r].beats) || 0) * spq)
        if (out.length === 0) {
            out.push({ time: 0, num: 4, den: 4, changed: true })
        }
        for (var j = 0; j < out.length; j++) {
            if (j + 1 < out.length) {
                out[j].end = out[j + 1].time
            } else {
                var len = j > 0 ? out[j].time - out[j - 1].time : out[j].num * 4 / out[j].den * spq
                out[j].end = out[j].time + len
                // More notes after the last given bar: add bars of the same length.
                while (out[out.length - 1].end < lastEnd - 1e-3 && len > 0) {
                    var prev = out[out.length - 1]
                    out.push({ time: prev.end, end: prev.end + len, num: prev.num, den: prev.den, changed: false })
                }
                break
            }
        }
        return out
    }
    function barAt(t) {
        var a = 0, b = barList.length
        while (a < b) {
            var m = (a + b) >> 1
            if (barList[m].time <= t + 1e-6) a = m + 1
            else b = m
        }
        return Math.max(0, a - 1)
    }
    function sysOf(b) {
        return Math.floor((b - firstBar) / perSystem)
    }
    function padLeft(b) {
        var bar = barList[b]
        var showTime = bar.changed && (b - firstBar) % perSystem !== 0 && b !== firstBar
        return sp * (showTime ? 4.4 : 2.0)
    }
    function xOf(t, b) {
        var bar = barList[b]
        var col = (b - firstBar) % perSystem
        var pl = padLeft(b), pr = sp * 1.4
        var f = (t - bar.time) / Math.max(1e-6, bar.end - bar.time)
        return headerWidth + col * barWidth + pl + Math.max(0, Math.min(1, f)) * (barWidth - pl - pr)
    }
    function yOf(pos, sys) {
        return sys * systemHeight + staffTop + sp * 4 - pos * sp / 2
    }

    // ── Layout ───────────────────────────────────────────────────────
    function computeLayout(first, per, nsys, w, sp, ns, rs, bl, fifths) {
        var L = { glyphs: [], rects: [], polys: [], ties: [], marks: [], boxes: {} }
        if (w <= 0 || bl.length === 0) {
            return L
        }
        var G = { clef: "", sharp: "", flat: "", natural: "", dot: "",
                  heads: ["", "", "", "", "", ""],
                  rests: ["", "", "", "", "", ""],
                  flagsUp: ["", "", "", "", "", ""], flagsDown: ["", "", "", "", "", ""] }
        var lineT = Math.max(1, Math.round(sp * 0.13)), stemW = Math.max(1, Math.round(sp * 0.12))
        var ledgerT = Math.max(1, Math.round(sp * 0.16))
        var f = Math.max(-7, Math.min(7, fifths))
        var last = Math.min(bl.length - 1, first + per * nsys - 1)
        function digits(n, x, y) {
            var s = String(n)
            for (var i = 0; i < s.length; i++) {
                L.glyphs.push({ x: x + (i - s.length / 2) * sp * 1.8 , y: y, g: String.fromCharCode(0xE080 + Number(s[i])), ink: "" })
            }
        }
        function timeSig(bar, x, sys) {
            digits(bar.num, x, yOf(6, sys))
            digits(bar.den, x, yOf(2, sys))
        }
        // Staff, clef, key, bars.
        for (var s = 0; s < nsys; s++) {
            var used = Math.min(per, Math.max(0, last - (first + s * per) + 1))
            if (used <= 0) continue
            var right = headerWidth + used * barWidth
            for (var l = 0; l < 5; l++) {
                L.rects.push({ x: sp * 0.4, y: yOf(l * 2, s) - lineT / 2, w: right - sp * 0.4, h: lineT, ink: "line" })
            }
            L.rects.push({ x: sp * 0.4, y: yOf(8, s), w: Math.max(1, Math.round(sp * 0.16)), h: sp * 4, ink: "" })
            L.glyphs.push({ x: sp * 0.9, y: yOf(6, s), g: G.clef, ink: "" })
            var kp = f > 0 ? [6, 3, 7, 4, 1, 5, 2] : [2, 5, 1, 4, 0, 3, -1]
            for (var k = 0; k < Math.abs(f); k++) {
                L.glyphs.push({ x: sp * (4.2 + k * 1.1), y: yOf(kp[k], s), g: f > 0 ? G.sharp : G.flat, ink: "" })
            }
            var firstOfLine = first + s * per
            if (s === 0 || bl[firstOfLine].changed) {
                timeSig(bl[firstOfLine], sp * (4.2 + Math.abs(f) * 1.1 + 1.3), s)
            }
            for (var c = 0; c < used; c++) {
                var bi = firstOfLine + c
                var bx = headerWidth + c * barWidth
                var endX = bx + barWidth
                var finalBar = bi === bl.length - 1
                L.rects.push({ x: endX - (finalBar ? sp * 0.9 : Math.max(1, Math.round(sp * 0.16))), y: yOf(8, s),
                               w: Math.max(1, Math.round(sp * 0.16)), h: sp * 4, ink: "" })
                if (finalBar) {
                    L.rects.push({ x: endX - sp * 0.5, y: yOf(8, s), w: sp * 0.5, h: sp * 4, ink: "" })
                }
                L.glyphs.push({ x: bx + sp * 0.2, y: yOf(8, s) - sp * 1.0, g: "", text: String(bi + 1), ink: "muted" })
                if (c > 0 && bl[bi].changed) {
                    timeSig(bl[bi], bx + sp * 1.6, s)
                }
            }
        }
        var from = bl[first].time, to = bl[last].end
        // Rests.
        for (var r = 0; rs && r < rs.length; r++) {
            var rt = Number(rs[r].time)
            if (rt < from - 1e-6 || rt >= to - 1e-6) continue
            var rb = barAt(rt), rsys = sysOf(rb)
            var rv = valueOf(rs[r].beats)
            var fullBar = Math.abs(Number(rs[r].beats) - bl[rb].num * 4 / bl[rb].den) < 1e-3
            var rx = fullBar ? headerWidth + ((rb - first) % per + 0.5) * barWidth - sp * 0.6 : xOf(rt, rb) - sp * 0.5
            var rtype = fullBar ? 0 : rv.type
            L.glyphs.push({ x: rx, y: yOf(rtype === 0 ? 6 : 4, rsys), g: G.rests[rtype], ink: "" })
            for (var rd = 0; rd < rv.dots && !fullBar; rd++) {
                L.glyphs.push({ x: rx + sp * (1.5 + rd * 0.5), y: yOf(5, rsys), g: G.dot, ink: "" })
            }
        }
        // Notes: heads, accidentals, dots, ledger lines; stems and beams after grouping.
        var items = []
        var acc = {}, accBar = -1
        for (var i = 0; ns && i < ns.length; i++) {
            var n = ns[i]
            var t = Number(n.time)
            if (t < from - 1e-6 || t >= to - 1e-6) continue
            var b = barAt(t), sys = sysOf(b)
            if (b !== accBar) {
                acc = {}
                accBar = b
            }
            var midi = Number(n.midi)
            var sp_ = spell(midi, f)
            var v = valueOf(n.beats)
            var headW = sp * (v.type === 0 ? 1.69 : 1.18)
            var x = xOf(t, b) - headW / 2
            var y = yOf(sp_.pos, sys)
            var st = n.state || "pending"
            var prevTied = i > 0 && ns[i - 1].tied && Number(ns[i - 1].midi) === midi
            var key = sp_.pos
            var current = acc[key] !== undefined ? acc[key] : keyAlter(sp_.letter, f)
            if (sp_.alter !== current && !(prevTied && barAt(Number(ns[i - 1].time)) !== b)) {
                var ag = sp_.alter > 0 ? G.sharp : sp_.alter < 0 ? G.flat : G.natural
                var aw = sp_.alter > 0 ? 1.0 : sp_.alter < 0 ? 0.9 : 0.7
                L.glyphs.push({ x: x - sp * (aw + 0.25), y: y, g: ag, ink: st, index: i })
            }
            acc[key] = sp_.alter
            L.glyphs.push({ x: x, y: y, g: G.heads[v.type], ink: st, index: i })
            for (var d = 0; d < v.dots; d++) {
                L.glyphs.push({ x: x + headW + sp * (0.35 + d * 0.5), y: yOf(sp_.pos % 2 === 0 ? sp_.pos + 1 : sp_.pos, sys), g: G.dot, ink: st, index: i })
            }
            for (var p = -2; p >= sp_.pos; p -= 2) {
                L.rects.push({ x: x - sp * 0.4, y: yOf(p, sys) - ledgerT / 2, w: headW + sp * 0.8, h: ledgerT, ink: "" })
            }
            for (var q = 10; q <= sp_.pos; q += 2) {
                L.rects.push({ x: x - sp * 0.4, y: yOf(q, sys) - ledgerT / 2, w: headW + sp * 0.8, h: ledgerT, ink: "" })
            }
            if (st !== "pending") {
                L.marks.push({ x: x + headW / 2, y: sys * systemHeight + sp * 1.0, state: st })
            }
            L.boxes[i] = { x: x, w: headW, sys: sys }
            var unit = bl[b].den === 8 && bl[b].num % 3 === 0 ? 1.5 : 1
            var spq = (bl[b].end - bl[b].time) / (bl[b].num * 4 / bl[b].den)
            items.push({ i: i, x: x, y: y, pos: sp_.pos, w: headW, v: v, sys: sys, bar: b, st: st, tied: !!n.tied,
                         group: Math.floor((t - bl[b].time) / spq / unit + 1e-3), t: t })
        }
        // A rest between two notes ends a beam.
        function restBetween(t0, t1) {
            for (var k = 0; rs && k < rs.length; k++) {
                var x = Number(rs[k].time)
                if (x > t0 + 1e-6 && x < t1 - 1e-6) return true
            }
            return false
        }
        var g = 0
        while (g < items.length) {
            var h = g
            if (items[g].v.type >= 3) {
                while (h + 1 < items.length && items[h + 1].v.type >= 3 && items[h + 1].bar === items[g].bar
                       && items[h + 1].group === items[g].group && items[h + 1].sys === items[g].sys
                       && !restBetween(items[h].t, items[h + 1].t)) {
                    h++
                }
            }
            var grp = items.slice(g, h + 1)
            var avg = 0
            for (var a = 0; a < grp.length; a++) avg += grp[a].pos
            avg /= grp.length
            var up = avg < 4
            var sx = function (it) { return up ? it.x + it.w - stemW : it.x }
            var natural = function (it) {
                // 3.5 spaces, and at least to the middle line for notes on ledger lines.
                var mid = yOf(4, it.sys)
                return up ? Math.min(it.y - sp * 3.5, mid) : Math.max(it.y + sp * 3.5, mid)
            }
            var beamAt = null
            if (grp.length > 1) {
                var x0 = sx(grp[0]), x1 = sx(grp[grp.length - 1])
                var b0 = natural(grp[0]), b1 = natural(grp[grp.length - 1])
                var dy = Math.max(-sp, Math.min(sp, b1 - b0))
                var slope = x1 > x0 ? dy / (x1 - x0) : 0
                var shift = 0
                for (var m = 0; m < grp.length; m++) {
                    var by = b0 + slope * (sx(grp[m]) - x0)
                    var need = up ? by - (grp[m].y - sp * 2.5) : (grp[m].y + sp * 2.5) - by
                    shift = Math.max(shift, need)
                }
                b0 += up ? -shift : shift
                beamAt = function (x) { return b0 + slope * (x - x0) }
                var thick = sp * 0.5, gap = sp * 0.25
                for (var lv = 1; lv <= 3; lv++) {
                    var off = (lv - 1) * (thick + gap) * (up ? 1 : -1)
                    for (var u = 0; u < grp.length; u++) {
                        if (grp[u].v.type < 2 + lv) continue
                        var e = u
                        while (e + 1 < grp.length && grp[e + 1].v.type >= 2 + lv) e++
                        var xa = sx(grp[u]), xb = sx(grp[e]) + stemW
                        if (e === u) {
                            var stub = sp * 1.1
                            if (u === grp.length - 1) { xa = sx(grp[u]) - stub; xb = sx(grp[u]) + stemW }
                            else { xa = sx(grp[u]); xb = xa + stub }
                        }
                        var ya = beamAt(xa) + off, yb = beamAt(xb) + off
                        var t2 = up ? thick : -thick
                        L.polys.push({ pts: [xa, ya, xb, yb, xb, yb + t2, xa, ya + t2], ink: "" })
                        u = e
                    }
                }
                var allTriplets = grp.every(function (it) { return it.v.triplet })
                if (allTriplets) {
                    var cx = (x0 + x1) / 2
                    var cy = beamAt(cx) + (up ? -sp * 1.2 : sp * 2.0)
                    L.glyphs.push({ x: cx - sp * 0.5, y: cy, g: "", ink: "" })
                }
            }
            for (var z = 0; z < grp.length; z++) {
                var it = grp[z]
                if (it.v.type === 0) continue
                var end = beamAt ? beamAt(sx(it)) : natural(it)
                var attach = up ? it.y - sp * 0.168 : it.y + sp * 0.168
                L.rects.push({ x: sx(it), y: Math.min(end, attach), w: stemW, h: Math.abs(attach - end), ink: "" })
                if (!beamAt && it.v.type >= 3) {
                    L.glyphs.push({ x: sx(it), y: end, g: (up ? G.flagsUp : G.flagsDown)[it.v.type], ink: "" })
                }
                if (!beamAt && it.v.triplet) {
                    L.glyphs.push({ x: it.x, y: up ? end - sp * 0.8 : end + sp * 1.8, g: "", ink: "" })
                }
                it.up = up
            }
            g = h + 1
        }
        // Ties: to the next note, on the side away from the stem; to the line end across lines.
        for (var y2 = 0; y2 < items.length; y2++) {
            var a1 = items[y2]
            if (!a1.tied) continue
            var nxt = y2 + 1 < items.length && items[y2 + 1].i === a1.i + 1 ? items[y2 + 1] : null
            var tx0 = a1.x + a1.w + sp * 0.1
            var tx1 = nxt && nxt.sys === a1.sys ? nxt.x - sp * 0.1 : headerWidth + Math.min(per, bl.length - first - a1.sys * per) * barWidth - sp * 0.3
            var below = a1.up !== false
            var ty = a1.y + (below ? sp * 0.45 : -sp * 0.45)
            var hgt = Math.min(sp * 1.2, Math.max(sp * 0.5, (tx1 - tx0) * 0.12)) * (below ? 1 : -1)
            var th = sp * 0.36 * (below ? 1 : -1)
            var dx = (tx1 - tx0) / 3
            L.ties.push({ ink: a1.st, path: "M " + tx0 + " " + ty
                + " C " + (tx0 + dx) + " " + (ty + hgt) + " " + (tx1 - dx) + " " + (ty + hgt) + " " + tx1 + " " + ty
                + " C " + (tx1 - dx) + " " + (ty + hgt - th) + " " + (tx0 + dx) + " " + (ty + hgt - th) + " " + tx0 + " " + ty + " Z" })
        }
        return L
    }

    function inkColor(ink) {
        if (ink === "line") return KanteStyle.mutedTextColor
        if (ink === "muted") return KanteStyle.mutedTextColor
        if (ink === "" || ink === "pending" || ink === undefined) return KanteStyle.textColor
        return KanteStyle.noteStateColor(ink)
    }

    // Selection band behind the current note.
    Rectangle {
        readonly property var box: staff.currentIndex >= 0 ? staff.layout.boxes[staff.currentIndex] : undefined
        visible: box !== undefined
        x: visible ? box.x - staff.sp * 0.6 : 0
        y: visible ? box.sys * staff.systemHeight + staff.staffTop - staff.sp * 1.5 : 0
        width: visible ? box.w + staff.sp * 1.2 : 0
        height: staff.sp * 7
        color: KanteStyle.selectionColor
        Rectangle { anchors.bottom: parent.bottom; width: parent.width; height: 2; color: KanteStyle.focusColor }
    }

    Repeater {
        model: staff.layout.rects
        delegate: Rectangle {
            required property var modelData
            x: modelData.x
            y: modelData.y
            width: modelData.w
            height: modelData.h
            color: staff.inkColor(modelData.ink)
            opacity: modelData.ink === "line" ? 0.8 : 1
        }
    }
    Repeater {
        model: staff.layout.polys
        delegate: Shape {
            required property var modelData
            antialiasing: true
            ShapePath {
                strokeColor: "transparent"
                fillColor: staff.inkColor(modelData.ink)
                PathPolyline {
                    path: {
                        var p = modelData.pts
                        return [Qt.point(p[0], p[1]), Qt.point(p[2], p[3]), Qt.point(p[4], p[5]), Qt.point(p[6], p[7]), Qt.point(p[0], p[1])]
                    }
                }
            }
        }
    }
    Repeater {
        model: staff.layout.ties
        delegate: Shape {
            required property var modelData
            antialiasing: true
            ShapePath {
                strokeColor: "transparent"
                fillColor: staff.inkColor(modelData.ink)
                PathSvg { path: modelData.path }
            }
        }
    }
    Repeater {
        model: staff.layout.glyphs
        delegate: Text {
            required property var modelData
            readonly property bool glyph: modelData.g !== ""
            readonly property bool current: modelData.index !== undefined && modelData.index === staff.currentIndex
            x: modelData.x
            y: modelData.y - (glyph ? baselineOffset : 0)
            text: glyph ? modelData.g : modelData.text
            color: current && (modelData.ink === "pending" || modelData.ink === "") ? KanteStyle.strongTextColor : staff.inkColor(modelData.ink)
            font.family: glyph ? music.font.family : KanteStyle.monoFont(10, false).family
            font.pixelSize: glyph ? Math.round(staff.sp * 4) : Math.max(8, Math.round(staff.sp * 0.95))
            Accessible.ignored: true
        }
    }
    Repeater {
        model: staff.layout.marks
        delegate: KanteNoteMark {
            required property var modelData
            width: Math.round(staff.sp * 1.3)
            height: width
            x: modelData.x - width / 2
            y: modelData.y - height / 2
            noteState: modelData.state
        }
    }

    Rectangle {
        readonly property int bar: staff.currentBar
        readonly property int sys: staff.sysOf(bar)
        visible: staff.cursor && sys >= 0 && sys < staff.systems && staff.barList.length > 0
        x: visible ? Math.round(staff.xOf(staff.position, bar)) : 0
        y: sys * staff.systemHeight + staff.staffTop - staff.sp * 1.5
        width: Math.max(2, KanteStyle.unit(2))
        height: staff.sp * 7
        color: KanteStyle.accentColor
        opacity: 0.8
    }
}
