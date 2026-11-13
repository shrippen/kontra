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
    }
}
