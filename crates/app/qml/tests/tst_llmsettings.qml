import QtQuick
import QtTest
import org.kde.kirigami as Kirigami
import Kante
import ".."

// The provider settings (LH-BIB-16), a group on the settings page: the model list shows which
// model is chosen, and none when none is. Run by scripts/check-video.sh.
Item {
    id: root
    width: 1000
    height: 800

    Component {
        id: windowComponent
        Kirigami.ApplicationWindow {
            id: testWindow
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
                property int keyChecks: 0
                function llmCheckKey(provider) { keyChecks += 1 }
                function changeLlmModel(model) { llmModel = model }
            }
            // The settings page's sticky header, as far as the group reads it.
            readonly property QtObject sticky: QtObject { property int currentIndex: 0 }
            LlmSettings { id: settings; sectionHeader: testWindow.sticky; section: 2 }
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
            return win.dialog()
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
    
        // Review #359: the group sits on the settings page now; asking the keyring as the page
        // opens would prompt to unlock it for anyone changing only the input. Asked once its
        // section shows.
        function test_theKeyringIsAskedOnceTheSectionShows() {
            var win = createTemporaryObject(windowComponent, root)
            compare(win.ctl.keyChecks, 0)
            win.sticky.currentIndex = 2
            compare(win.ctl.keyChecks, 1)
            win.sticky.currentIndex = 0
            win.sticky.currentIndex = 2
            compare(win.ctl.keyChecks, 1)
        }
    }
}
