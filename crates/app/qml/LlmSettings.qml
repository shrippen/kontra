import QtQuick
import QtQuick.Layouts
import QtQuick.Controls as QQC2
import Kante

// LLM provider for "Stück wünschen" (LH-BIB-16): provider, model and the API key, kept in the
// system's own secret store, never in Kontra's settings file. Opened from PieceWish.qml; the
// settings page embeds it too once its own rewrite (another PR) lands.
KanteDialog {
    id: dialog
    objectName: "llmSettings"
    parent: QQC2.Overlay.overlay
    anchors.centerIn: parent
    modal: true
    size: KanteDialog.Size.Medium
    title: texts.t("llm.settings.title")
    standardButtons: QQC2.Dialog.Close

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
    // answer (dialog.ctl.llmKeyBusy disables Save/Delete meanwhile).
    function refreshKeyState() {
        keyState = texts.t("llm.key.checking")
        keyStateId = ""
        ctl.llmCheckKey(ctl.llmProvider)
    }

    onOpened: {
        providers = JSON.parse(ctl.llmProvidersJson())
        models = JSON.parse(ctl.llmModelsJson)
        error = ""
        refreshKeyState()
    }
    // `llmProvider` is `ctl`'s property, not this dialog's: a plain `onXChanged` only fires
    // for properties of the object it is written on.
    Connections {
        target: dialog.ctl
        function onLlmProviderChanged() { dialog.refreshKeyState() }
        function onLlmModelsJsonChanged() { dialog.models = JSON.parse(dialog.ctl.llmModelsJson) }
        function onLlmKeyState(provider, state, ready, text) {
            if (provider !== dialog.ctl.llmProvider) return
            dialog.keyState = text
            dialog.keyStateId = state
        }
        function onLlmKeyError(provider, text) {
            if (provider === dialog.ctl.llmProvider) dialog.error = text
        }
    }

    ColumnLayout {
        anchors.fill: parent
        spacing: KanteStyle.unit(8)

        KanteSettingRow {
            Layout.fillWidth: true
            title: texts.t("llm.settings.provider")
            KanteComboBox {
                Accessible.name: texts.t("llm.settings.provider")
                width: KanteStyle.unit(220)
                model: dialog.providers.map(function(p) { return p.name })
                currentIndex: dialog.providers.findIndex(function(p) { return p.id === dialog.ctl.llmProvider })
                onActivated: function(index) {
                    dialog.ctl.changeLlmProvider(dialog.providers[index].id)
                    dialog.refreshKeyState()
                }
            }
        }

        KanteSettingRow {
            Layout.fillWidth: true
            visible: dialog.isCustom
            title: texts.t("llm.settings.baseurl")
            KanteTextField {
                Layout.fillWidth: true
                Accessible.name: texts.t("llm.settings.baseurl")
                text: dialog.ctl.llmBaseUrl
                onEditingFinished: dialog.ctl.changeLlmBaseUrl(text)
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
                    text: dialog.ctl.llmModel
                    onEditingFinished: dialog.ctl.changeLlmModel(text)
                }
                KanteButton {
                    text: texts.t("llm.settings.model.load")
                    enabled: !dialog.ctl.llmBusy
                    onClicked: dialog.ctl.llmFetchModels()
                }
            }
        }
        KanteComboBox {
            Layout.fillWidth: true
            visible: dialog.models.length > 0
            Accessible.name: texts.t("llm.settings.model")
            model: dialog.models
            onActivated: function(index) {
                modelField.text = dialog.models[index]
                dialog.ctl.changeLlmModel(dialog.models[index])
            }
        }

        KanteSettingRow {
            Layout.fillWidth: true
            visible: dialog.needsKey
            title: texts.t("llm.settings.key")
            hint: dialog.keyState
            RowLayout {
                KanteTextField {
                    id: keyField
                    Layout.fillWidth: true
                    echoMode: TextInput.Password
                    placeholderText: texts.t("llm.settings.key.placeholder")
                    Accessible.name: texts.t("llm.settings.key")
                }
                KanteButton {
                    text: dialog.keyStateId === "stored"
                        ? texts.t("llm.settings.key.replace") : texts.t("llm.settings.key.save")
                    enabled: keyField.text.length > 0 && !dialog.ctl.llmKeyBusy
                    onClicked: {
                        dialog.error = ""
                        if (dialog.ctl.llmSetKey(dialog.ctl.llmProvider, keyField.text))
                            keyField.text = ""
                    }
                }
                KanteButton {
                    text: texts.t("llm.settings.key.delete")
                    emphasis: KanteButton.Emphasis.Quiet
                    visible: dialog.keyStateId === "stored"
                    enabled: !dialog.ctl.llmKeyBusy
                    onClicked: {
                        dialog.error = ""
                        dialog.ctl.llmDeleteKey(dialog.ctl.llmProvider)
                    }
                }
            }
        }
        KanteCallout {
            Layout.fillWidth: true
            visible: dialog.error !== ""
            kind: KanteCallout.Kind.Warning
            text: dialog.error
            dismissible: true
            autoHide: false
            onDismissed: dialog.error = ""
        }
        KanteButton {
            visible: dialog.currentProvider && dialog.currentProvider.docsUrl !== ""
            text: texts.t("llm.settings.key.docs")
            emphasis: KanteButton.Emphasis.Quiet
            onClicked: if (dialog.currentProvider) Qt.openUrlExternally(dialog.currentProvider.docsUrl)
        }
    }
}
