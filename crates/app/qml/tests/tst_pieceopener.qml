import QtQuick
import QtTest
import ".."

// Where opening a piece leads (Main.qml's routing): to practice when it opened, to the library
// when it did not and a bass track must be chosen, nowhere otherwise. Run by
// scripts/check-video.sh.
Item {
    id: root
    width: 400
    height: 300

    property var shown: []
    property bool opens: false
    property string choices: "[]"
    property string where: ""

    readonly property QtObject ctl: QtObject {
        readonly property string trackChoicesJson: root.choices
        function openFile(path) { return root.opens }
        function openNext() { return root.opens }
    }

    Component {
        id: openerComponent
        PieceOpener {
            ctl: root.ctl
            current: root.where
            onShowRequested: name => root.shown = root.shown.concat([name])
        }
    }

    TestCase {
        name: "PieceOpener"
        when: windowShown

        function init() {
            root.shown = []
            root.opens = false
            root.choices = "[]"
            root.where = ""
        }

        function test_aTrackToChooseLeadsToTheLibrary() {
            var opener = createTemporaryObject(openerComponent, root)
            root.choices = '[{"name": "Bass"}]'
            compare(opener.openPiece("/x.mid"), false)
            compare(root.shown, ["library"])
        }

        function test_noTrackToChooseStays() {
            var opener = createTemporaryObject(openerComponent, root)
            compare(opener.openPiece("/x.mid"), false)
            compare(root.shown, [])
        }

        function test_theLibraryStaysWhereItIs() {
            var opener = createTemporaryObject(openerComponent, root)
            root.choices = '[{"name": "Bass"}]'
            root.where = "library"
            compare(opener.openPiece("/x.mid"), false)
            compare(root.shown, [])
        }

        function test_anOpenedPieceLeadsToPractice() {
            var opener = createTemporaryObject(openerComponent, root)
            root.opens = true
            root.choices = '[{"name": "Bass"}]'
            var steps = []
            compare(opener.openPiece("/x.mid", function() { steps.push("prepare") },
                                     function() { steps.push("after") }), true)
            compare(root.shown, ["practice"])
            compare(steps, ["prepare", "after"])
        }

        function test_practiceKeepsItsPage() {
            var opener = createTemporaryObject(openerComponent, root)
            root.opens = true
            root.where = "practice"
            compare(opener.openPiece("/x.mid"), true)
            compare(root.shown, [])
        }

        function test_nextWithATrackToChooseLeadsToTheLibrary() {
            var opener = createTemporaryObject(openerComponent, root)
            root.choices = '[{"name": "Bass"}]'
            opener.openNext()
            compare(root.shown, ["library"])
        }

        function test_nextWithoutChoicesStays() {
            var opener = createTemporaryObject(openerComponent, root)
            opener.openNext()
            compare(root.shown, [])
        }

        function test_nextThatOpenedStays() {
            var opener = createTemporaryObject(openerComponent, root)
            root.opens = true
            root.choices = '[{"name": "Bass"}]'
            opener.openNext()
            compare(root.shown, [])
        }
    }
}
