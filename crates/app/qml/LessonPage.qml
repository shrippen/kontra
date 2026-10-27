import QtQuick
import QtQuick.Layouts
import QtQuick.Controls as QQC2
import org.kde.kirigami as Kirigami
import Kante

// One lesson (LH-LRN-07, J-G1), page by page: each page one step that fits a screen, the steps
// on top. A page shown counts as seen; the last one finishes the lesson. Terms open the glossary
// as a layer, so "back" returns to the same step.
Kirigami.ScrollablePage {
    id: page

    readonly property var ctl: applicationWindow().ctl
    readonly property var texts: applicationWindow().texts
    readonly property string lessonId: applicationWindow().lessonId
    readonly property var lesson: {
        const text = ctl.lessonJson(lessonId)
        return text !== "" ? JSON.parse(text) : null
    }
    readonly property int count: lesson ? lesson.pages.length : 0
    readonly property bool last: step >= count - 1
    property int step: Math.min(applicationWindow().lessonPage, Math.max(0, count - 1))
    property int earBlock: -1

    function go(to) {
        if (earBlock >= 0) {
            ctl.earStop()
            earBlock = -1
        }
        step = to
        page.flickable.contentY = 0
    }

    background: Rectangle { color: KanteStyle.backgroundColor }
    title: lesson ? lesson.title : texts.t("learn.title")
    padding: KanteStyle.unit(16)

    onStepChanged: {
        applicationWindow().lessonPage = step
        if (lesson) {
            ctl.lessonSeen(lessonId, step)
        }
    }
    Component.onCompleted: if (lesson) ctl.lessonSeen(lessonId, step)
    Component.onDestruction: if (earBlock >= 0) ctl.earStop()
    // A glossary layer shares the ear; once it stops it, the block here starts over.
    Connections {
        target: page.ctl
        function onEarStateChanged() {
            if (page.ctl.earState === "") {
                page.earBlock = -1
            }
        }
    }

    Shortcut { sequence: StandardKey.MoveToNextPage; enabled: !page.last; onActivated: page.go(page.step + 1) }
    Shortcut { sequence: StandardKey.MoveToPreviousPage; enabled: page.step > 0; onActivated: page.go(page.step - 1) }

    ColumnLayout {
        spacing: KanteStyle.unit(16)

        KanteButton {
            text: texts.t("learn.course")
            emphasis: KanteButton.Emphasis.Quiet
            onClicked: applicationWindow().show("learn")
        }
        KanteHeading { text: page.title; pageTitle: true }

        KanteCallout {
            Layout.fillWidth: true
            visible: page.lesson === null
            kind: KanteCallout.Kind.Warn
            autoHide: false
            title: texts.t("learn.missing")
        }
        // Not for this instrument (LH-LRN-12): said, not hidden.
        KanteCallout {
            Layout.fillWidth: true
            visible: page.lesson !== null && !page.lesson.fits
            kind: KanteCallout.Kind.Info
            autoHide: false
            title: texts.t("learn.locked")
        }

        KanteSteps {
            Layout.fillWidth: true
            visible: page.count > 1
            model: page.lesson ? page.lesson.pages.map(function(p) { return p.title }) : []
            current: page.step
            stepText: texts.t("learn.step")
            stateNames: ({ done: texts.t("learn.state.done"), current: texts.t("learn.state.current"),
                           upcoming: texts.t("learn.state.open") })
        }

        // The steps above name the current page; a heading repeated it (QA 27.10.).
        KanteHeading {
            level: 2
            visible: page.count === 1
            text: page.lesson && page.count > 0 ? page.lesson.pages[page.step].title : ""
        }

        Repeater {
            model: page.lesson && page.count > 0 ? page.lesson.pages[page.step].blocks : []
            delegate: LessonBlock {
                Layout.fillWidth: true
                lessonId: page.lessonId
                pageIndex: page.step
                earBlock: page.earBlock
                onTerm: function(id) { applicationWindow().glossary(id) }
                onEar: function(index) {
                    // A "listen" that fails (no device) leaves the running round as it was.
                    const before = page.earBlock
                    page.earBlock = index
                    if (!ctl.lessonEar(page.lessonId, page.step, index)) {
                        page.earBlock = before
                    }
                }
                onTuner: applicationWindow().show("tuner")
            }
        }

        RowLayout {
            spacing: KanteStyle.unit(8)
            KanteButton {
                visible: page.step > 0
                text: texts.t("learn.previous")
                onClicked: page.go(page.step - 1)
            }
            KanteButton {
                visible: page.count > 0
                text: texts.t(page.last ? "learn.finish" : "learn.next")
                emphasis: KanteButton.Emphasis.Primary
                onClicked: page.last ? applicationWindow().show("learn") : page.go(page.step + 1)
            }
        }
    }
}
