import QtQuick
import QtQuick.Layouts
import QtQuick.Controls as QQC2
import org.kde.kirigami as Kirigami
import Kante

// Start page: one step from opening Kontra to playing. Continue the last piece, take the next
// bundled exercise that does not sit yet, see the week's practice (P1 needs a next step, P2 his
// piece, P3 her minutes). The numbers come from the core (homeJson); this page only shows them.
Kirigami.ScrollablePage {
    id: page

    readonly property var ctl: applicationWindow().ctl
    readonly property var texts: applicationWindow().texts
    property var summary: ({ last: null, next: null, spots: null, probe: null, week: { minutes: 0, runs: 0 } })
    readonly property bool sameAsLast: summary.next !== null && summary.last !== null
        && summary.next.path === summary.last.path

    function reload() { summary = JSON.parse(ctl.homeJson()) }
    function open(path) {
        if (ctl.openFile(path)) {
            applicationWindow().show("practice")
        } else if (JSON.parse(ctl.trackChoicesJson).length > 0) {
            // No clear bass track: the library asks which one (it shows the choice).
            applicationWindow().show("library")
        }
    }
    // A spot from the last days: the piece in the workshop, looping that bar.
    function practise(path, bar) {
        if (!ctl.openFile(path)) {
            if (JSON.parse(ctl.trackChoicesJson).length > 0)
                applicationWindow().show("library")
            return
        }
        ctl.changeMode("workshop")
        ctl.loopBars(bar - 1, bar - 1)
        applicationWindow().show("practice")
    }
    // A section practised before, once without notes (DIDAKTIK I1); the count-in gives a bar to
    // get ready.
    function probe(offer) {
        if (!ctl.openFile(offer.path)) {
            if (JSON.parse(ctl.trackChoicesJson).length > 0)
                applicationWindow().show("library")
            return
        }
        ctl.changeMode("workshop")
        // The bars moved (backing aligned anew): their written numbers, as for a spot.
        if (!ctl.loopAt(offer.startS, offer.endS))
            ctl.loopBars(offer.from - 1, offer.to - 1)
        applicationWindow().show("practice")
        ctl.startProbe()
    }
    function bestText(best) {
        return best !== null && best >= 0
            ? texts.t("library.best", { percent: Math.round(best * 100) })
            : texts.t("exercises.new")
    }

    background: Rectangle { color: KanteStyle.backgroundColor }
    title: texts.t("home.title")
    padding: KanteStyle.unit(16)

    Component.onCompleted: reload()

    ColumnLayout {
        spacing: KanteStyle.unit(16)

        KanteHeading { text: texts.t("home.title"); pageTitle: true }

        // The assistant was skipped: offer it once more, it is what makes the first notes count.
        KanteHintCard {
            Layout.fillWidth: true
            visible: !ctl.setupDone
            tier: KanteHintCard.Tier.Blue
            tierText: texts.t("home.setup.tier")
            titleText: texts.t("home.setup.title")
            text: texts.t("home.setup.text")
            KanteButton {
                text: texts.t("home.setup.start")
                emphasis: KanteButton.Emphasis.Primary
                onClicked: applicationWindow().show("setup")
            }
        }

        // Two cards side by side on wide windows, one above the other on narrow ones.
        GridLayout {
            Layout.fillWidth: true
            columns: page.width > KanteStyle.unit(760) ? 2 : 1
            columnSpacing: KanteStyle.unit(16)
            rowSpacing: KanteStyle.unit(16)

            KanteHintCard {
                Layout.fillWidth: true
                Layout.fillHeight: true
                Layout.preferredWidth: 1
                tier: KanteHintCard.Tier.Primary
                tierText: texts.t("home.last.tier")
                meta: page.summary.last ? page.bestText(page.summary.last.best) : ""
                titleText: page.summary.last ? page.summary.last.title : texts.t("home.last.none")
                text: page.summary.last
                    ? (page.summary.last.artist !== "" ? page.summary.last.artist + " · " : "")
                      + (page.summary.last.best === null ? texts.t("library.unplayed")
                         : page.summary.last.practiceMin < 1 ? texts.t("library.underminute")
                         : texts.t("library.minutes", { minutes: Math.round(page.summary.last.practiceMin) }))
                    : texts.t("home.last.none.text")
                KanteButton {
                    visible: page.summary.last !== null
                    text: texts.t("home.last.play")
                    emphasis: ctl.setupDone ? KanteButton.Emphasis.Primary : KanteButton.Emphasis.Normal
                    onClicked: page.open(page.summary.last.path)
                }
                KanteButton {
                    text: texts.t("home.library")
                    emphasis: KanteButton.Emphasis.Quiet
                    onClicked: applicationWindow().show("library")
                }
            }

            KanteHintCard {
                Layout.fillWidth: true
                Layout.fillHeight: true
                Layout.preferredWidth: 1
                tier: KanteHintCard.Tier.Blue
                tierText: texts.t("home.next.tier")
                meta: page.summary.next ? page.summary.next.level : ""
                titleText: page.summary.next ? page.summary.next.title : texts.t("home.next.done")
                text: !page.summary.next ? texts.t("home.next.done.text")
                    : page.sameAsLast ? texts.t("home.next.same")
                    : page.bestText(page.summary.next.best) + " · " + texts.t("home.next.text")
                KanteButton {
                    // The same piece as "continue": one button for it is enough.
                    visible: page.summary.next !== null && !page.sameAsLast
                    text: texts.t("home.next.play")
                    // One main action per page: before the setup, that is the setup.
                    emphasis: ctl.setupDone ? KanteButton.Emphasis.Primary : KanteButton.Emphasis.Normal
                    onClicked: page.open(page.summary.next.path)
                }
                KanteButton {
                    text: texts.t("home.exercises")
                    emphasis: KanteButton.Emphasis.Quiet
                    onClicked: applicationWindow().show("exercises")
                }
            }
        }

        // Round 11, "spots from yesterday": the bars that were hardest in the last days come back
        // after a night (DIDAKTIK: spaced repetition), each one click from the workshop.
        KanteHintCard {
            Layout.fillWidth: true
            visible: page.summary.spots !== null
            tier: KanteHintCard.Tier.Blue
            tierText: texts.t("home.spots.tier")
            titleText: page.summary.spots ? page.summary.spots.title : ""
            text: texts.t("home.spots.text")
            Repeater {
                model: page.summary.spots ? page.summary.spots.bars : []
                delegate: KanteButton {
                    required property int modelData
                    required property int index
                    text: texts.t("home.spots.bar", { bar: modelData })
                    emphasis: index === 0 ? KanteButton.Emphasis.Normal : KanteButton.Emphasis.Quiet
                    onClicked: page.practise(page.summary.spots.path, modelData)
                }
            }
        }

        // DIDAKTIK I1: what was practised with notes, checked once without them after a night.
        KanteHintCard {
            Layout.fillWidth: true
            visible: page.summary.probe !== null
            tier: KanteHintCard.Tier.Blue
            tierText: texts.t("home.probe.tier")
            titleText: page.summary.probe ? page.summary.probe.title : ""
            text: texts.t("home.probe.text")
            KanteButton {
                text: page.summary.probe ? texts.t("home.probe.play", { from: page.summary.probe.from, to: page.summary.probe.to }) : ""
                onClicked: page.probe(page.summary.probe)
            }
        }

        // The week in two figures; the details are on the progress page.
        KanteSectionLabel { text: texts.t("home.week"); rule: true; Layout.fillWidth: true }
        RowLayout {
            Layout.fillWidth: true
            spacing: KanteStyle.unit(32)
            KanteKpi {
                // Under a minute is not nothing: "< 1 min" once anything was played.
                value: page.summary.week.minutes > 0 && page.summary.week.minutes < 1
                    ? texts.t("home.week.under")
                    : texts.t("home.week.minutes", { minutes: Math.round(page.summary.week.minutes) })
                label: texts.t("home.week.minutes.label")
            }
            KanteKpi {
                value: String(page.summary.week.runs)
                label: texts.t("home.week.runs.label")
            }
            Item { Layout.fillWidth: true }
            KanteButton {
                text: texts.t("home.progress")
                emphasis: KanteButton.Emphasis.Quiet
                onClicked: applicationWindow().show("progress")
            }
        }

        // Before playing: tune; between pieces: fretboard and ear.
        KanteSectionLabel { text: texts.t("home.tools"); rule: true; Layout.fillWidth: true }
        Flow {
            Layout.fillWidth: true
            spacing: KanteStyle.unit(8)
            KanteButton { text: texts.t("nav.tuner"); onClicked: applicationWindow().show("tuner") }
            KanteButton { text: texts.t("nav.quiz"); onClicked: applicationWindow().show("quiz") }
            KanteButton { text: texts.t("nav.ear"); onClicked: applicationWindow().show("ear") }
        }
    }
}
