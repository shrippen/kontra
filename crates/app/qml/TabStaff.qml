import QtQuick
import Kante

// The tablature view's staff, sized to its room as NotationStaff is: as many lines as fit,
// grown until they fill it, a short piece up to half again (QA 14.10., owner 12.11.: four
// lines and a third of the page empty, from a guessed line height). It lays out again only
// once a resize has ended (Kante 1.82 settleMs).
KanteTabStaff {
    // Size factor of the presentation (LH-ANZ-09), 1 otherwise.
    property real presentScale: 1
    // Most onsets in one bar and the room one takes (PracticePage.densestBar, tabNoteWidth).
    property int densestBar: 0
    property real noteWidth: 0

    fill: true
    settleMs: applicationWindow().staffSettleMs
    lineSpacing: KanteStyle.unit(14) * presentScale
    minBarWidth: Math.max(KanteStyle.unit(200), densestBar * noteWidth)
}
