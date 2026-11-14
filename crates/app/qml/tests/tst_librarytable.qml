import QtQuick
import QtQuick.Layouts
import QtTest
import Kante
import ".."

// The library's table (LH-BIB-19): a second click on a column reverses it, a new column starts
// in its own direction, the direction is in the title as a sign, groups have a heading with a
// count, a row opens the piece. Run by scripts/check-video.sh.
Item {
    id: root
    width: 1000
    height: 500

    readonly property QtObject texts: QtObject {
        function t(key, args) { return key + (args ? " " + JSON.stringify(args) : "") }
    }
    readonly property QtObject ctl: QtObject {
        readonly property string recentJson: "[]"
        function tagsJson() { return "[]" }
        function setTags(path, json) {}
    }
    property string sort: "title:asc"
    property var view: ({
        rows: [
            { id: "/a.mid", path: "/a.mid", title: "Alpha", artist: "Amy", format: "MID", origin: "imported",
              bpm: 100, lastPlayed: null, bestPercent: 80, missing: false, tags: [], group: "amy" },
            { id: "/b.mid", path: "/b.mid", title: "Beta", artist: "Amy", format: "GP", origin: "download",
              bpm: null, lastPlayed: null, bestPercent: null, missing: true, tags: ["slap"], group: "amy" },
            { id: "/c.mid", path: "/c.mid", title: "Gamma", artist: "", format: "MID", origin: "exercise",
              bpm: 90, lastPlayed: null, bestPercent: null, missing: false, tags: [], group: "" }
        ],
        groups: [{ key: "amy", title: "Amy", count: 2 }, { key: "", title: "No artist", count: 1 }]
    })

    LibraryTable {
        id: table
        anchors.fill: parent
        ctl: root.ctl
        texts: root.texts
        view: root.view
        sort: root.sort
        onSortChosen: function(s) { root.sort = s }
    }
    SignalSpy { id: opened; target: table; signalName: "openPiece" }

    TestCase {
        name: "LibraryTable"
        when: windowShown

        function init() { root.sort = "title:asc" }

        function test_secondClickReversesAndNewColumnStartsInItsOwnDirection() {
            table.sortBy("title")
            compare(root.sort, "title:desc")
            table.sortBy("title")
            compare(root.sort, "title:asc")
            table.sortBy("lastPlayed")
            compare(root.sort, "lastPlayed:desc")
            table.sortBy("tempo")
            compare(root.sort, "tempo:asc")
        }

        function test_directionIsASignInTheTitle() {
            var title = table.columns.filter(function(c) { return c.id === "title" })[0]
            compare(table.columnTitle(title), "library.col.title ▲")
            root.sort = "title:desc"
            compare(table.columnTitle(title), "library.col.title ▼")
            var tempo = table.columns.filter(function(c) { return c.id === "tempo" })[0]
            compare(table.columnTitle(tempo), "library.col.tempo")
        }

        function test_groupHeadingHasTheCount() {
            compare(table.groupText("amy"), "Amy · 2")
            compare(table.groupText(""), "No artist · 1")
        }

        // QA 13.11.: "last played 13.11.25" did not fit the column and ended in "…"; the cell
        // shows the day alone, the screen reader keeps the sentence.
        function test_lastCellIsShortTheNameIsWhole() {
            var old = new Date(2025, 10, 13).getTime() / 1000
            compare(table.lastShort(old), new Date(old * 1000).toLocaleDateString(Qt.locale(), Locale.ShortFormat))
            compare(table.lastShort(Date.now() / 1000), "library.last.short.today")
            compare(table.lastPlayed(Date.now() / 1000), "library.last.today")
            compare(table.lastShort(0), "")
        }

        function test_narrowDropsTempoAndLast() {
            table.compact = true
            compare(table.columns.map(function(c) { return c.id }), ["title", "origin", "result"])
            table.compact = false
            compare(table.columns.length, 5)
        }

        function test_clickOnARowOpensIt() {
            opened.clear()
            var list = findChild(table, "rows")
            verify(list !== null)
            waitForRendering(list)
            var row = findChild(list.itemAtIndex(0), "row")
            verify(row !== null)
            mouseClick(row, 60, 20)
            compare(opened.count, 1)
            compare(opened.signalArguments[0][0], "/a.mid")
        }

        // QA 14.11. (K1, P7): every row and its "…" were Tab stops, 300 for 152 exercises.
        // The rows are one stop now; the arrows move inside, Tab goes on to the row's "…" and
        // then out of the table.
        function test_rowsAreOneTabStop() {
            var list = findChild(table, "rows")
            waitForRendering(list)
            var rowOf = function(i) { return findChild(list.itemAtIndex(i), "row") }
            var moreOf = function(i) { return findChild(list.itemAtIndex(i), "more") }
            rowOf(0).forceActiveFocus()
            keyClick(Qt.Key_Down)
            verify(rowOf(1).activeFocus, "Down goes to the next row")
            keyClick(Qt.Key_Up)
            verify(rowOf(0).activeFocus, "Up comes back")
            keyClick(Qt.Key_Down)
            keyClick(Qt.Key_Tab)
            verify(moreOf(1).activeFocus, "Tab goes to the row's own menu")
            keyClick(Qt.Key_Tab)
            verify(!rowOf(2).activeFocus && !rowOf(0).activeFocus && !moreOf(2).activeFocus,
                   "then out of the table, not to the next row")
        }

        // The list takes Tab itself (below): Backtab from the row must still leave the table,
        // not land on the list, which hands it back to the row.
        function test_backtabLeavesTheTable() {
            var list = findChild(table, "rows")
            table.focusRow(1)
            verify(findChild(list.itemAtIndex(1), "row").activeFocus)
            keyClick(Qt.Key_Backtab)
            wait(50)
            verify(!list.activeFocus, "Backtab leaves the list")
        }

        // Review #334: keys the tag field does not take (arrows, page keys) reached the list,
        // which took the focus out of the field.
        function test_keysStayInTheTagField() {
            var list = findChild(table, "rows")
            table.focusRow(0)
            table.tagsPath = "/a.mid"
            try {
                table.focusTagsRequested()
                var field = list.Window.activeFocusItem
                verify(field !== null && field.objectName !== "row", "the tag field has the focus")
                keyClick(Qt.Key_PageDown)
                compare(list.Window.activeFocusItem, field, "Page down stays in the field")
                keyClick(Qt.Key_Down)
                compare(list.Window.activeFocusItem, field, "Down stays in the field")
            } finally {
                table.tagsPath = ""
            }
        }

        // Review #332, round 2: a focused row scrolled away by wheel or scroll bar pulled the
        // view back to it.
        function test_scrollingAwayFromTheFocusStays() {
            var many = []
            for (var i = 0; i < 150; i++) {
                many.push({ id: "/" + i + ".mid", path: "/" + i + ".mid", title: "Piece " + i, artist: "",
                            format: "MID", origin: "imported", bpm: null, lastPlayed: null,
                            bestPercent: null, missing: false, tags: [], group: "" })
            }
            var old = root.view
            root.view = { rows: many, groups: [] }
            try {
                var list = findChild(table, "rows")
                waitForRendering(list)
                table.focusRow(2)
                list.contentY = 3000
                wait(50)
                compare(list.contentY, 3000, "the view stays where it was scrolled")
                // Review #332, round 3: the page keys work from there too.
                keyClick(Qt.Key_End)
                verify(findChild(list.itemAtIndex(149), "row").activeFocus, "End goes to the last row")
                list.contentY = 0
                wait(50)
                keyClick(Qt.Key_PageUp)
                verify(findChild(list.itemAtIndex(139), "row").activeFocus, "Page up from the last row")
            } finally {
                root.view = old
            }
        }

        // Review #332: with the stop row scrolled out of the built rows (search, sort, wheel),
        // Tab passed the table by. The list takes Tab itself and hands it to that row.
        function test_aRowOutOfViewIsStillTheStop() {
            var many = []
            for (var i = 0; i < 150; i++) {
                many.push({ id: "/" + i + ".mid", path: "/" + i + ".mid", title: "Piece " + i, artist: "",
                            format: "MID", origin: "imported", bpm: null, lastPlayed: null,
                            bestPercent: null, missing: false, tags: [], group: "" })
            }
            var old = root.view
            root.view = { rows: many, groups: [] }
            try {
                var list = findChild(table, "rows")
                waitForRendering(list)
                table.focusIndex = 60
                list.positionViewAtBeginning()
                waitForRendering(list)
                verify(list.itemAtIndex(60) === null, "row 60 is not built")
                list.forceActiveFocus()
                tryVerify(function() {
                    var line = list.itemAtIndex(60)
                    return line !== null && findChild(line, "row").activeFocus
                }, 1000, "the list hands the focus to row 60")
            } finally {
                root.view = old
            }
        }
    }
}
