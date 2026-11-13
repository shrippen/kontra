import QtQuick
import QtTest
import org.kde.kirigami as Kirigami
import Kante
import ".."

// The provider dialog of "Stück wünschen" (LH-BIB-16): the model list shows which model is
// chosen, and none when none is. Run by scripts/check-video.sh.
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
            readonly property QtObject texts: QtObject {
                function t(key, args) { return key }
            }
            readonly property QtObject ctl: QtObject {
                property string llmProvider: "custom"
                property string llmBaseUrl: "http://127.0.0.1:1/v1"
                property string llmModel: ""
                property string llmModelsJson: "[]"
                property bool llmBusy: false
                property bool llmKeyBusy: false
                signal llmKeyState(string provider, string state, bool ready, string text)
                signal llmKeyError(string provider, string text)
                function llmProvidersJson() {
                    return JSON.stringify([{ id: "custom", name: "Custom", needsKey: false }])
                }
                function llmCheckKey(provider) {}
                function changeLlmModel(model) { llmModel = model }
            }
            LlmSettings { id: settings }
            function dialog() { return settings }
        }
    }

    TestCase {
        name: "LlmSettings"
        when: windowShown

        function openWith(model, models) {
            var win = createTemporaryObject(windowComponent, root)
            win.ctl.llmModel = model
            win.ctl.llmModelsJson = JSON.stringify(models)
            var dialog = win.dialog()
            dialog.open()
            tryCompare(dialog, "opened", true)
            return dialog
        }

        // QA 14.11.: after "Load models" the list showed the first model, the field stayed
        // empty and nothing was chosen until the shown entry was picked again.
        function test_noModelChosenShowsNone() {
            var list = findChild(openWith("", ["fake-bass", "other"]), "llmModels")
            compare(list.currentIndex, -1)
            compare(list.displayText, "llm.settings.model.pick")
        }

        function test_theChosenModelIsShown() {
            var list = findChild(openWith("other", ["fake-bass", "other"]), "llmModels")
            compare(list.currentIndex, 1)
        }
    }
}
