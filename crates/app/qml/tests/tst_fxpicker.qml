import QtQuick
import QtQuick.Controls as QQC2
import QtTest
import Kante
import ".."

// The effect picker is a dialog (owner 12.11.): Enter adds and closes, Escape closes without
// a change, and a new opening adds nothing on its own (review #295: Enter added the last
// effect again). Run by scripts/check-video.sh.
Item {
    id: root
    width: 1000
    height: 900

    // Mixer reads ctl and texts from the application window; keys stand for the texts.
    readonly property QtObject texts: QtObject {
        function t(key) { return key }
    }
    readonly property QtObject ctl: QtObject {
        property bool monitor: true
        property string effectsJson: JSON.stringify({
            kinds: [{ key: "overdrive", name: "Overdrive", group: "distortion" },
                    { key: "fuzz", name: "Fuzz", group: "distortion" }],
            plugins: [], groups: [{ key: "distortion", name: "Distortion", count: 2 }],
            presets: [], slots: [], full: false, adding: "", needsRescan: false,
            empty: "", fullText: "", latencyText: "", scan: { running: false, text: "", broken: [] }
        })
        // Each "add" with its key.
        property var added: []
        function editEffects(action, index, key, value) {
            if (action === "add") {
                added = added.concat([key])
            }
        }
    }
    function applicationWindow() { return root }

    Mixer {
        id: mixer
        width: root.width
    }

    TestCase {
        name: "FxPicker"
        when: windowShown

        function openPicker() {
            mouseClick(findChild(mixer, "fxOpen"))
            var dialog = findChild(mixer, "fxPicker")
            tryCompare(dialog, "opened", true)
            return dialog
        }

        function init() {
            root.ctl.added = []
        }

        function test_enterAddsTheMatchAndCloses() {
            var dialog = openPicker()
            keyClick(Qt.Key_F)
            keyClick(Qt.Key_U)
            keyClick(Qt.Key_Return)
            compare(root.ctl.added, ["fuzz"])
            tryCompare(dialog, "visible", false)
        }

        function test_escapeAddsNothing() {
            var dialog = openPicker()
            keyClick(Qt.Key_F)
            keyClick(Qt.Key_Escape)
            tryCompare(dialog, "visible", false)
            compare(root.ctl.added, [])
        }

        function test_reopenedEnterAddsNothing() {
            var dialog = openPicker()
            keyClick(Qt.Key_F)
            keyClick(Qt.Key_U)
            keyClick(Qt.Key_Return)
            tryCompare(dialog, "visible", false)
            dialog = openPicker()
            compare(findChild(mixer, "fxSearch").text, "")
            keyClick(Qt.Key_Return)
            compare(root.ctl.added, ["fuzz"])
        }
    }
}
