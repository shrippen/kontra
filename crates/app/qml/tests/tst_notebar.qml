import QtQuick
import QtTest

// The note bar under the page (J-B1, QA 27.10.): a message that comes after an earlier one was
// dismissed shows too (before, the bar stayed hidden after the first close). Run by
// scripts/check-video.sh.
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
        function dismissMessage() {
            message = ""
            errorText = ""
        }
        function useOfferedInput() {}
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
