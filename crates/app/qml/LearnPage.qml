import QtQuick
import QtCore
import QtQuick.Layouts
import QtQuick.Controls as QQC2
import QtQuick.Dialogs
import org.kde.kirigami as Kirigami
import Kante

// Lernen (LH-LRN-07, J-G1): the course for the instrument, chapter by chapter as a path. Done
// lessons carry a check, the next one is marked, lessons for other instruments stay visible but
// locked (LH-LRN-12). Each chapter ends with a piece at three speeds; the glossary is one click.
// Courses from providers are added here (LH-LRN-14, J-G5): provider, licence and video hosts are
// shown before anything is added.
Kirigami.ScrollablePage {
    id: page

    readonly property var ctl: applicationWindow().ctl
    readonly property var texts: applicationWindow().texts
    property var course: JSON.parse(ctl.learnJson())
    // The package chosen to add, as `coursePreview` reads it; null: none.
    property var preview: null
    property string previewUrl: ""
    property string addError: ""
    property bool addressShown: false
    readonly property var fetchState: ctl.courseFetchJson !== "" ? JSON.parse(ctl.courseFetchJson) : ({ state: "idle" })
    readonly property bool fetching: fetchState.state === "running"
    readonly property string fetchError: fetchState.state === "error" ? fetchState.message : ""
    readonly property string nextId: course.next ? course.next.id : ""

    function open(id) {
        applicationWindow().lessonId = id
        applicationWindow().lessonPage = 0
        applicationWindow().show("lesson")
    }
    function showPreview(url) {
        page.previewUrl = url
        page.addError = ""
        page.preview = JSON.parse(ctl.coursePreview(url))
    }
    // A package opened with Kontra (file manager, `kontra kurs.kontrakurs`).
    Component.onCompleted: {
        if (ctl.startCourse !== "") {
            page.showPreview(ctl.startCourse)
            ctl.startCourse = ""
        }
    }

    function fetch() {
        ctl.courseFetch(addressField.text)
    }
    // A fetched package is previewed and added like a file.
    Connections {
        target: page.ctl
        function onCourseFetchJsonChanged() {
            if (page.fetchState.state === "done") {
                page.showPreview(page.fetchState.path)
            }
        }
    }

    function reload() {
        page.course = JSON.parse(ctl.learnJson())
    }
    function addCourse() {
        page.addError = ctl.courseAdd(page.previewUrl)
        if (page.addError !== "") {
            return
        }
        page.preview = null
        page.addressShown = false
        page.reload()
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
            KanteButton {
                text: texts.t("courses.add")
                emphasis: KanteButton.Emphasis.Quiet
                onClicked: courseDialog.open()
            }
            KanteButton {
                text: texts.t("courses.address")
                emphasis: KanteButton.Emphasis.Quiet
                onClicked: {
                    page.addressShown = true
                    addressField.forceActiveFocus()
                }
            }
        }

        // An address from a provider: Kontra loads only what is typed here (privacy).
        RowLayout {
            visible: page.addressShown
            spacing: KanteStyle.unit(8)
            KanteTextField {
                id: addressField
                Layout.preferredWidth: KanteStyle.unit(420)
                placeholderText: texts.t("courses.address.label")
                Accessible.name: texts.t("courses.address.label")
                inputMethodHints: Qt.ImhUrlCharactersOnly
                onAccepted: page.fetch()
            }
            KanteButton {
                text: page.fetching ? texts.t("courses.fetching") : texts.t("courses.fetch")
                enabled: !page.fetching && addressField.text.trim() !== ""
                onClicked: page.fetch()
            }
        }
        KanteCallout {
            Layout.fillWidth: true
            Layout.maximumWidth: KanteStyle.unit(640)
            visible: page.fetchError !== ""
            kind: KanteCallout.Kind.Warn
            autoHide: false
            text: page.fetchError
        }

        // More than one course for this instrument: choose which to learn.
        RowLayout {
            visible: (page.course.courses || []).length > 1
            spacing: KanteStyle.unit(8)
            QQC2.Label {
                color: KanteStyle.textColor
                text: texts.t("courses.label")
            }
            KanteComboBox {
                Accessible.name: texts.t("courses.label")
                model: (page.course.courses || []).map(function(c) { return c.title })
                currentIndex: (page.course.courses || []).findIndex(function(c) { return c.id === page.course.course })
                onActivated: function(index) {
                    ctl.chooseCourse(page.course.courses[index].id)
                    page.reload()
                }
            }
        }

        // What the chosen package is, before it is added.
        KanteCallout {
            Layout.fillWidth: true
            Layout.maximumWidth: KanteStyle.unit(640)
            visible: page.preview !== null
            autoHide: false
            kind: page.preview && page.preview.ok && page.preview.problems.length === 0
                ? KanteCallout.Kind.Info : KanteCallout.Kind.Warn
            title: !page.preview ? "" : page.preview.ok ? page.preview.title : page.preview.error
            text: {
                const p = page.preview
                if (!p || !p.ok) {
                    return ""
                }
                let lines = [texts.t("courses.from", { provider: p.provider, license: p.license, lessons: p.lessons }),
                             p.hosts.length > 0 ? texts.t("courses.hosts", { hosts: p.hosts.join(", ") })
                                                : texts.t("courses.nohosts")]
                if (!p.fits) {
                    lines.push(texts.t("courses.other"))
                }
                if (p.replaces) {
                    lines.push(texts.t("courses.replaces"))
                }
                if (p.problems.length > 0) {
                    lines.push(texts.t("courses.problems", { count: p.problems.length + p.more }))
                    lines = lines.concat(p.problems)
                }
                if (p.more > 0) {
                    lines.push(texts.t("courses.more", { count: p.more }))
                }
                if (page.addError !== "") {
                    lines.push(page.addError)
                }
                return lines.join("\n")
            }
            KanteButton {
                visible: page.preview !== null && page.preview.ok && page.preview.problems.length === 0
                text: texts.t("courses.confirm")
                emphasis: KanteButton.Emphasis.Primary
                onClicked: page.addCourse()
            }
            KanteButton {
                text: texts.t("courses.cancel")
                emphasis: KanteButton.Emphasis.Quiet
                onClicked: page.preview = null
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

    FileDialog {
        id: courseDialog
        title: texts.t("courses.add")
        currentFolder: StandardPaths.writableLocation(StandardPaths.DownloadLocation)
        nameFilters: [texts.t("courses.filter")]
        onAccepted: page.showPreview(selectedFile.toString())
    }
}
