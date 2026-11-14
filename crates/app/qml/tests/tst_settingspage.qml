import QtQuick
import QtTest
import org.kde.kirigami as Kirigami
import Kante
import ".."

// Settings page: the provider of "Stück wünschen" (LH-BIB-16) has a section of its own, which
// the wish dialog's link jumps to. Run by scripts/check-video.sh.
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
            function readingPad(w) { return 16 }
            readonly property QtObject ctl: QtObject {
                property string llmProvider: "custom"
                property string llmBaseUrl: ""
                property string llmModel: ""
                property string llmModelsJson: "[]"
                property bool llmBusy: false
                property bool llmKeyBusy: false
                property bool showMixer: false
                property string startSection: ""
                property string inputDevicesJson: "[]"
                property string outputDevicesJson: "[]"
                property string stemsJson: "[]"
                signal llmKeyState(string provider, string state, bool ready, string text)
                signal llmKeyError(string provider, string text)
                function llmProvidersJson() {
                    return JSON.stringify([{ id: "custom", name: "Custom", needsKey: false }])
                }
                function llmCheckKey(provider) {}
                function refreshDevices() {}
                function licencesJson() { return "[]" }
                function updateNotice(a) { return "{}" }
                function cachesJson() { return "[]" }
            }
            SettingsPage { id: settings }
            function page() { return settings }
        }
    }

    TestCase {
        name: "SettingsPageLlm"
        when: windowShown

        function test_hasTheProviderSection() {
            var win = createTemporaryObject(windowComponent, root)
            var page = win.page()
            verify(findChild(page, "llmSettings"), "provider group on the page")
            verify(page.sectionIndex("llm") >= 0, "section tab")
        }

        function test_showSectionScrollsThere() {
            var win = createTemporaryObject(windowComponent, root)
            var page = win.page()
            page.showSection("llm")
            tryVerify(function() { return page.flickable.contentY > 0 })
        }
    }
}
