import QtQuick
import QtTest
import org.kde.kirigami as Kirigami
import Kante
import ".."

// A bundled exercise opens through the window's routing, so one that needs a bass track choice
// leads to the track question like every other way to open a piece (audit 14.11., Fund 5).
// Run by scripts/check-video.sh.
Item {
    id: root
    width: 1100
    height: 800

    property var openedPieces: []

    Component {
        id: windowComponent
        Kirigami.ApplicationWindow {
            width: root.width
            height: root.height
            visible: true
            property int exerciseLevel: -1
            readonly property QtObject texts: QtObject {
                function t(key, args) { return key }
            }
            readonly property QtObject ctl: QtObject {
                property string startSection: ""
                function termJson(topic) { return JSON.stringify({ id: topic, title: topic, text: topic }) }
                function exercisesJson() {
                    return JSON.stringify([{
                        level: "L1", mastered: 0,
                        items: [{ path: "/x/e1.mid", title: "E1", goal: "g", best: -1,
                                  mastered: false, goalTopic: "beat" }]
                    }])
                }
            }
            function readingPad(width) { return 0 }
            function show(page) {}
            function openPiece(path) { root.openedPieces = root.openedPieces.concat([path]) }
            pageStack.initialPage: ExercisePage {}
        }
    }

    TestCase {
        name: "ExerciseOpen"
        when: windowShown

        function row(item) {
            if (item.visible && item.text === "E1") {
                return item
            }
            for (var i = 0; i < item.children.length; i++) {
                var found = row(item.children[i])
                if (found) {
                    return found
                }
            }
            return null
        }

        function test_exerciseIsRouted() {
            root.openedPieces = []
            var win = createTemporaryObject(windowComponent, root)
            var page = win.pageStack.currentItem
            tryVerify(function() { return row(page) !== null })
            row(page).clicked()
            compare(root.openedPieces, ["/x/e1.mid"])
        }
    }
}
