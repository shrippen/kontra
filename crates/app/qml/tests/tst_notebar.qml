import QtQuick
import QtTest

// The note bar under the page (J-B1, QA 27.10.): a message that comes after an earlier one was
// dismissed shows too (before, the bar stayed hidden after the first close). A failed input
// stays with "Erneut versuchen" (LH-EIN-15). Run by scripts/check-video.sh.
Item {
    id: root
    width: 640
    height: 480

    QtObject {
        id: fakeCtl
        property string message: ""
        property string errorText: ""
        property string offerInput: ""
        property string offerText: ""
        property string inputState: "off"
        property string inputName: ""
        property int retries: 0
        function dismissMessage() {
            message = ""
            errorText = ""
        }
        function useOfferedInput() {}
        function retryInput() { retries++ }
    }

    QtObject {
        id: fakeTexts
        function t(key) { return key }
    }

    Loader {
        id: loader
        width: parent.width
        // Required properties go in with the source.
        Component.onCompleted: setSource(Qt.resolvedUrl("../NoteBar.qml"), { ctl: fakeCtl, texts: fakeTexts })
    }

    TestCase {
        name: "NoteBar"
        when: windowShown

        function test_keeps_a_failed_input_with_retry() {
            var bar = loader.item
            fakeCtl.inputState = "failed"
            compare(bar.visible, true)
            var retry = findChild(bar, "inputRetry")
            verify(retry !== null)
            verify(retry.visible)
            mouseClick(retry)
            compare(fakeCtl.retries, 1)

            // Connected shows where the level is, not in the bar.
            fakeCtl.inputState = "connected"
            compare(bar.visible, false)
            fakeCtl.inputState = "off"
        }

        function test_shows_each_new_message() {
            var bar = loader.item
            verify(bar !== null)
            compare(bar.visible, false)

            fakeCtl.errorText = "broken file"
            compare(bar.visible, true)

            fakeCtl.dismissMessage()
            compare(bar.visible, false)

            fakeCtl.message = "3 imported"
            compare(bar.visible, true)
        }
    }
}
