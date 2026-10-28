import QtQuick
import Kante

// The notation view's staff, sized to its room. Takes `bars` like KanteBassStaff.
KanteBassStaff {
    id: staff

    // Size factor of the presentation (LH-ANZ-09), 1 otherwise.
    property real presentScale: 1

    // Most onsets in one bar (PracticePage.densestBar).
    property int densestBar: 0

    // Staff spaces per bar: 18 at least, 2.6 per onset of the densest bar, as the tablature
    // gives each (QA 27.10., M9: sixteenths ran into each other, eight bars in a line).
    readonly property real minBarSpaces: 18
    readonly property real noteSpaces: 2.6
    readonly property real barSpaces: Math.max(minBarSpaces, densestBar * noteSpaces)

    // Smaller where one line does not fit (beamer at 150 %): cut off, its beams ran
    // over the transport (G18). A KanteBassStaff line is 14.5 staff spaces, 16.5 with
    // chord symbols.
    // Two lines rather than one with room left under it, as long as their staff space
    // keeps three quarters of the wanted one (QA 14.10., presentation at the beamer:
    // two bars and a third of the screen empty).
    // A short piece grows up to half again while all its lines still fit (QA 14.10.);
    // a bar is `barSpaces` staff spaces wide, the clef and key about 8.
    staffSpace: {
        var wanted = KanteStyle.unit(10) * presentScale
        var lineSpaces = hasChords ? 16.5 : 14.5
        for (var grown = wanted * 1.5; grown > wanted; grown -= wanted * 0.05) {
            var perLine = Math.max(1, Math.floor((width - grown * 8) / (grown * barSpaces)))
            if (Math.ceil(bars.length / perLine) * lineSpaces * grown <= height) {
                return grown
            }
        }
        var line = height / lineSpaces
        return Math.min(wanted, line / 2 >= wanted * 0.75 ? line / 2 : line)
    }
    minBarWidth: staffSpace * barSpaces
    // As many lines as fit, as in the tablature (QA 13.10.: one line, the rest empty).
    systems: Math.max(1, Math.floor(height / systemHeight))
}
