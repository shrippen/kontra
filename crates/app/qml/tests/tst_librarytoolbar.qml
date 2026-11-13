import QtQuick
import QtTest
import Kante
import ".."

// No function is lost at a narrow width (LH-BIB-19, 200 % scale): the grouping moves below the
// search, the sort gets a choice with every column. Run by scripts/check-video.sh.
Item {
    id: root
    width: 900
    height: 500

    readonly property QtObject texts: QtObject {
        function t(key, args) { return key + (args ? " " + JSON.stringify(args) : "") }
    }
    readonly property QtObject ctl: QtObject {
        readonly property string recentJson: "[]"
    }
    property string sort: "title:asc"

    LibraryToolbar {
        id: bar
        width: parent.width
        texts: root.texts
        groups: ["none", "artist", "folder", "origin"]
        sourceItems: [{ kind: "item", key: "all", text: "All", sign: "" }]
    }
    LibraryTable {
        id: table
        y: 100
        width: parent.width
        height: 300
        ctl: root.ctl
        texts: root.texts
        view: ({ rows: [], groups: [] })
        sort: root.sort
        onSortChosen: function(s) { root.sort = s }
    }
    SignalSpy { id: grouped; target: bar; signalName: "groupChosen" }

    TestCase {
        name: "LibraryNarrow"
        when: windowShown

        function init() {
            bar.slim = false
            table.compact = false
            root.sort = "title:asc"
            grouped.clear()
        }

        function test_wideHasTheGroupingBesideTheSearch() {
            tryCompare(findChild(bar, "groupWide"), "active", true)
            compare(findChild(bar, "groupNarrow").active, false)
        }

        function test_narrowKeepsTheGroupingBelowTheSearch() {
            bar.slim = true
            compare(findChild(bar, "groupWide").active, false)
            var narrow = findChild(bar, "groupNarrow")
            compare(narrow.active, true)
            verify(narrow.item !== null)
            compare(narrow.item.model.length, 4)
            narrow.item.activated(2)
            compare(grouped.count, 1)
            compare(grouped.signalArguments[0][0], "folder")
        }

        function test_narrowKeepsTheSearchOnItsOwnLine() {
            bar.slim = true
            compare(findChild(bar, "searchWide").active, false)
            verify(findChild(bar, "searchNarrow").active)
        }

        function test_narrowShowsTheSourceChoice() {
            bar.slim = true
            verify(findChild(bar, "sourceChoice").visible)
        }

        function test_compactOffersEverySortKey() {
            table.compact = true
            var choice = findChild(table, "sortChoice")
            verify(choice.visible)
            var combo = choice.children[0]
            compare(combo.model.length, 5)
            combo.activated(2)
            compare(root.sort, "tempo:asc")
            combo.activated(3)
            compare(root.sort, "lastPlayed:desc")
        }

        function test_compactCanReverseTheDirection() {
            table.compact = true
            root.sort = "tempo:asc"
            var button = findChild(table, "sortChoice").children[1]
            button.clicked()
            compare(root.sort, "tempo:desc")
        }

        function test_wideHasNoSortChoice() {
            verify(!findChild(table, "sortChoice").visible)
        }
    }
}
