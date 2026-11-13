import QtQuick
import QtQuick.Layouts
import QtQuick.Controls as QQC2
import Kante

// "Stück wünschen" (LH-BIB-16): a wish in words plus a level; an LLM writes a piece plan,
// Kontra validates and renders it, then opens it like any generated exercise. Opened from
// ExercisePage next to the song generator.
KanteDialog {
    id: dialog
    objectName: "pieceWish"
    parent: QQC2.Overlay.overlay
    anchors.centerIn: parent
    modal: true
    size: KanteDialog.Size.Medium
    title: texts.t("wish.title")
    standardButtons: QQC2.Dialog.Close

    readonly property var ctl: applicationWindow().ctl
    readonly property var texts: applicationWindow().texts
    readonly property var levels: ["easy", "medium", "hard"]
    property int level: 1
    property bool hasProvider: false

    // The key never answers synchronously (it can live behind a slow or locked keyring); the
    // dialog shows "checking" (dialog.ctl.llmKeyBusy) until `llmKeyState` names the answer.
    function refreshProvider() {
        hasProvider = false
        var providers = JSON.parse(ctl.llmProvidersJson())
        var chosen = providers.some(function(p) { return p.id === ctl.llmProvider })
        if (chosen) ctl.llmCheckKey(ctl.llmProvider)
    }

    onOpened: {
        wishField.text = ""
        refreshProvider()
    }
    Connections {
        target: dialog.ctl
        function onLlmProviderChanged() { dialog.refreshProvider() }
        function onLlmKeyState(provider, text) {
            if (provider === dialog.ctl.llmProvider)
                dialog.hasProvider = text !== dialog.texts.t("llm.key.missing")
        }
    }

    ColumnLayout {
        anchors.fill: parent
        spacing: KanteStyle.unit(8)

        QQC2.Label {
            Layout.fillWidth: true
            visible: dialog.ctl.llmKeyBusy
            text: dialog.texts.t("llm.key.checking")
        }

        KanteCallout {
            Layout.fillWidth: true
            visible: !dialog.hasProvider && !dialog.ctl.llmKeyBusy
            kind: KanteCallout.Kind.Info
            text: dialog.texts.t("wish.no.provider")
            autoHide: false
            KanteButton {
                text: dialog.texts.t("wish.settings.link")
                onClicked: settingsDialog.open()
            }
        }

        QQC2.Label {
            Layout.fillWidth: true
            wrapMode: Text.WordWrap
            color: KanteStyle.mutedTextColor
            text: dialog.texts.t("wish.hint")
        }

        // No KanteTextField takes several lines; a plain styled TextArea instead (lessons,
        // agent.md): Kante colors set directly so it reads in both themes.
        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: KanteStyle.unit(84)
            color: KanteStyle.backgroundColor
            border.color: KanteStyle.tint2Color
            radius: KanteStyle.unit(4)
            QQC2.ScrollView {
                anchors.fill: parent
                anchors.margins: KanteStyle.unit(4)
                QQC2.TextArea {
                    id: wishField
                    wrapMode: TextEdit.Wrap
                    color: KanteStyle.textColor
                    placeholderTextColor: KanteStyle.mutedTextColor
                    placeholderText: dialog.texts.t("wish.placeholder")
                    Accessible.name: dialog.texts.t("wish.title")
                    background: null
                }
            }
        }

        KanteSettingRow {
            Layout.fillWidth: true
            title: dialog.texts.t("wish.level")
            KanteSegmented {
                Accessible.name: dialog.texts.t("wish.level")
                model: dialog.levels.map(function(l) { return dialog.texts.t("exercises.level." + l) })
                currentIndex: dialog.level
                onActivated: function(index) { dialog.level = index }
            }
        }

        RowLayout {
            Layout.fillWidth: true
            QQC2.Label {
                Layout.fillWidth: true
                visible: dialog.ctl.llmBusy
                text: dialog.texts.t("wish.running")
            }
            KanteButton {
                text: dialog.texts.t("wish.generate")
                emphasis: KanteButton.Emphasis.Primary
                enabled: dialog.hasProvider && wishField.text.trim().length > 0
                    && !dialog.ctl.llmBusy && !dialog.ctl.llmKeyBusy
                onClicked: {
                    if (dialog.ctl.llmWish(wishField.text, dialog.levels[dialog.level]))
                        dialog.close()
                }
            }
        }
    }

    LlmSettings { id: settingsDialog }
}
