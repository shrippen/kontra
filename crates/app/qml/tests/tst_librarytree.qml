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
        anchors.fill: parent
        texts: root.texts
        counts: root.counts
    }
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
    }
}
