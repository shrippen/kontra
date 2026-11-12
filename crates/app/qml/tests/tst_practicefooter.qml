import QtQuick
import QtQuick.Layouts
import QtTest
import Kante
import ".."

// Under the notes nothing grows or shrinks once the player presses play (owner 12.11.: the
// notes moved at "Play" and again while playing). The level replaced the idle line, "input
// connected", the level advice and the finger came and went, each a line more or less.
// Run by scripts/check-video.sh.
Item {
    id: root
    width: 1100
    height: 400

    readonly property QtObject texts: QtObject {
        function t(key, args) { return key === "fretboard.marker" ? "%1 %2 %3" : key + (args ? " " + JSON.stringify(args) : "") }
    }
    readonly property QtObject ctl: QtObject {
        property bool playing: false
        property bool fretboardShown: true
        property bool lowStringOnTop: false
        property bool leftHanded: false
        property string connection: "jack"
        property string midiPortName: ""
        property string levelState: ""
        property bool suggestLaptop: false
        property string inputState: "idle"
        property string inputDetail: ""
        property real peakDb: -60
        property real rmsDb: -60
        property bool clipped: false
        function changeFretboard(on) { fretboardShown = on }
        function changeConnection(c) {}
    }
    function applicationWindow() { return root }

    // The practice page's part the footer reads.
    QtObject {
        id: practice
        property bool presenting: false
        property bool lean: false
        property bool compact: false
        property bool fretboardRoom: true
        property var stringNames: ["G", "D", "A", "E"]
        property var nextNote: ({ string: 2, fret: 3, finger: 0, position: 1 })
        property int handPosition: 1
        property var shift: null
        property var signs: []
        property bool hasChordSymbols: false
    }

    ColumnLayout {
        id: column
        anchors.fill: parent
        spacing: 0
        Item {
            id: notes
            Layout.fillWidth: true
            Layout.fillHeight: true
        }
        PracticeFooter {
            id: footer
            Layout.fillWidth: true
            practice: practice
        }
    }

    TestCase {
        name: "PracticeFooter"
        when: windowShown

        function init() {
            ctl.playing = false
            ctl.fretboardShown = true
            ctl.levelState = ""
            ctl.suggestLaptop = false
            ctl.inputState = "idle"
            practice.nextNote = { string: 2, fret: 3, finger: 0, position: 1 }
            practice.handPosition = 1
            practice.shift = null
        }

        // Plays a run step by step; the notes keep their height at every step.
        function checkSteady() {
            waitForRendering(column)
            var before = notes.height
            var steps = [
                function() { ctl.playing = true; ctl.inputState = "connected"; ctl.inputDetail = "Scarlett Solo" },
                function() { ctl.levelState = "silent" },
                function() { ctl.levelState = "quiet" },
                function() { ctl.levelState = "good" },
                function() { ctl.suggestLaptop = true },
                function() { practice.nextNote = { string: 1, fret: 5, finger: 3, position: 5 }; practice.handPosition = 5 },
                function() { practice.shift = { position: 7, bar: 12 } },
                function() { practice.nextNote = null; practice.handPosition = 0; practice.shift = null },
                function() { ctl.playing = false; ctl.levelState = ""; ctl.suggestLaptop = false; ctl.inputState = "idle" }
            ]
            for (var i = 0; i < steps.length; i++) {
                steps[i]()
                waitForRendering(column)
                compare(notes.height, before, "step " + i)
            }
        }

        function test_steadyBesideFretboard() {
            checkSteady()
        }

        function test_steadyWithoutFretboard() {
            ctl.fretboardShown = false
            checkSteady()
        }
    }
}
