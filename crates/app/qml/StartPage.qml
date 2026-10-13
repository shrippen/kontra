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
    property var summary: ({ last: null, next: null, week: { minutes: 0, runs: 0 } })

    function reload() { summary = JSON.parse(ctl.homeJson()) }
    function open(path) {
        if (ctl.openFile(path)) {
            applicationWindow().show("practice")
        }
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
                tier: KanteHintCard.Tier.Green
                tierText: texts.t("home.next.tier")
                meta: page.summary.next ? page.summary.next.level : ""
                titleText: page.summary.next ? page.summary.next.title : texts.t("home.next.done")
                text: page.summary.next
                    ? page.bestText(page.summary.next.best) + " · " + texts.t("home.next.text")
                    : texts.t("home.next.done.text")
                KanteButton {
                    visible: page.summary.next !== null
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

        // The week in two figures; the details are on the progress page.
        KanteSectionLabel { text: texts.t("home.week"); rule: true; Layout.fillWidth: true }
        RowLayout {
            Layout.fillWidth: true
            spacing: KanteStyle.unit(32)
            KanteKpi {
                value: texts.t("home.week.minutes", { minutes: Math.round(page.summary.week.minutes) })
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
