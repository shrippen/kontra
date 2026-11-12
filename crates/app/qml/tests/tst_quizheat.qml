import QtQuick
import QtTest
import Kante

// Fretboard quiz overview (LH-LRN-18, review #222): a wrong note is counted at once, and the
// overview showed it only with the next question. It now shows it right away.
Item {
    id: root
    width: 1024
    height: 640

    // QuizPage reads ctl and texts from the application window; keys stand for the texts.
    readonly property QtObject texts: QtObject {
        function t(key) { return key }
    }
    readonly property QtObject ctl: QtObject {
        property string tuningJson: '[{"name":"E"},{"name":"A"},{"name":"D"},{"name":"G"}]'
        property bool lowStringOnTop: false
        property string tunerNote: ""
        property real tunerMidi: 0
        // Wrong notes counted for B on the A string (string 1, fret 2).
        property int misses: 0
        function quizJson() {
            var rows = [0, 1, 2, 3].map(function(s) {
                return [0, 1, 2, 3, 4].map(function(f) {
                    return s === 1 && f === 2 && misses > 0 ? { right: 0, wrong: misses } : null
                })
            })
            return JSON.stringify({ prompt: "quiz", solved: false, score: "", range: { low: 0, high: 4 },
                positions: [], heat: { frets: [0, 1, 2, 3, 4], rows: rows } })
        }
        function quizStart(kind, low, high) { return quizJson() }
        function quizNext() { return quizJson() }
        function quizAnswer(midi) { misses++; return false }
        function startTuner() {}
        function stopTuner() {}
    }
    function applicationWindow() { return root }
    function readingPad(pageWidth) { return 0 }

    Loader {
        id: loader
        anchors.fill: parent
        source: Qt.resolvedUrl("../QuizPage.qml")
    }

    TestCase {
        name: "QuizHeat"
        when: windowShown

        function grid(item) {
            if (item.cells !== undefined && item.rowNames !== undefined) {
                return item
            }
            for (var i = 0; i < item.children.length; i++) {
                var found = grid(item.children[i])
                if (found) {
                    return found
                }
            }
            return null
        }

        function test_wrong_note_shows_at_once() {
            tryVerify(function() { return loader.status === Loader.Ready })
            var heat = grid(loader.item)
            verify(heat !== null, "overview")
            // High string on top: A is the third row.
            compare(heat.cells[2][2], null)

            // C2 three times in a row: a steady wrong note.
            ctl.tunerNote = "C2"
            ctl.tunerMidi = 36.0
            ctl.tunerMidi = 36.1
            ctl.tunerMidi = 36.2
            compare(ctl.misses, 1)
            verify(heat.cells[2][2] !== null, "A string, fret 2 counted")
            compare(heat.cells[2][2].tip, "quiz.heat.tip")
        }
    }
}
