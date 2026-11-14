import QtQuick
import QtQuick.Layouts
import QtQuick.Controls as QQC2
import Kante

// LLM provider for "Stück wünschen" (LH-BIB-16): provider, model and the API key, kept in the
// system's own secret store, never in Kontra's settings file. A group of the settings page
// (section "llm"); the wish dialog's link leads there.
KanteSettingGroup {
    id: group
    objectName: "llmSettings"
    title: texts.t("llm.settings.title")
    countText: texts.t(count === 1 ? "settings.group.count.one" : "settings.group.count", { n: count })

    readonly property var ctl: applicationWindow().ctl
    readonly property var texts: applicationWindow().texts
    property var providers: []
    property var models: []
    property string keyState: ""
    // The state's fixed name (`llmKeyState`'s `state`), never the translated text.
    property string keyStateId: ""
    property string error: ""

    readonly property var currentProvider: {
        for (var i = 0; i < providers.length; i++) {
            if (providers[i].id === ctl.llmProvider) return providers[i]
        }
        return null
    }
    readonly property bool isCustom: ctl.llmProvider === "custom"
    readonly property bool needsKey: currentProvider ? currentProvider.needsKey : true

    // The key never answers synchronously (it can live behind a slow or locked keyring); the
    // row shows "checking" (`llm.key.checking`) until `llmKeyState`/`llmKeyError` names the
    // answer (group.ctl.llmKeyBusy disables Save/Delete meanwhile).
    function refreshKeyState() {
        keyState = texts.t("llm.key.checking")
        keyStateId = ""
        ctl.llmCheckKey(ctl.llmProvider)
    }

    // The keyring can ask to be unlocked: it is asked once this group's section shows in the
    // page's sticky header (`sectionHeader`, `section`), not whenever the settings open.
    property var sectionHeader: null
    property int section: -1
    property bool keyAsked: false
    readonly property bool sectionShown: sectionHeader === null || sectionHeader.currentIndex === section
    onSectionShownChanged: askKeyOnce()
    function askKeyOnce() {
        if (!sectionShown || keyAsked) {
            return
        }
        keyAsked = true
        refreshKeyState()
    }

    Component.onCompleted: {
        providers = JSON.parse(ctl.llmProvidersJson())
        models = JSON.parse(ctl.llmModelsJson)
        error = ""
        askKeyOnce()
    }
    // `llmProvider` is `ctl`'s property, not this group's: a plain `onXChanged` only fires
    // for properties of the object it is written on.
    Connections {
        target: group.ctl
        function onLlmProviderChanged() { group.refreshKeyState() }
        function onLlmModelsJsonChanged() { group.models = JSON.parse(group.ctl.llmModelsJson) }
        function onLlmKeyState(provider, state, ready, text) {
            if (provider !== group.ctl.llmProvider) return
            group.keyState = text
            group.keyStateId = state
        }
        function onLlmKeyError(provider, text) {
            if (provider === group.ctl.llmProvider) group.error = text
        }
    }

    KanteSettingRow {
        Layout.fillWidth: true
        title: texts.t("llm.settings.provider")
        KanteComboBox {
            Accessible.name: texts.t("llm.settings.provider")
            width: KanteStyle.unit(220)
            model: group.providers.map(function(p) { return p.name })
            currentIndex: group.providers.findIndex(function(p) { return p.id === group.ctl.llmProvider })
            onActivated: function(index) {
                group.ctl.changeLlmProvider(group.providers[index].id)
                group.refreshKeyState()
            }
        }
    }

    KanteSettingRow {
        Layout.fillWidth: true
        visible: group.isCustom
        title: texts.t("llm.settings.baseurl")
        KanteTextField {
            Layout.fillWidth: true
            Accessible.name: texts.t("llm.settings.baseurl")
            text: group.ctl.llmBaseUrl
            onEditingFinished: group.ctl.changeLlmBaseUrl(text)
        }
    }

    KanteSettingRow {
        Layout.fillWidth: true
        title: texts.t("llm.settings.model")
        RowLayout {
            KanteTextField {
                id: modelField
                Layout.fillWidth: true
                placeholderText: texts.t("llm.settings.model.placeholder")
                Accessible.name: texts.t("llm.settings.model")
                text: group.ctl.llmModel
                onEditingFinished: group.ctl.changeLlmModel(text)
            }
            KanteButton {
                text: texts.t("llm.settings.model.load")
                enabled: !group.ctl.llmBusy
                onClicked: group.ctl.llmFetchModels()
            }
        }
    }
    KanteComboBox {
        objectName: "llmModels"
        Layout.fillWidth: true
        visible: group.models.length > 0
        Accessible.name: texts.t("llm.settings.model")
        model: group.models
        // The chosen model, or none with a prompt: the first entry is no choice (QA 14.11.).
        currentIndex: group.models.indexOf(group.ctl.llmModel)
        displayText: currentIndex < 0 ? texts.t("llm.settings.model.pick") : currentText
        onActivated: function(index) {
            modelField.text = group.models[index]
            group.ctl.changeLlmModel(group.models[index])
        }
    }

    KanteSettingRow {
        Layout.fillWidth: true
        visible: group.needsKey
        title: texts.t("llm.settings.key")
        hint: group.keyState
        RowLayout {
            KanteTextField {
                id: keyField
                Layout.fillWidth: true
                echoMode: TextInput.Password
                placeholderText: texts.t("llm.settings.key.placeholder")
                Accessible.name: texts.t("llm.settings.key")
            }
            KanteButton {
                text: group.keyStateId === "stored"
                    ? texts.t("llm.settings.key.replace") : texts.t("llm.settings.key.save")
                enabled: keyField.text.length > 0 && !group.ctl.llmKeyBusy
                onClicked: {
                    group.error = ""
                    if (group.ctl.llmSetKey(group.ctl.llmProvider, keyField.text))
                        keyField.text = ""
                }
            }
            KanteButton {
                text: texts.t("llm.settings.key.delete")
                emphasis: KanteButton.Emphasis.Quiet
                visible: group.keyStateId === "stored"
                enabled: !group.ctl.llmKeyBusy
                onClicked: {
                    group.error = ""
                    group.ctl.llmDeleteKey(group.ctl.llmProvider)
                }
            }
        }
    }
    KanteCallout {
        Layout.fillWidth: true
        visible: group.error !== ""
        kind: KanteCallout.Kind.Warn
        text: group.error
        dismissible: true
        autoHide: false
        onDismissed: group.error = ""
    }
    KanteButton {
        visible: group.currentProvider && group.currentProvider.docsUrl !== ""
        text: texts.t("llm.settings.key.docs")
        emphasis: KanteButton.Emphasis.Quiet
        onClicked: if (group.currentProvider) Qt.openUrlExternally(group.currentProvider.docsUrl)
    }
}
