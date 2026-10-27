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
    // The keyboard starts in the sheet, on "close" (QA 27.10., P7).
    onVisibleChanged: {
        if (visible) {
            closeButton.forceActiveFocus(Qt.TabFocusReason)
        }
    }
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
        QQC2.ScrollBar.vertical: QQC2.ScrollBar { id: vbar; policy: flick.contentHeight > flick.height ? QQC2.ScrollBar.AlwaysOn : QQC2.ScrollBar.AsNeeded }

    ColumnLayout {
        id: content
        // Room for the scroll bar: on a beamer it covered "Close" (QA 13.10.).
        width: flick.width - (flick.contentHeight > flick.height ? vbar.width + KanteStyle.unit(8) : 0)
        spacing: KanteStyle.unit(12)

        RowLayout {
            Layout.fillWidth: true
            KanteHeading { text: texts.t("summary.title"); Layout.fillWidth: true }
            KanteButton { id: closeButton; text: texts.t("summary.close"); emphasis: KanteButton.Emphasis.Quiet; onClicked: sheet.close() }
        }

        KanteKpi {
            Layout.fillWidth: true
            value: sheet.summary ? sheet.summary.hitRateText : ""
            label: sheet.summary ? sheet.summary.countsText : ""
        }

        // Blind probe (DIDAKTIK I1): without help next to the workshop with notes, in figures and
        // a sentence (not colour alone).
        readonly property var probe: sheet.summary ? sheet.summary.probe : null
        KanteSectionLabel {
            visible: content.probe !== null
            text: texts.t("probe.title")
        }
        // Without a workshop run to compare the hit rate above says it all; only the sentence.
        RowLayout {
            Layout.fillWidth: true
            visible: content.probe !== null && content.probe["with"] !== ""
            spacing: KanteStyle.unit(32)
            KanteKpi {
                value: content.probe ? content.probe["with"] : ""
                label: texts.t("probe.with.label")
            }
            KanteKpi {
                value: content.probe ? content.probe.without : ""
                label: texts.t("probe.without.label")
            }
            Item { Layout.fillWidth: true }
        }
        QQC2.Label {
            Layout.fillWidth: true
            visible: content.probe !== null
            wrapMode: Text.WordWrap
            text: content.probe ? content.probe.text : ""
            color: KanteStyle.textColor
        }

        // The exercise sits now (P1): the next one of the plan, one click away.
        KanteHintCard {
            Layout.fillWidth: true
            id: nextCard
            readonly property var upcoming: sheet.summary ? sheet.summary.next : null
            visible: upcoming !== null
            tier: KanteHintCard.Tier.Green
            tierText: texts.t("summary.next.tier")
            meta: upcoming ? upcoming.level : ""
            titleText: upcoming ? upcoming.title : ""
            text: texts.t("summary.next.text")
            KanteButton {
                text: texts.t("summary.next.play")
                emphasis: KanteButton.Emphasis.Primary
                onClicked: {
                    var path = nextCard.upcoming.path
                    sheet.close()
                    ctl.openFile(path)
                }
            }
        }
        // "?" at pocket and spread (LH-LRN-09), when the line names them.
        RowLayout {
            Layout.fillWidth: true
            spacing: KanteStyle.unit(4)
            QQC2.Label {
                Layout.fillWidth: true
                wrapMode: Text.WordWrap
                text: sheet.summary ? [sheet.summary.pocketText, sheet.summary.tendencyText, sheet.summary.spreadText, sheet.summary.intonationText].filter(function(t) { return t !== "" }).join(" · ") : ""
                color: KanteStyle.textColor
            }
            TermTip { topic: sheet.summary && sheet.summary.pocketText !== "" ? "summary.pocket" : "" }
            TermTip { topic: sheet.summary && sheet.summary.spreadText !== "" ? "summary.spread" : "" }
        }
        KanteTimingHistogram {
            Layout.fillWidth: true
            // Lower on low screens, so the hardest bars stay in view.
            Layout.preferredHeight: Math.min(KanteStyle.unit(180), Math.max(KanteStyle.unit(100), flick.height * 0.3))
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
        // Techniques apart from the hit rate (LH-BEW-06, round 9): sign and count per technique;
        // all played shows a framed sign, otherwise a struck one (shape, not colour alone).
        readonly property var techniques: sheet.summary && sheet.summary.techniques ? sheet.summary.techniques : []
        readonly property string untold: sheet.summary && sheet.summary.untoldText ? sheet.summary.untoldText : ""
        KanteSectionLabel {
            text: texts.t("summary.techniques")
            visible: content.techniques.length > 0 || content.untold !== ""
        }
        Flow {
            Layout.fillWidth: true
            visible: content.techniques.length > 0
            spacing: KanteStyle.unit(16)
            Repeater {
                model: content.techniques
                delegate: RowLayout {
                    required property var modelData
                    spacing: KanteStyle.unit(6)
                    KanteTechniqueMark {
                        Layout.preferredHeight: KanteStyle.unit(18)
                        Layout.preferredWidth: implicitWidth
                        technique: modelData.technique
                        result: modelData.played === modelData.judged ? "played" : "other"
                    }
                    QQC2.Label {
                        text: modelData.text
                        color: KanteStyle.textColor
                    }
                    TermTip { topic: "technique." + modelData.technique }
                }
            }
        }
        QQC2.Label {
            Layout.fillWidth: true
            visible: content.techniques.length > 0 || content.untold !== ""
            wrapMode: Text.WordWrap
            text: [content.techniques.length > 0 ? texts.t("summary.techniques.note") : "", content.untold].filter(function(t) { return t !== "" }).join(" ")
            color: KanteStyle.mutedTextColor
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
