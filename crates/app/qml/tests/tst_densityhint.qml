import QtQuick
import QtTest
import Kante
import ".."

// The density suggestion comes and goes only while stopped (review #302: after a loop's pass
// it came while playing and moved the notes). Run by scripts/check-video.sh.
Item {
    id: root
    width: 800
    height: 300

    readonly property QtObject texts: QtObject {
        function t(key) { return key }
    }
    readonly property QtObject ctl: QtObject {
        property bool playing: false
        property int densitySuggestion: 0
        property string densityText: "fewer notes?"
        function changeDensity(d, how) {}
        function declineDensity() { densitySuggestion = 0 }
    }
    function applicationWindow() { return root }

    DensityHint {
        id: hint
        width: parent.width
    }

    TestCase {
        name: "DensityHint"
        when: windowShown

        function test_onlyWhileStopped() {
            verify(!hint.visible)
            ctl.playing = true
            ctl.densitySuggestion = 50
            verify(!hint.visible, "came while playing")
            ctl.playing = false
            verify(hint.visible, "not shown after the run")
            ctl.playing = true
            verify(hint.visible, "left at play")
            ctl.playing = false
            ctl.declineDensity()
            verify(!hint.visible, "stayed after declining")
        }
    }
}
