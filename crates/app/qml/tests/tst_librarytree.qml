import QtQuick
import QtTest
import Kante
import ".."

// The library's source tree (LH-BIB-17): counts from the data layer, subfolders indented, a
// missing folder said in words, an origin with its sign. Run by scripts/check-video.sh.
Item {
    id: root
    width: 300
    height: 700

    readonly property QtObject texts: QtObject {
        function t(key, args) { return key + (args ? " " + JSON.stringify(args) : "") }
    }
    readonly property var counts: ({
        all: 214, recent: 7,
        origins: { exercise: 152, generated: 3, download: 40, imported: 19, folder: 5 },
        folders: [
            { path: "/m/Band", name: "Band", count: 5, missing: false, cutOff: false,
              children: [{ path: "/m/Band/Rock", name: "Rock", count: 2, children: [] }] },
            { path: "/m/Gone", name: "Gone", count: 0, missing: true, cutOff: false, children: [] }
        ],
        setlists: [{ name: "Friday", count: 2 }]
    })

    LibraryTree {
        id: tree
        width: parent.width
        height: parent.height - 40
        texts: root.texts
        counts: root.counts
    }
    // Something after the tree for Tab to go to.
    Rectangle { id: after; y: parent.height - 30; width: 20; height: 20; activeFocusOnTab: true }
    SignalSpy { id: picked; target: tree; signalName: "picked" }
    SignalSpy { id: unlink; target: tree; signalName: "unlinkRequested" }
    SignalSpy { id: addFolder; target: tree; signalName: "addFolderRequested" }

    TestCase {
        name: "LibraryTree"
        when: windowShown

        function find(key) {
            return tree.entries.filter(function(e) { return e.key === key })[0]
        }

        function test_countsAndSigns() {
            compare(find("all").count, 214)
            compare(find("recent").count, 7)
            compare(find("exercise").count, 152)
            compare(find("exercise").sign, "◆")
            compare(find("generated").sign, "◇")
            compare(find("download").sign, "↓")
            compare(find("imported").sign, "●")
            compare(find("folder:/m/Band").count, 5)
            compare(find("setlist:Friday").count, 2)
        }

        function test_subfolderIsNestedAndTheTopOneCanBeUnlinked() {
            compare(find("folder:/m/Band/Rock").depth, 1)
            verify(!find("folder:/m/Band/Rock").top)
            verify(find("folder:/m/Band").top)
        }

        function test_missingFolderSaysSoInWords() {
            verify(find("folder:/m/Gone").text.indexOf("library.folder.missing") >= 0)
        }

        function test_everyOriginHasAWordNextToItsSign() {
            ["exercise", "generated", "download", "imported"].forEach(function(o) {
                compare(find(o).text, "library.origin." + o)
            })
        }

        function test_clickPicksTheSource() {
            picked.clear()
            tree.positionViewAtBeginning()
            var row = tree.itemAtIndex(0)
            verify(row !== null)
            mouseClick(row, 40, 10)
            compare(picked.count, 1)
            compare(picked.signalArguments[0][0], "all")
        }

        function test_actionsAreEntriesToo() {
            var actions = tree.entries.filter(function(e) { return e.kind === "action" })
            compare(actions.map(function(a) { return a.id }), ["folder", "setlist"])
        }

        // QA 14.11. (K1, P7): every entry was a Tab stop. The tree is one stop now: the arrows
        // move between entries (over the headings), Tab leaves it.
        function test_theTreeIsOneTabStop() {
            tree.positionViewAtBeginning()
            waitForRendering(tree)
            var rowAt = function(i) { return findChild(tree.itemAtIndex(i), "treeRow") }
            rowAt(0).forceActiveFocus()
            keyClick(Qt.Key_Down)
            verify(rowAt(1).activeFocus, "Down goes to the next entry")
            keyClick(Qt.Key_Down)
            verify(rowAt(3).activeFocus, "the heading in between is passed over")
            keyClick(Qt.Key_Up)
            verify(rowAt(1).activeFocus, "Up passes it too")
            keyClick(Qt.Key_Tab)
            verify(after.activeFocus, "Tab leaves the tree")
        }

        // Review #332: an entry scrolled out of the built ones (many folders, a short window)
        // took no Tab, so the tree could not be reached. The tree takes it and hands it on.
        function test_anEntryOutOfViewIsStillTheStop() {
            var last = tree.entries.length - 1
            var height = tree.height
            var buffer = tree.cacheBuffer
            tree.height = 60
            tree.cacheBuffer = 0
            try {
                tree.focusIndex = last
                tree.positionViewAtBeginning()
                waitForRendering(tree)
                verify(tree.itemAtIndex(last) === null, "the last entry is not built")
                after.forceActiveFocus()
                keyClick(Qt.Key_Backtab)
                tryVerify(function() {
                    var cell = tree.itemAtIndex(last)
                    return cell !== null && findChild(cell, "treeRow").activeFocus
                }, 1000, "Backtab reaches the tree's stop entry")
            } finally {
                tree.height = height
                tree.cacheBuffer = buffer
            }
        }
    }
}
