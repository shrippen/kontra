import QtQuick
import QtQuick.Controls as QQC2
import QtTest
import Kante
import ".."

// The two dialogs behind "Add pieces" (LH-BIB-17): the generator takes style, level, key, length
// and a variant to the bridge; a folder is linked, imported or imported as a setlist. Run by
// scripts/check-video.sh.
Item {
    id: root
    width: 900
    height: 700

    readonly property QtObject texts: QtObject {
        function t(key, args) { return key }
    }
    readonly property QtObject ctl: QtObject {
        property var calls: []
        function generatePiece(style, level, key, bpm, seed, length) {
            calls.push(["piece", style, level, key, seed, length])
            return true
        }
        function linkFolder(url) { calls.push(["link", url]) }
        function importFolder(url, asSetlist) { calls.push(["import", url, asSetlist]) }
    }

    GenerateDialog { id: generate; ctl: root.ctl; texts: root.texts }
    AddFolderDialog { id: folder; ctl: root.ctl; texts: root.texts }
    SignalSpy { id: made; target: generate; signalName: "generated" }

    TestCase {
        name: "LibraryDialogs"
        when: windowShown

        function init() {
            root.ctl.calls = []
            made.clear()
        }

        function test_generateTakesTheChoicesAndAVariant() {
            generate.style = 3
            generate.level = 2
            generate.key = 9
            generate.length = 2
            generate.variant = 0
            generate.open()
            tryCompare(generate, "visible", true)
            generate.variant += 1
            generate.variant += 1
            generate.accept()
            compare(root.ctl.calls.length, 1)
            compare(root.ctl.calls[0], ["piece", "funk", "hard", 9, 2, "long"])
            compare(made.count, 1)
        }

        // QA 13.11.: the reset button appeared where "Another variant" was, so a second click on
        // the same spot went back to the plain piece.
        function test_anotherVariantStaysUnderThePointer() {
            generate.variant = 0
            generate.open()
            tryCompare(generate, "opened", true)
            var other = findChild(generate, "otherVariant")
            verify(other !== null)
            var spot = other.mapToItem(root, other.width / 2, other.height / 2)
            mouseClick(root, spot.x, spot.y)
            // The layout settles between two clicks of a hand.
            wait(100)
            mouseClick(root, spot.x, spot.y)
            compare(generate.variant, 2)
            generate.close()
        }

        // The same in a narrow window, where the row puts its controls on the left (review #325).
        function test_anotherVariantStaysUnderThePointerWhenNarrow() {
            root.width = 330
            generate.variant = 0
            generate.open()
            tryCompare(generate, "opened", true)
            wait(50)
            var other = findChild(generate, "otherVariant")
            var spot = other.mapToItem(root, other.width / 2, other.height / 2)
            mouseClick(root, spot.x, spot.y)
            wait(100)
            mouseClick(root, spot.x, spot.y)
            compare(generate.variant, 2)
            generate.close()
            root.width = 900
        }

        function test_aFirstPieceIsThePlainOne() {
            generate.style = 0
            generate.level = 1
            generate.key = 4
            generate.length = 1
            generate.variant = 0
            generate.open()
            generate.accept()
            compare(root.ctl.calls[0], ["piece", "ballad", "medium", 4, 0, "normal"])
        }

        function test_folderIsLinkedByDefault() {
            folder.ask("file:///home/x/Bass%20Noten")
            tryCompare(folder, "visible", true)
            compare(folder.folderName, "Bass Noten")
            folder.accept()
            compare(root.ctl.calls, [["link", "file:///home/x/Bass%20Noten"]])
        }

        function test_folderCanBeImportedOrMadeASetlist() {
            folder.ask("file:///home/x/Gig")
            folder.way = 1
            folder.accept()
            folder.ask("file:///home/x/Gig")
            folder.way = 2
            folder.accept()
            compare(root.ctl.calls, [["import", "file:///home/x/Gig", false], ["import", "file:///home/x/Gig", true]])
        }

        function test_askStartsWithLinkAgain() {
            folder.way = 2
            folder.ask("file:///home/x/Gig")
            compare(folder.way, 0)
            folder.close()
        }
    }
}
