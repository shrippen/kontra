import QtQuick
import QtQuick.Layouts
import QtQuick.Controls as QQC2
import org.kde.kirigami as Kirigami
import Kante

// Glossary (LH-LRN-09, J-G2): with `termId` one entry, short, with its figure and the lesson
// that teaches it; without, the index of all terms for the instrument. It opens as a layer over
// the page that asked, so "back" (or Esc) returns to the same place with nothing changed.
// Terms explained by several courses show every text with its origin (LH-LRN-14).
Kirigami.ScrollablePage {
    id: page

    property string termId: ""
    readonly property var ctl: applicationWindow().ctl
    readonly property var texts: applicationWindow().texts
    readonly property var entry: {
        if (termId === "") {
            return null
        }
        const text = ctl.glossaryJson(termId)
        return text !== "" ? JSON.parse(text) : null
    }
    readonly property var terms: termId === "" ? JSON.parse(ctl.glossaryListJson()) : []
    property int earBlock: -1

    background: Rectangle { color: KanteStyle.backgroundColor }
    title: entry ? entry.title : texts.t("learn.glossary")
    padding: KanteStyle.unit(16)
    // A layer gets Kirigami's own header; the page brings its back button in Kante instead.
    globalToolBarStyle: Kirigami.ApplicationHeaderStyle.None

    Component.onDestruction: if (earBlock >= 0) ctl.earStop()
    // Only the top layer: two active Esc shortcuts would make Qt fire neither.
    Shortcut {
        sequence: "Esc"
        enabled: applicationWindow().pageStack.layers.currentItem === page
        onActivated: applicationWindow().pageStack.layers.pop()
    }

    ColumnLayout {
        spacing: KanteStyle.unit(16)

        KanteButton {
            text: texts.t("learn.back")
            emphasis: KanteButton.Emphasis.Quiet
            onClicked: applicationWindow().pageStack.layers.pop()
        }
        KanteHeading { text: page.title; pageTitle: true }

        KanteCallout {
            Layout.fillWidth: true
            visible: page.termId !== "" && page.entry === null
            kind: KanteCallout.Kind.Warn
            autoHide: false
            title: texts.t("learn.missing")
        }

        // The entry: its blocks; "listen" plays through ear training like in a lesson. Several
        // courses explaining the term follow each other, each under its name (LH-LRN-14).
        Repeater {
            model: page.entry ? page.entry.blocks : []
            delegate: ColumnLayout {
                id: entryBlock
                required property var modelData
                required property int index
                Layout.fillWidth: true
                spacing: KanteStyle.unit(8)

                KanteSectionLabel {
                    Layout.fillWidth: true
                    visible: entryBlock.modelData.from !== undefined
                    rule: true
                    text: entryBlock.modelData.from || ""
                }
                LessonBlock {
                    Layout.fillWidth: true
                    modelData: entryBlock.modelData
                    index: entryBlock.index
                    earBlock: page.earBlock
                    onTerm: function(id) { applicationWindow().glossary(id) }
                    onEar: function(index) {
                        page.earBlock = index
                        ctl.termEar(page.termId, index)
                    }
                }
            }
        }
        KanteButton {
            visible: page.entry !== null && page.entry.lesson !== null
            text: page.entry && page.entry.lesson ? texts.t("learn.term.lesson", { title: page.entry.lesson.title }) : ""
            onClicked: {
                const id = page.entry.lesson.id
                applicationWindow().pageStack.layers.clear()
                applicationWindow().lessonId = id
                applicationWindow().lessonPage = 0
                applicationWindow().show("lesson")
            }
        }

        // The index, by name.
        Flow {
            Layout.fillWidth: true
            spacing: KanteStyle.unit(8)
            visible: page.termId === ""
            Repeater {
                model: page.terms
                delegate: KanteButton {
                    required property var modelData
                    text: modelData.title
                    emphasis: KanteButton.Emphasis.Quiet
                    onClicked: applicationWindow().glossary(modelData.id)
                }
            }
        }
    }
}
