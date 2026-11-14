import QtQuick
import QtTest
import Kante
import ".."

// The run summary keeps the keyboard like a dialog (QA 28.10., P7): Tab stays in it, the page
// behind is hidden from screen readers, and closing puts the focus on play. Run by
// scripts/check-video.sh.
Item {
    id: root
    width: 800
    height: 900

    // SummarySheet reads ctl and texts from the application window; keys stand for the texts,
    // the histogram's in English.
    readonly property QtObject texts: QtObject {
        readonly property var map: ({ "histogram.bin": "%1 to %2 %3: %4", "histogram.count": "%1 notes" })
        function t(key) { return map[key] || key }
    }
    readonly property QtObject ctl: QtObject {
        property string summaryJson: JSON.stringify({
            hitRateText: "80 %", countsText: "8/10", probe: null, next: null,
            pocketText: "", tendencyText: "", spreadText: "", intonationText: "",
            counts: [1, 2, 3], firstBinMs: -10, binMs: 10, meanOffsetMs: 0,
            techniques: [], untoldText: "", hardestNotes: [], notes: [],
            hardestBars: [{ number: 2 }, { number: 3 }],
            observation: { solid: false, text: "observe.text" }, recalibrateText: ""
        })
        function termJson(topic) { return "" }
        property bool playing: false
        // Bars 5–8 weak (LH-FOR-06).
        function songMap() {
            return JSON.stringify({ values: [1, 1, 0.9, 1, 0.4, 0.3, 0.5, 0.2], labels: ["1", "2", "3", "4", "5", "6", "7", "8"],
                                    tips: [], loop: null, played: true })
        }
    }
    function applicationWindow() { return root }
    // The page a button asked for.
    property string shownPage: ""
    function show(name) { shownPage = name }
    // The routing after opening a piece lives in Main.qml; a page only asks for it.
    property var openedPieces: []
    function openPiece(path) { openedPieces = openedPieces.concat([path]) }

    Column {
        id: behindItem
        KanteButton { id: play; text: "play" }
        KanteButton {
            id: outside
            text: "outside"
            // Each time the focus touched it.
            property int touched: 0
            onActiveFocusChanged: {
                if (activeFocus) {
                    touched++
                }
            }
        }
    }

    SummarySheet {
        id: sheet
        width: 600
        height: 800
        returnTo: play
        behind: [behindItem]
    }

    TestCase {
        name: "SummarySheet"
        when: windowShown

        // The summary as the sheet starts; each test that changes it gets it back, also when it
        // fails (review #197).
        property string initialSummary
        function initTestCase() { initialSummary = root.ctl.summaryJson }
        function cleanup() {
            sheet.close()
            root.ctl.summaryJson = initialSummary
        }

        function focused() { return root.Window.activeFocusItem }
        function inSheet(item) {
            for (var p = item; p; p = p.parent) {
                if (p === sheet) {
                    return true
                }
            }
            return false
        }

        // The first item below `item` that has a function `name`.
        function having(item, name) {
            if (typeof item[name] === "function") {
                return item
            }
            for (var i = 0; i < item.children.length; i++) {
                var found = having(item.children[i], name)
                if (found) {
                    return found
                }
            }
            return null
        }

        // Screen readers hear each bar's range and count in the app's language (QA 28.10. B9).
        function test_bar_names_translated() {
            var hist = having(sheet, "binName")
            verify(hist !== null)
            compare(hist.binName(2), "+10 to +20 ms: 3 notes")
        }

        // The first visible item below `item` showing `text`.
        function showing(item, text) {
            if (item.visible && item.text === text) {
                return item
            }
            for (var i = 0; i < item.children.length; i++) {
                var found = showing(item.children[i], text)
                if (found) {
                    return found
                }
            }
            return null
        }

        // QA 14.11. (J-C12): no note hit drew an empty chart with "mean 0 ms", a time
        // nobody played.
        // Opening the next piece goes through the window's routing, so a piece that needs a
        // bass track choice leads to the track question (audit 14.11., Fund 5). Before: the sheet
        // called ctl.openFile itself and nothing was routed.
        function test_nextPieceIsRouted() {
            var before = root.ctl.summaryJson
            var summary = JSON.parse(before)
            summary.next = { path: "/x/next.mid", level: "L1", title: "Next" }
            root.ctl.summaryJson = JSON.stringify(summary)
            root.openedPieces = []
            try {
                sheet.open()
                var play = showing(sheet, "summary.next.play")
                verify(play !== null)
                play.clicked()
                compare(root.openedPieces, ["/x/next.mid"])
            } finally {
                root.ctl.summaryJson = before
            }
        }

        function test_noHitNoTimingChart() {
            var before = root.ctl.summaryJson
            var summary = JSON.parse(before)
            summary.counts = [0, 0, 0]
            root.ctl.summaryJson = JSON.stringify(summary)
            try {
                sheet.open()
                verify(!findChild(sheet, "timingChart").visible)
                sheet.close()
            } finally {
                root.ctl.summaryJson = before
            }
            sheet.open()
            verify(findChild(sheet, "timingChart").visible, "with hits it shows")
            sheet.close()
        }

        // One observation in words above the details (LH-BEW-09).
        function test_observation_shown() {
            sheet.open()
            verify(showing(sheet, "observe.text") !== null)
            verify(showing(sheet, "summary.recalibrate.button") === null)
            sheet.close()
        }

        // The latency hint stands alone where "solid" would contradict it, with its button
        // (LH-EIN-14, review #195).
        function test_recalibrate_alone() {
            var summary = JSON.parse(root.ctl.summaryJson)
            summary.observation = null
            summary.recalibrateText = "recal.text"
            root.ctl.summaryJson = JSON.stringify(summary)
            sheet.open()
            verify(showing(sheet, "recal.text") !== null)
            verify(showing(sheet, "summary.recalibrate.button") !== null)
        }

        // A string out of tune is named with the way to the tuner (LH-TUN-05).
        function test_detuned_string() {
            var summary = JSON.parse(root.ctl.summaryJson)
            summary.detunedText = "detuned.text"
            root.ctl.summaryJson = JSON.stringify(summary)
            sheet.open()
            verify(showing(sheet, "detuned.text") !== null)
            var button = showing(sheet, "summary.detuned.button")
            verify(button !== null)
            root.shownPage = ""
            mouseClick(button)
            compare(root.shownPage, "tuner")
            tryVerify(function() { return !sheet.opened })
        }

        // A lesson's goal in words (LH-LRN-20); pure instruction shows no percent.
        function test_lesson_goal() {
            var summary = JSON.parse(root.ctl.summaryJson)
            summary.goal = { text: "goal.text", met: false }
            root.ctl.summaryJson = JSON.stringify(summary)
            sheet.open()
            verify(showing(sheet, "goal.text") !== null)
            verify(showing(sheet, "80 %") !== null)
            sheet.close()

            summary.goal = { none: true }
            root.ctl.summaryJson = JSON.stringify(summary)
            sheet.open()
            verify(showing(sheet, "summary.goal.none") !== null)
            verify(showing(sheet, "80 %") === null)
            sheet.close()
        }

        // J-B3: dragging over bars 5–8 of the song map practises them (LH-FOR-06).
        function test_song_map_sets_the_loop() {
            sheet.open()
            var strip = having(sheet, "indexAt")
            verify(strip !== null)
            tryVerify(function() { return strip.visible && strip.width > 0 })
            var practised = []
            var take = function(from, to) { practised = [from, to] }
            sheet.practiceBars.connect(take)
            var step = strip.barWidth + strip.spacing
            mousePress(strip, step * 4 + 1, 10)
            mouseMove(strip, step * 7 + 1, 10)
            mouseRelease(strip, step * 7 + 1, 10)
            sheet.practiceBars.disconnect(take)
            compare(practised, [4, 7])
        }

        function test_trap_and_return() {
            outside.touched = 0
            sheet.open()
            tryVerify(function() { return inSheet(focused()) }, 2000, "focus in the sheet")
            var start = focused()
            verify(!behindItem.visible, "page behind hidden")

            // Round the sheet by Tab: back at the start without leaving it.
            for (var i = 0; i < 20 && (i === 0 || focused() !== start); i++) {
                keyClick(Qt.Key_Tab)
                verify(inSheet(focused()), "Tab " + i + " left the sheet to " + focused())
            }
            verify(focused() === start, "Tab came round to " + start)
            keyClick(Qt.Key_Backtab, Qt.ShiftModifier)
            verify(inSheet(focused()), "Shift+Tab left the sheet to " + focused())
            compare(outside.touched, 0)

            sheet.close()
            verify(behindItem.visible, "page behind back")
            tryVerify(function() { return play.activeFocus }, 2000, "play focused, focus on " + focused())
        }
    }
}
