import QtQuick
import QtTest
import Kante
import ".."

// The learn page (LH-LRN-01): its chapters. Run by scripts/check-video.sh.
Item {
    id: root
    width: 1200
    height: 700

    readonly property int chapterCount: 15
    readonly property QtObject texts: QtObject {
        function t(key, args) { return key }
    }
    readonly property QtObject ctl: QtObject {
        property string startCourse: ""
        property string courseFetchJson: ""
        property string courseFolderJson: ""
        property bool qaRun: true
        function learnJson() {
            var chapters = []
            for (var i = 0; i < root.chapterCount; ++i) {
                chapters.push({ id: "c" + i, title: "Chapter " + i, done: 0, total: 2, questions: 0,
                    units: [{ id: "u" + i, title: "Unit", done: false, fits: true }],
                    finale: { kind: "practice", title: "Piece", speeds: [60, 80, 100] } })
            }
            return JSON.stringify({ fits: true, next: null, chapters: chapters, course: "", courses: [] })
        }
    }
    property string checkChapter: ""
    property string lessonId: ""
    property int lessonPage: 0
    function applicationWindow() { return root }
    function readingPad(width) { return 0 }

    Component {
        id: pageComponent
        LearnPage { width: 1200; height: 700 }
    }

    TestCase {
        name: "LearnChapters"
        when: windowShown

        function chapters(item) {
            var n = item.text === "learn.chapter" ? 1 : 0
            for (var i = 0; i < item.children.length; ++i) {
                n += chapters(item.children[i])
            }
            return n
        }

        // N6: all 15 chapters were built before the page's first frame (about 150 ms), the
        // screen shows four. The first come with the page, the others a frame after another.
        function test_laterChaptersFollowThePage() {
            var page = createTemporaryObject(pageComponent, root)
            verify(page !== null)
            verify(chapters(page) < root.chapterCount, "built with the page: " + chapters(page))
            tryCompare(page, "built", root.chapterCount, 3000)
            tryVerify(function() { return chapters(page) === root.chapterCount }, 3000, "all follow")
        }

        // Review #347: a reload (another course, a saved course folder) built all again at once.
        function test_aReloadStartsWithTheFirstChapters() {
            var page = createTemporaryObject(pageComponent, root)
            tryCompare(page, "built", root.chapterCount, 3000)
            page.reload()
            verify(chapters(page) < root.chapterCount, "built after the reload: " + chapters(page))
            tryVerify(function() { return chapters(page) === root.chapterCount }, 3000, "all follow")
        }
    }
}
