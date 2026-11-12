import QtQuick
import QtQuick.Layouts
import QtQuick.Controls as QQC2
import Kante

// Pegel-Assistent (LH-EIN-03/04): shows the advice Core derives from the level of the last
// seconds (`levelState`, `suggestLaptop`, see appcore `level.rs`); this file only words it.
ColumnLayout {
    id: advice

    /// Only shown while sound is expected (playing, or the setup's level step).
    property bool listening: true
    /// Only the advice, without what gain is: a low window keeps its room for the notes (S12).
    property bool brief: false

    enum Fit {
        Grow,
        Steady
    }
    /// Steady: one message at a time in a slot of two lines that keeps its height, so the notes
    /// above stay where they are while playing (owner 12.11.). The whole text is the tooltip.
    property int fit: LevelAdvice.Fit.Grow
    signal useLaptopJack()

    readonly property var ctl: applicationWindow().ctl
    readonly property var texts: applicationWindow().texts
    readonly property string state: listening ? ctl.levelState : ""
    readonly property bool suggestLaptop: listening && ctl.suggestLaptop
    readonly property bool steady: fit === LevelAdvice.Fit.Steady
    readonly property bool warning: state !== "" && state !== "good"
    // Steady shows the most pressing: the laptop jack, then the level, else the input's name.
    readonly property string message: suggestLaptop ? "laptop" : warning ? "level" : "input"
    // Above any count of lines the label could wrap to.
    readonly property int allLines: 1000

    spacing: KanteStyle.unit(6)
    Layout.preferredHeight: steady ? Math.max(lines.height * 2, KanteStyle.heightMedium) : -1
    clip: steady
    FontMetrics { id: lines }

    // Which input listens, next to its level (LH-EIN-15); faults show in the note bar.
    InputStatus {
        id: inputLine
        Layout.fillWidth: true
        visible: shown && (!advice.steady || advice.message === "input")
        part: InputStatus.Part.Line
        ctl: advice.ctl
        texts: advice.texts
    }

    QQC2.Label {
        Layout.fillWidth: true
        wrapMode: Text.WordWrap
        maximumLineCount: advice.steady ? 2 : advice.allLines
        elide: Text.ElideRight
        visible: advice.warning && (!advice.steady || advice.message === "level")
        text: advice.state === "" ? "" : advice.texts.t("advice." + advice.state)
        color: KanteStyle.mutedTextColor
        Accessible.role: Accessible.StaticText
        Accessible.name: text
        HoverHandler { id: levelHover }
        QQC2.ToolTip.visible: advice.steady && levelHover.hovered && truncated
        QQC2.ToolTip.text: text
    }
    QQC2.Label {
        Layout.fillWidth: true
        wrapMode: Text.WordWrap
        visible: !advice.brief && !advice.steady && (advice.state === "quiet" || advice.state === "loud" || advice.state === "clip")
        text: advice.texts.t("advice.gain")
        color: KanteStyle.mutedTextColor
        font: KanteStyle.smallFont
    }
    RowLayout {
        visible: advice.suggestLaptop
        Layout.fillWidth: true
        QQC2.Label {
            Layout.fillWidth: true
            wrapMode: Text.WordWrap
            maximumLineCount: advice.steady ? 2 : advice.allLines
            elide: Text.ElideRight
            text: advice.texts.t("advice.laptop")
            color: KanteStyle.textColor
            HoverHandler { id: laptopHover }
            QQC2.ToolTip.visible: advice.steady && laptopHover.hovered && truncated
            QQC2.ToolTip.text: text
        }
        KanteButton {
            text: advice.texts.t("advice.laptop.use")
            onClicked: advice.useLaptopJack()
        }
    }
    // A layout with no child shown takes no room: the slot keeps it empty too.
    Item {
        visible: advice.steady
        Layout.fillWidth: true
        Layout.fillHeight: true
    }
}
