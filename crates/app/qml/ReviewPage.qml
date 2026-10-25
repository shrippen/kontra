import QtQuick
import QtQuick.Layouts
import QtQuick.Controls as QQC2
import org.kde.kirigami as Kirigami
import Kante

// Wiederholen (LH-LRN-10, J-G3): the questions due again, one at a time, each with the lesson
// it comes from. The list is taken once on opening; answering moves a question to its next day.
Kirigami.ScrollablePage {
    id: page

    readonly property var ctl: applicationWindow().ctl
    readonly property var texts: applicationWindow().texts
    readonly property var questions: JSON.parse(ctl.reviewJson()).questions
    readonly property int total: questions.length
    readonly property var current: step < total ? questions[step] : null
    property int step: 0
    property int correct: 0
    property bool answered: false

    function next() {
        answered = false
        loader.active = false
        step += 1
        loader.active = true
        page.flickable.contentY = 0
    }

    background: Rectangle { color: KanteStyle.backgroundColor }
    title: texts.t("review.title")
    padding: KanteStyle.unit(16)

    ColumnLayout {
        spacing: KanteStyle.unit(16)

        KanteButton {
            text: texts.t("learn.course")
            emphasis: KanteButton.Emphasis.Quiet
            onClicked: applicationWindow().show("learn")
        }
        KanteHeading { text: page.title; pageTitle: true }

        KanteSectionLabel {
            Layout.fillWidth: true
            visible: page.current !== null
            rule: true
            text: page.current ? texts.t("review.step", { number: page.step + 1, total: page.total }) + " · "
                                 + texts.t("review.from", { title: page.current.lesson.title }) : ""
        }

        // The block is made anew for each question, so its pick starts empty.
        Loader {
            id: loader
            Layout.fillWidth: true
            visible: page.current !== null
            sourceComponent: page.current !== null ? question : null
        }
        Component {
            id: question
            LessonBlock {
                modelData: page.current.question
                index: page.current.block
                lessonId: page.current.lesson.id
                pageIndex: page.current.page
                onAnswered: function(wasRight) {
                    page.answered = true
                    if (wasRight) {
                        page.correct += 1
                    }
                }
                onTerm: function(id) { applicationWindow().glossary(id) }
            }
        }

        KanteButton {
            visible: page.current !== null && page.answered
            text: texts.t("review.next")
            emphasis: KanteButton.Emphasis.Primary
            onClicked: page.next()
        }

        // The end of the round, or nothing due.
        KanteCallout {
            Layout.fillWidth: true
            visible: page.current === null
            kind: page.total > 0 ? KanteCallout.Kind.Ok : KanteCallout.Kind.Info
            autoHide: false
            title: page.total > 0 ? texts.t("review.done", { right: page.correct, total: page.total })
                                  : texts.t("review.none")
        }
        KanteButton {
            visible: page.current === null
            text: texts.t("review.back")
            onClicked: applicationWindow().show("learn")
        }
    }
}
