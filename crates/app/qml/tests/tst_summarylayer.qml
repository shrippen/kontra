import QtQuick
import QtTest
import Kante
import ".."

// The summary loads after the page (review #303): loaded, it is still a card of at most
// 640 px in the middle, hides the page behind from screen readers and gives the keyboard back
// to play when closed (QA 28.10., P7). Run by scripts/check-video.sh.
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

    SummaryLayer {
        id: summaryLayer
        anchors.fill: parent
        returnTo: play
        behind: [behindItem]
    }

    TestCase {
        name: "SummaryLayer"
        when: windowShown

        // The loaded sheet: the loader is the layer's child with an item.
        function sheet() {
            for (var i = 0; i < summaryLayer.children.length; i++) {
                var child = summaryLayer.children[i]
                if (child.item !== undefined && child.item !== null) {
                    return child.item
                }
            }
            return null
        }

        function test_loadedSheetKeepsItsPlace() {
            tryVerify(function() { return sheet() !== null }, 3000)
            var card = sheet()
            compare(card.returnTo, play)
            compare(card.behind.length, 1)
            summaryLayer.open()
            verify(card.width <= 640, "width " + card.width)
            fuzzyCompare(card.mapToItem(summaryLayer, 0, 0).x + card.width / 2, summaryLayer.width / 2, 1)
            verify(!behindItem.visible, "page behind still shown to screen readers")
            summaryLayer.close()
            tryVerify(function() { return play.activeFocus }, 2000)
        }
    }
}
