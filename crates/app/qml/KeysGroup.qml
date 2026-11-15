import QtQuick
import QtQuick.Layouts
import QtQuick.Controls as QQC2
import Kante

// All keyboard shortcuts in one place (LH-NF-07); a group of the settings page.
KanteSettingGroup {
    title: applicationWindow().texts.t("keys.title")
    countText: ""

    GridLayout {
        Layout.fillWidth: true
        columns: 2
        columnSpacing: KanteStyle.unit(16)
        rowSpacing: KanteStyle.unit(4)
        Repeater {
            model: [
                ["Space / K / PgDn", "keys.play"], ["Pos1 / Home / PgUp", "keys.rewind"], ["L", "keys.loop"],
                ["[ / ,", "keys.loopstart"], ["] / .", "keys.loopend"],
                ["M", "keys.metronome"], ["C", "keys.countin"], ["X", "keys.mixer"], ["+ / −", "keys.speed"],
                ["N", "keys.next"], ["1–4", "keys.mode"], ["Alt+1–5", "keys.view"], ["Ctrl+0–9", "keys.pages"], ["F5 / Esc", "keys.present"]
            ]
            delegate: RowLayout {
                Layout.columnSpan: 2
                spacing: KanteStyle.unit(16)
                QQC2.Label {
                    Layout.preferredWidth: KanteStyle.unit(150)
                    text: modelData[0]
                    font: KanteStyle.monoFont(KanteStyle.defaultFont.pointSize, false)
                    color: KanteStyle.strongTextColor
                }
                QQC2.Label {
                    Layout.fillWidth: true
                    wrapMode: Text.WordWrap
                    text: applicationWindow().texts.t(modelData[1])
                    color: KanteStyle.textColor
                }
            }
        }
    }
    // Foot switch (LH-MOD-07): any pedal that types keys works, no driver needed.
    QQC2.Label {
        Layout.fillWidth: true
        wrapMode: Text.WordWrap
        text: applicationWindow().texts.t("keys.footswitch")
        color: KanteStyle.mutedTextColor
    }
}
