import QtQuick
import QtTest
import Kante

// Setup step "level and tuning" at 200 % (1920×1080 → 960×540, QA 28.10. B4, P7): note, cents
// and turn direction lay below the footer, and no key reached them. Tab now stops at the tuner
// and the view follows it. Run by scripts/check-video.sh.
Item {
    id: root
    width: 960
    height: 540

    // SetupPage reads ctl and texts from the application window; keys stand for the texts.
    readonly property QtObject texts: QtObject {
        readonly property var map: ({})
        // The step counter takes two numbers ("Step %1 of %2").
        function t(key) { return key === "setup.state.step" ? "%1/%2" : key }
    }
    readonly property QtObject ctl: QtObject {
        property string inputDevicesJson: "[]"
        property string inputDevice: ""
        property bool laptopJack: false
        property real peakDb: -12
        property real rmsDb: -18
        property bool clipped: false
        property string levelState: "good"
        property bool suggestLaptop: false
        property bool tunerActive: false
        property string tunerNote: "E1"
        property real tunerCents: -12
        property bool calibrating: false
        property string calibrationText: ""
        property int calibrationBeat: -1
        property real latencyMs: 0
        property string instrument: "bass"
        property int strings: 4
        property bool fretless: false
        property string connection: "interface"
        property string midiPortName: ""
        property string midiPortsJson: "[]"
        function refreshDevices() {}
        function startTuner() { tunerActive = true }
        function stopTuner() { tunerActive = false }
        function chooseInput(id) {}
        function changeConnection(c) {}
        function chooseMidi(p) {}
        function changeInstrument(i) {}
        function chooseInstrument(i) {}
        function startCalibration() {}
        function finishSetup() {}
        function changeSimpleView(on) {}
        function learnJson() { return "{}" }
    }
    function show(name) {}
    function applicationWindow() { return root }

    Loader {
        id: loader
        anchors.fill: parent
        source: Qt.resolvedUrl("../SetupPage.qml")
    }

    TestCase {
        name: "SetupLevel"
        when: windowShown

        // The tuner gauge: the item whose accessible name is the tuner's.
        function gauge(item) {
            if (item.Accessible.role === Accessible.Indicator && item.accessibleName === "tuner.name") {
                return item
            }
            for (var i = 0; i < item.children.length; i++) {
                var found = gauge(item.children[i])
                if (found) {
                    return found
                }
            }
            return null
        }

        function test_tab_reaches_tuner() {
            var page = loader.item
            verify(page !== null, "SetupPage loads")
            page.step = page.steps.indexOf("level")
            var tuner = gauge(page.flickable.contentItem)
            verify(tuner !== null)
            waitForRendering(page)

            // At 200 % the tuner's lower half lies below the view.
            var flick = page.flickable
            var top = tuner.mapToItem(flick, 0, 0).y
            verify(top + tuner.height > flick.height, "tuner already in view; the test needs a low window")

            // Tab from the top until the tuner has the focus (a bounded walk).
            page.forceActiveFocus(Qt.TabFocusReason)
            for (var i = 0; i < 20 && !tuner.activeFocus; i++) {
                keyClick(Qt.Key_Tab)
            }
            verify(tuner.activeFocus, "Tab never reached the tuner")

            // The view followed: the whole tuner (note, cents, hint) is visible.
            tryVerify(function() {
                var y = tuner.mapToItem(flick, 0, 0).y
                return y >= 0 && y + tuner.height <= flick.height
            }, 1000, "tuner not scrolled into view")
        }
    }
}
