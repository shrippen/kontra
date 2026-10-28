import QtQuick
import QtTest
import ".."

// Bars per line follow the densest bar (QA 27.10., M9, P3): sixteenths ran into each other, eight
// bars stood in one line. Run by scripts/check-video.sh.
Item {
    id: root
    width: 1200
    height: 600

    // Eight 4/4 bars at 60 BPM (a bar is 4 s) with `perBar` even notes each.
    function piece(perBar) {
        var bars = [], notes = []
        for (var b = 0; b < 8; b++) {
            bars.push({ time: b * 4, numerator: 4, denominator: 4 })
            for (var n = 0; n < perBar; n++) {
                notes.push({ time: b * 4 + n * 4 / perBar, beats: 4 / perBar, midi: 40, state: "pending" })
            }
        }
        return { bars: bars, notes: notes }
    }

    NotationStaff {
        id: staff
        anchors.fill: parent
    }

    TestCase {
        name: "NotationStaff"
        when: windowShown

        function load(perBar) {
            var p = root.piece(perBar)
            staff.bars = p.bars
            staff.notes = p.notes
            staff.densestBar = perBar
        }

        function test_sixteenths_get_room() {
            load(16)
            // Each note at least two and a half staff spaces: room for the head and its beam.
            verify(staff.barWidth / 16 >= staff.staffSpace * 2.5,
                   "bar " + staff.barWidth + " px for 16 notes at " + staff.staffSpace + " px")
        }

        function test_quarters_keep_their_lines() {
            load(4)
            verify(staff.perSystem >= 4, "only " + staff.perSystem + " bars per line")
        }
    }
}
