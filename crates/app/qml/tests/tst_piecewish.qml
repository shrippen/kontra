import QtQuick
import QtTest
import org.kde.kirigami as Kirigami
import Kante
import ".."

// "Stück wünschen" (LH-BIB-16): the dialog stays while the provider writes, says so, and
// opens the practice page with the piece, wherever it was opened from. Run by
// scripts/check-video.sh.
Item {
    id: root
    width: 1000
    height: 800

    Component {
        id: windowComponent
        Kirigami.ApplicationWindow {
            width: root.width
            height: root.height
            visible: true
            property string shown: ""
            readonly property QtObject texts: QtObject {
                function t(key, args) { return key }
            }
            readonly property QtObject ctl: QtObject {
                property string llmProvider: "custom"
                property string llmModel: "fake"
                property string llmModelsJson: "[]"
                property bool llmBusy: false
                property bool llmKeyBusy: false
                property string errorText: ""
                signal llmKeyState(string provider, string state, bool ready, string text)
                signal llmKeyError(string provider, string text)
                signal llmPieceOpened()
                function llmProvidersJson() {
                    return JSON.stringify([{ id: "custom", name: "Custom", needsKey: false, defaultModel: "" }])
                }
                function llmCheckKey(provider) { llmKeyState(provider, "not_needed", true, "") }
                function llmWish(text, level) { llmBusy = true; return true }
            }
            function show(page) { shown = page }
            PieceWish { id: wish }
            function dialog() { return wish }
        }
    }

    TestCase {
        name: "PieceWish"
        when: windowShown

        // QA 14.11.: the dialog closed at once, so "Kontra writes the piece …" was never seen,
        // and from the library the finished piece did not open the practice page.
        function test_staysWhileWritingThenOpensPractice() {
            var win = createTemporaryObject(windowComponent, root)
            var dialog = win.dialog()
            dialog.open()
            tryCompare(dialog, "opened", true)
            findChild(dialog, "wishText").text = "Motown in A"
            findChild(dialog, "wishGenerate").clicked()
            verify(win.ctl.llmBusy)
            verify(dialog.visible, "open while the provider writes")
            verify(findChild(dialog, "wishRunning").visible)

            win.ctl.llmBusy = false
            win.ctl.llmPieceOpened()
            tryCompare(dialog, "visible", false)
            compare(win.shown, "practice")
        }

        // Review #327: a failed wish said so only in the note bar, dimmed behind the open dialog.
        function test_aFailureShowsInTheDialog() {
            var win = createTemporaryObject(windowComponent, root)
            var dialog = win.dialog()
            dialog.open()
            tryCompare(dialog, "opened", true)
            findChild(dialog, "wishText").text = "Motown in A"
            findChild(dialog, "wishGenerate").clicked()
            // As the bridge does on `LlmEvent::Failed`: not busy, then the error.
            win.ctl.llmBusy = false
            win.ctl.errorText = "bars: 17 bars, at most 16"
            var callout = findChild(dialog, "wishError")
            verify(callout.visible)
            compare(callout.text, "bars: 17 bars, at most 16")
            verify(dialog.visible, "the wish stays for another try")
            compare(win.shown, "")
        }
    }
}
