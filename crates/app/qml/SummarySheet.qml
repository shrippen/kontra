import QtQuick
import QtQuick.Layouts
import QtQuick.Controls as QQC2
import org.kde.kirigami as Kirigami
import Kante

// Evaluation after a run (LH-BEW-03): hit rate, timing tendency, histogram, hardest bars.
// Shown in the page over the notes (not a window dialog), so it is part of screenshots.
KanteCard {
    id: sheet

    readonly property var ctl: applicationWindow().ctl
    readonly property var texts: applicationWindow().texts
    readonly property var summary: ctl.summaryJson !== "" ? JSON.parse(ctl.summaryJson) : null

    signal practiceBar(int bar)

    function open() { visible = true }
    function close() { visible = false }

    visible: false
    barColor: KanteStyle.accentColor
    color: KanteStyle.backgroundColor
    borderColor: KanteStyle.frameColor
    // Sizes come from the content's implicit size; the content is placed, not anchored to fill,
    // so the card's size never feeds back into it.
    readonly property real contentHeight: content.implicitHeight + KanteStyle.unit(32)

    // Scrolls when the screen is lower than the content (beamer at 150 %).
    Flickable {
        id: flick
        x: KanteStyle.unit(16)
        y: KanteStyle.unit(16)
        width: sheet.width - KanteStyle.unit(32)
        height: sheet.height - KanteStyle.unit(32)
        contentWidth: width
        contentHeight: content.implicitHeight
        clip: true
        boundsBehavior: Flickable.StopAtBounds
        QQC2.ScrollBar.vertical: QQC2.ScrollBar { policy: flick.contentHeight > flick.height ? QQC2.ScrollBar.AlwaysOn : QQC2.ScrollBar.AsNeeded }

    ColumnLayout {
        id: content
        width: flick.width
        spacing: KanteStyle.unit(12)

        RowLayout {
            Layout.fillWidth: true
            KanteHeading { text: texts.t("summary.title"); Layout.fillWidth: true }
            KanteButton { text: texts.t("summary.close"); emphasis: KanteButton.Emphasis.Quiet; onClicked: sheet.close() }
        }

        KanteKpi {
            Layout.fillWidth: true
            value: sheet.summary ? sheet.summary.hitRateText : ""
            label: sheet.summary ? sheet.summary.countsText : ""
        }
        QQC2.Label {
            Layout.fillWidth: true
            wrapMode: Text.WordWrap
            text: sheet.summary ? [sheet.summary.tendencyText, sheet.summary.spreadText, sheet.summary.intonationText].filter(function(t) { return t !== "" }).join(" · ") : ""
            color: KanteStyle.textColor
        }
        KanteTimingHistogram {
            Layout.fillWidth: true
            Layout.preferredHeight: KanteStyle.unit(180)
            counts: sheet.summary ? sheet.summary.counts : []
            firstBinMs: sheet.summary ? sheet.summary.firstBinMs : -100
            binMs: sheet.summary ? sheet.summary.binMs : 10
            meanMs: sheet.summary ? sheet.summary.meanOffsetMs : NaN
            earlyText: texts.t("result.early")
            lateText: texts.t("result.late")
            meanText: texts.t("histogram.mean")
            countText: texts.t("histogram.count")
            accessibleName: texts.t("histogram.name")
        }
        KanteSectionLabel {
            text: texts.t("summary.hardest")
            visible: sheet.summary && sheet.summary.hardestBars.length > 0
        }
        Flow {
            Layout.fillWidth: true
            spacing: KanteStyle.unit(8)
            Repeater {
                model: sheet.summary ? sheet.summary.hardestBars : []
                delegate: KanteButton {
                    required property var modelData
                    text: texts.t("summary.practice.bar", { bar: modelData.number })
                    onClicked: sheet.practiceBar(modelData.index)
                }
            }
        }

        // Per note (LH-BEW-01): the hardest notes, and on request every note, with timing and cents.
        KanteSectionLabel {
            text: texts.t("summary.hardest.notes")
            visible: sheet.summary && sheet.summary.hardestNotes.length > 0
        }
        Repeater {
            model: sheet.summary ? sheet.summary.hardestNotes : []
            delegate: noteRow
        }
        KanteButton {
            id: allNotes
            visible: sheet.summary && sheet.summary.notes.length > 0
            checkable: true
            emphasis: KanteButton.Emphasis.Quiet
            text: texts.t("summary.all.notes", { n: sheet.summary ? sheet.summary.notes.length : 0 })
        }
        Repeater {
            model: allNotes.checked && sheet.summary ? sheet.summary.notes : []
            delegate: noteRow
        }
    }
}

    Component {
        id: noteRow
        RowLayout {
            required property var modelData
            Layout.fillWidth: true
            spacing: KanteStyle.unit(8)
            Accessible.role: Accessible.StaticText
            Accessible.name: modelData.text
            KanteNoteMark {
                Layout.preferredWidth: KanteStyle.unit(14)
                Layout.preferredHeight: KanteStyle.unit(14)
                noteState: modelData.state
            }
            QQC2.Label {
                Layout.fillWidth: true
                wrapMode: Text.WordWrap
                text: modelData.text
                color: KanteStyle.textColor
            }
        }
    }
}
