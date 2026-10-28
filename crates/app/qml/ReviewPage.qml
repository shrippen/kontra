import QtQuick
import QtQuick.Layouts
import QtQuick.Controls as QQC2
import org.kde.kirigami as Kirigami
import Kante

// Wiederholen (LH-LRN-10, J-G3): the questions due again, one at a time, each with the lesson
// it comes from. The list is taken once on opening; answering moves a question to its next day.
// With `checkChapter` set on the window, "Kann ich schon": all questions of that chapter; all
// right ticks it off.
Kirigami.ScrollablePage {
    id: page

    // Tab never leaves the focus below the edge (Kante 1.49, QA 27.10. S2).
    readonly property KanteFocusFollow focusFollow: KanteFocusFollow { flickable: page.flickable }

    readonly property var ctl: applicationWindow().ctl
    readonly property var texts: applicationWindow().texts
    readonly property string chapter: applicationWindow().checkChapter
    readonly property var round: JSON.parse(chapter !== "" ? ctl.chapterCheckJson(chapter) : ctl.reviewJson())
    readonly property var questions: round.questions
    readonly property int total: questions.length
    readonly property var current: step < total ? questions[step] : null
    property int step: 0
    property int correct: 0
    property bool answered: false
    // The chapter's result, asked once at the end.
    property bool passed: false

    function next() {
        answered = false
        loader.active = false
        step += 1
        if (step >= total && chapter !== "") {
            passed = ctl.chapterPass(chapter, round.since)
        }
        loader.active = true
        page.flickable.contentY = 0
    }

    background: Rectangle { color: KanteStyle.backgroundColor }
    title: chapter !== "" ? texts.t("check.title", { title: round.title }) : texts.t("review.title")
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
            kind: page.total > 0 && (page.chapter === "" || page.passed) ? KanteCallout.Kind.Ok : KanteCallout.Kind.Info
            autoHide: false
            title: page.total === 0 ? texts.t("review.none")
                : page.chapter === "" ? texts.t("review.done", { right: page.correct, total: page.total })
                : page.passed ? texts.t("check.passed", { total: page.total })
                : texts.t("check.open", { right: page.correct, total: page.total })
        }
        KanteButton {
            visible: page.current === null
            text: texts.t("review.back")
            onClicked: applicationWindow().show("learn")
        }
    }
}
