import QtQuick
import QtTest
import org.kde.kirigami as Kirigami
import Kante
import ".."

// "Next piece" on the practice page opens through the window like every other way to open a
// piece, so an unclear bass track leads to the track question in the library (audit follow-up
// to #351). Run by scripts/check-video.sh.
Item {
    id: root
    width: 1296
    height: 792

    property int windowOpens: 0
    property int directOpens: 0

    readonly property QtObject texts: QtObject {
        function t(key) { return key }
    }
    property QtObject ctl: QtObject {
        property string mode: "song"
        property string view: "combined"
        property bool call: false
        property string nextTitle: "Next"
        function openNext() { root.directOpens += 1; return false }
    }
    // Stands in for Main.qml's window.
    function applicationWindow() { return root }
    function openNext() { windowOpens += 1 }

    Component {
        id: pageComponent
        PracticePage {
            width: 1296
            height: 792
        }
    }

    TestCase {
        name: "PracticeNext"
        when: windowShown

        function test_nextPieceOpensThroughTheWindow() {
            root.windowOpens = 0
            root.directOpens = 0
            var page = createTemporaryObject(pageComponent, root)
            var button = findChild(page, "nextPiece")
            verify(button, "button")
            button.clicked()
            compare(root.directOpens, 0)
            compare(root.windowOpens, 1)
        }

        function test_theNKeyOpensThroughTheWindow() {
            root.windowOpens = 0
            root.directOpens = 0
            var page = createTemporaryObject(pageComponent, root)
            page.forceActiveFocus()
            keyClick(Qt.Key_N)
            compare(root.directOpens, 0)
            compare(root.windowOpens, 1)
        }
    }
}
