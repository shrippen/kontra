import QtQuick
import QtQuick.Layouts
import QtQuick.Controls as QQC2
import org.kde.kirigami as Kirigami
import Kante

// Lernen (LH-LRN-07, J-G1): the course for the instrument, chapter by chapter as a path. Done
// lessons carry a check, the next one is marked, lessons for other instruments stay visible but
// locked (LH-LRN-12). Each chapter ends with a piece at three speeds; the glossary is one click.
Kirigami.ScrollablePage {
    id: page

    readonly property var ctl: applicationWindow().ctl
    readonly property var texts: applicationWindow().texts
    readonly property var course: JSON.parse(ctl.learnJson())
    readonly property string nextId: course.next ? course.next.id : ""

    function open(id) {
        applicationWindow().lessonId = id
        applicationWindow().lessonPage = 0
        applicationWindow().show("lesson")
    }
    function finale(chapter, percent) {
        if (!ctl.finaleExercise(chapter)) {
            return
        }
        ctl.changeSpeed(percent / 100)
        applicationWindow().show("practice")
    }

    background: Rectangle { color: KanteStyle.backgroundColor }
    title: texts.t("learn.title")
    padding: KanteStyle.unit(16)

    ColumnLayout {
        spacing: KanteStyle.unit(16)

        KanteHeading { text: texts.t("learn.title"); pageTitle: true }
        QQC2.Label {
            Layout.fillWidth: true
            wrapMode: Text.WordWrap
            color: KanteStyle.textColor
            text: texts.t(page.course.fits ? "learn.intro" : "learn.nocourse")
        }
        RowLayout {
            spacing: KanteStyle.unit(8)
            KanteButton {
                visible: page.nextId !== ""
                text: page.course.next ? texts.t("learn.continue", { title: page.course.next.title }) : ""
                emphasis: KanteButton.Emphasis.Primary
                onClicked: page.open(page.nextId)
            }
            KanteButton {
                text: texts.t("learn.glossary")
                emphasis: KanteButton.Emphasis.Quiet
                onClicked: applicationWindow().glossary("")
            }
        }

        Repeater {
            model: page.course.chapters
            delegate: ColumnLayout {
                id: chapter
                required property var modelData
                required property int index
                Layout.fillWidth: true
                spacing: KanteStyle.unit(8)

                KanteSectionLabel {
                    Layout.fillWidth: true
                    rule: true
                    text: texts.t("learn.chapter", { number: chapter.index + 1, title: chapter.modelData.title,
                                                     done: chapter.modelData.done, total: chapter.modelData.total })
                }
                KantePath {
                    Layout.fillWidth: true
                    Layout.maximumWidth: KanteStyle.unit(560)
                    model: chapter.modelData.units.map(function(u) {
                        return { title: u.title,
                                 state: !u.fits ? "locked" : u.done ? "done" : u.id === page.nextId ? "current" : "open" }
                    })
                    lockedText: texts.t("learn.locked.short")
                    doneText: texts.t("learn.state.done")
                    currentText: texts.t("learn.state.current")
                    openText: texts.t("learn.state.open")
                    onActivated: function(index) { page.open(chapter.modelData.units[index].id) }
                }
                // Nothing is locked (LERNEN.md): who knows a chapter proves it with its questions.
                KanteButton {
                    // Only lessons for this instrument can be ticked off.
                    visible: chapter.modelData.questions > 0
                        && chapter.modelData.units.some(function(u) { return u.fits && !u.done })
                    text: texts.t("learn.check")
                    emphasis: KanteButton.Emphasis.Quiet
                    onClicked: {
                        applicationWindow().checkChapter = chapter.modelData.id
                        applicationWindow().show("review")
                    }
                }

                // The chapter's piece, slow to full speed.
                KanteHintCard {
                    Layout.fillWidth: true
                    Layout.maximumWidth: KanteStyle.unit(560)
                    visible: chapter.modelData.finale !== null
                    tier: KanteHintCard.Tier.Blue
                    tierText: texts.t("learn.finale")
                    titleText: chapter.modelData.finale ? chapter.modelData.finale.title : ""
                    text: texts.t("learn.finale.text")
                    Repeater {
                        model: chapter.modelData.finale ? chapter.modelData.finale.speeds : []
                        delegate: KanteButton {
                            required property int modelData
                            required property int index
                            text: texts.t("learn.practice.speed", { percent: modelData })
                            emphasis: index === 0 ? KanteButton.Emphasis.Normal : KanteButton.Emphasis.Quiet
                            onClicked: page.finale(chapter.modelData.id, modelData)
                        }
                    }
                }
            }
        }
    }
}
