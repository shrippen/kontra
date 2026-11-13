import QtQuick
import QtTest
import org.kde.kirigami as Kirigami
import Kante
import ".."

// The library page as a whole (LH-BIB-17): its head stays at the top, whatever the source
// holds. Run by scripts/check-video.sh.
Item {
    id: root
    width: 1100
    height: 700

    // A library with one piece; `empty` leaves the chosen source without rows.
    property bool empty: false
    readonly property var counts: ({
        all: 1, recent: 0,
        origins: { exercise: 1, generated: 0, download: 0, imported: 0, folder: 0 },
        folders: [{ path: "/m/Gone", name: "Gone", count: 0, missing: true, cutOff: false,
                    children: [] }],
        setlists: []
    })
    readonly property var row: ({
        path: "/x/a.mid", title: "A", artist: "", origin: "exercise", originText: "Übung",
        kind: "MID", bpm: null, lastPlayed: null, lastText: "", result: null, resultText: "",
        missing: false, folder: "", tags: [], group: ""
    })

    Component {
        id: windowComponent
        Kirigami.ApplicationWindow {
            width: root.width
            height: root.height
            visible: true
            readonly property QtObject texts: QtObject {
                function t(key, args) { return key }
            }
            readonly property QtObject ctl: QtObject {
                property string recentJson: "[]"
                property string setlistsJson: "[]"
                property int libraryRevision: 0
                property string trackChoicesJson: "[]"
                property bool qaRun: true
                property bool loaded: false
                property string tagsJson: "[]"
                property string songSearchJson: ""
                function libraryJson(source, sort, group, query) {
                    return JSON.stringify({ counts: root.counts, rows: root.empty ? [] : [root.row],
                                           groups: [] })
                }
                property int rescans: 0
                function libraryRescan() { rescans += 1 }
            }
            function readingPad(width) { return 0 }
            function show(page) {}
            pageStack.initialPage: LibraryPage {}
        }
    }

    TestCase {
        name: "LibraryPage"
        when: windowShown

        function headingOf(page) {
            return findChild(page, "libraryHeading")
        }

        // QA 13.11.: without the table nothing in the column took the height, and the layout
        // centred the head (title, search, buttons) in the middle of the page.
        function test_headStaysAtTheTopOfAnEmptySource() {
            root.empty = false
            var win = createTemporaryObject(windowComponent, root)
            var page = win.pageStack.currentItem
            tryVerify(function() { return headingOf(page) !== null && headingOf(page).height > 0 })
            var top = headingOf(page).mapToItem(page, 0, 0).y

            root.empty = true
            win.ctl.libraryRevision += 1
            wait(50)
            compare(headingOf(page).mapToItem(page, 0, 0).y, top)
        }

        // QA 13.11.: an unmounted folder said "no pieces in this source"; it names the folder
        // as not found and is looked for again when picked.
        function test_aMissingFolderIsSaidAndLookedForAgain() {
            root.empty = true
            var win = createTemporaryObject(windowComponent, root)
            var page = win.pageStack.currentItem
            tryVerify(function() { return findChild(page, "libraryEmpty") !== null })
            var before = win.ctl.rescans
            page.pick("folder:/m/Gone")
            compare(findChild(page, "libraryEmpty").title, "library.folder.gone.title")
            compare(win.ctl.rescans, before + 1)
        }
    }
}
