import QtQuick
import QtTest
import org.kde.kirigami as Kirigami
import Kante
import ".."

// Learn page at P4's screen (W3 09.11.): the button row did not wrap and made the page wider
// than the window; "Kurs von Adresse …" and the intro were cut off on the right. German texts,
// the longest. Run by scripts/check-video.sh.
Item {
    id: root
    // P4's page: 1280×720 at 150 % → 853 logical pixels, minus the sidebar 680, with a font
    // 1.5 times the test's: about 26 grid units, Kontra's narrowest window (Main.qml). In grid
    // units, so the check holds with any font.
    width: Kirigami.Units.gridUnit * 26
    height: Kirigami.Units.gridUnit * 30

    // The German catalog (scripts/check-video.sh allows the file read).
    readonly property QtObject texts: Texts {
        source: {
            const request = new XMLHttpRequest()
            request.open("GET", Qt.resolvedUrl("../../../appcore/i18n/de.json"), false)
            request.send()
            return request.responseText
        }
    }
    readonly property QtObject ctl: QtObject {
        property string courseFetchJson: ""
        property string courseFolderJson: ""
        property string startCourse: ""
        function learnJson() {
            return JSON.stringify({ fits: true, next: { id: "l1", title: "Der erste Ton" }, chapters: [], courses: [] })
        }
    }
    property string checkChapter: ""
    function show(name) {}
    function glossary(term) {}
    function applicationWindow() { return root }
    // As Main.qml: text pages keep one centred reading width.
    readonly property int readingWidth: KanteStyle.unit(1040)
    function readingPad(pageWidth) {
        return Math.max(KanteStyle.unit(16), (pageWidth - readingWidth) / 2)
    }

    Loader {
        id: loader
        anchors.fill: parent
        source: Qt.resolvedUrl("../LearnPage.qml")
    }

    TestCase {
        name: "LearnWidth"
        when: windowShown

        // Visible items under `item` whose right edge lies beyond `edge` (in `flick`).
        function overflow(item, flick, edge) {
            var found = []
            for (var i = 0; i < item.children.length; i++) {
                var child = item.children[i]
                if (!child.visible || child.width <= 0) {
                    continue
                }
                var right = child.mapToItem(flick, child.width, 0).x
                if (right > edge + 0.5) {
                    found.push((child.text || child.toString()) + " ends at " + Math.round(right))
                }
                found = found.concat(overflow(child, flick, edge))
            }
            return found
        }

        function test_fits_p4_width() {
            var page = loader.item
            verify(page !== null, "LearnPage loads")
            waitForRendering(page)
            verify(texts.t("courses.address") !== "courses.address", "German catalog loaded")

            var flick = page.flickable
            var cut = overflow(flick.contentItem, flick, flick.width)
            compare(cut.length, 0, "wider than the page (" + Math.round(flick.width) + "): " + cut.join("; "))
        }

        // Narrow: the plain margin and one column. Wide: the content stays at the reading width,
        // centred, and the chapters go in two columns (D2, D6).
        function test_reading_width_and_columns() {
            var page = loader.item
            waitForRendering(page)
            compare(page.leftPadding, KanteStyle.unit(16))
            compare(page.rightPadding, KanteStyle.unit(16))
            verify(!page.twoColumns, "one column at P4's width")

            var narrowWidth = root.width
            root.width = Kirigami.Units.gridUnit * 90
            waitForRendering(page)
            fuzzyCompare(page.leftPadding, (page.width - root.readingWidth) / 2, 1)
            fuzzyCompare(page.rightPadding, page.leftPadding, 0.01)
            verify(page.width - page.leftPadding - page.rightPadding <= root.readingWidth + 1)
            verify(page.twoColumns, "two columns when wide")
            root.width = narrowWidth
        }
    }
}
