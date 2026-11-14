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
            LlmSettings { id: settings }
            function dialog() { return settings }
        }
    }

    // The settings page as far as the group reads it: a scrolled view, the group below others.
    Component {
        id: pageComponent
        Kirigami.ApplicationWindow {
            width: root.width
            height: 300
            visible: true
            property alias view: flick
            property int above: 1000
            readonly property QtObject texts: QtObject { function t(key, args) { return key } }
            readonly property QtObject ctl: QtObject {
                property string llmProvider: "custom"
                property string llmBaseUrl: ""
                property string llmModel: ""
                property string llmModelsJson: "[]"
                property bool llmBusy: false
                property bool llmKeyBusy: false
                property int keyChecks: 0
                signal llmKeyState(string provider, string state, bool ready, string text)
                signal llmKeyError(string provider, string text)
                function llmProvidersJson() {
                    return JSON.stringify([{ id: "custom", name: "Custom", needsKey: false }])
                }
                function llmCheckKey(provider) { keyChecks += 1 }
                function changeLlmModel(model) { llmModel = model }
            }
            Flickable {
                id: flick
                anchors.fill: parent
                contentHeight: column.height
                Column {
                    id: column
                    width: flick.width
                    Item { width: 1; height: above }
                    LlmSettings { width: column.width; view: flick }
                    Item { width: 1; height: 2000 }
                }
            }
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
        // opens would prompt to unlock it for anyone changing only the input. Asked once the
        // group comes into view, however it got there (the tabs' scrollspy skips sections in a
        // tall window, second review of #359).
        function test_theKeyringIsAskedOnceTheGroupShows() {
            var page = createTemporaryObject(pageComponent, root)
            compare(page.ctl.keyChecks, 0)
            page.view.contentY = 900
            tryCompare(page.ctl, "keyChecks", 1)
            page.view.contentY = 0
            page.view.contentY = 900
            compare(page.ctl.keyChecks, 1)
        }

        function test_aGroupInViewAtOnceIsAskedAtOnce() {
            var page = createTemporaryObject(pageComponent, root, { above: 0 })
            tryCompare(page.ctl, "keyChecks", 1)
        }
    }
}
