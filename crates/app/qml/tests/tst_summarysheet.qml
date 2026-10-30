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
    }
    function applicationWindow() { return root }

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
