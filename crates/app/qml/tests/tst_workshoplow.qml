import QtQuick
import QtTest
import org.kde.kirigami as Kirigami
import Kante
import ".."

// The workshop counts a page as low from 820 px (QA 14.11., J-C12, P4): bars, target tempo,
// blocks and density leave the lane about 60 px at 792 px. Other modes keep 560.
// Run by scripts/check-video.sh.
Item {
    id: root
    width: 1296
    height: 792

    readonly property QtObject texts: QtObject {
        function t(key) { return key }
    }
    property QtObject ctl: QtObject {
        property string mode: "workshop"
        property string view: "combined"
        property bool call: false
    }
    function applicationWindow() { return root }

    Component {
        id: pageComponent
        PracticePage {
            width: 1296
            height: 792
        }
    }

    TestCase {
        name: "WorkshopLow"
        when: windowShown

        function pageIn(mode) {
            root.ctl.mode = mode
            var page = createTemporaryObject(pageComponent, root)
            verify(page, "page created")
            return page
        }

        function test_workshopIsLowAt792() {
            var page = pageIn("workshop")
            verify(page.compact, "compact")
            verify(!page.fretboardRoom, "no fretboard")
            verify(page.lean, "settings folded")
        }

        function test_otherModesKeepTheFretboardAt792() {
            var page = pageIn("song")
            verify(!page.compact, "not compact")
            verify(page.fretboardRoom, "fretboard")
            verify(!page.lean, "settings shown")
        }
    }
}
