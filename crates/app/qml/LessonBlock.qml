import QtQuick
import QtQuick.Layouts
import QtQuick.Controls as QQC2
import Kante

// One block of a lesson or glossary entry (`Core::block_json`): text, list, hint, figure on the
// fretboard, "listen" and "try it" through ear training, practice at a chosen speed, a question,
// the tuner, a video with subtitles. Term links (`term:<id>`) go out as `term(id)`; the page decides.
ColumnLayout {
    id: block

    required property var modelData
    required property int index
    /** Lesson id and page for "listen", "try" and "practice"; empty in the glossary. */
    property string lessonId: ""
    property int pageIndex: 0
    /** The block whose ear question runs now (the page keeps it), so only that one shows it. */
    property int earBlock: -1
    readonly property var kindName: modelData.kind
    readonly property var ctl: applicationWindow().ctl
    readonly property var texts: applicationWindow().texts
    readonly property var round: earBlock === index && ctl.earState !== "" ? JSON.parse(ctl.earState) : null
    readonly property var stringNames: JSON.parse(ctl.tuningJson).map(function(s) { return s.name })

    signal term(string id)
    signal ear(int index)
    signal tuner()
    /** The first pick of a question (it is recorded for review). */
    signal answered(bool right)

    function link(target) {
        if (target.indexOf("term:") === 0) {
            block.term(target.slice(5))
        }
    }

    spacing: KanteStyle.unit(8)

    // Text and list items: rich text, terms as underlined links.
    Repeater {
        model: block.kindName === "text" ? [block.modelData.html]
            : block.kindName === "list" ? block.modelData.items : []
        delegate: QQC2.Label {
            required property string modelData
            Layout.fillWidth: true
            wrapMode: Text.WordWrap
            textFormat: Text.StyledText
            linkColor: KanteStyle.infoColor
            color: KanteStyle.textColor
            text: (block.kindName === "list" ? "• " : "") + modelData
            onLinkActivated: function(target) { block.link(target) }
            HoverHandler { cursorShape: parent.hoveredLink !== "" ? Qt.PointingHandCursor : Qt.ArrowCursor }
        }
    }

    KanteCallout {
        Layout.fillWidth: true
        visible: block.kindName === "hint"
        kind: KanteCallout.Kind.Info
        autoHide: false
        text: block.kindName === "hint" ? block.modelData.text : ""
    }

    // Tones: their names, and where they lie on a string instrument.
    QQC2.Label {
        Layout.fillWidth: true
        visible: block.modelData.notes !== undefined
        wrapMode: Text.WordWrap
        color: KanteStyle.strongTextColor
        text: block.modelData.notes ? block.modelData.notes.join(" – ") : ""
    }
    KanteFretboard {
        Layout.fillWidth: true
        // Two try-outs fit one laptop screen (J-G1; QA 27.10.: 1366×768 scrolled at 200 each).
        Layout.preferredHeight: Math.max(KanteStyle.unit(110), Math.min(KanteStyle.unit(150), width / 7))
        visible: block.modelData.positions !== undefined && block.modelData.positions.length > 0
        strings: block.stringNames.length
        stringNames: block.stringNames
        frets: 12
        lowStringOnTop: block.ctl.lowStringOnTop
        // The heard notes once the question is solved, else the tones to show.
        markers: (block.round && block.round.solved) ? block.round.positions
            : (block.modelData.positions || []).map(function(p) {
                return { string: p.string, fret: p.fret, role: "scale", label: p.label }
            })
        accessibleName: block.texts.t("fretboard.name")
        markerText: block.texts.t("fretboard.marker")
    }

    // Listen: Kontra plays the tones. Try: Kontra listens; each played note gets its mark.
    RowLayout {
        visible: block.kindName === "listen" || block.kindName === "try"
        spacing: KanteStyle.unit(8)
        KanteButton {
            text: block.texts.t(block.kindName === "try" ? "learn.try" : "learn.listen")
            emphasis: KanteButton.Emphasis.Primary
            onClicked: block.ear(block.index)
        }
        KanteButton {
            visible: block.kindName === "try"
            text: block.texts.t("learn.listen")
            enabled: block.round !== null && !block.round.prompting
            onClicked: block.ctl.earPlay()
        }
    }
    KanteCallout {
        id: tryState
        Layout.fillWidth: true
        visible: block.kindName === "try" && block.round !== null
        kind: block.round && block.round.solved ? KanteCallout.Kind.Ok : KanteCallout.Kind.Info
        autoHide: false
        title: !block.round ? ""
            : block.round.solved ? block.texts.t("learn.try.done")
            : block.round.silent ? block.texts.t("ear.silent")
            : block.round.prompting ? block.texts.t("ear.prompting")
            : !block.round.listening ? block.texts.t("learn.try.click")
            : block.texts.t("learn.try.now")
        text: block.round && block.round.solved ? block.round.reveal : ""
        Accessible.name: title
        onTitleChanged: if (visible) Accessible.announce(title)
    }
    // Without an input the keys answer instead of the bass.
    KanteKeyboard {
        readonly property var keys: JSON.parse(block.ctl.keyRangeJson)
        Layout.fillWidth: true
        Layout.preferredHeight: KanteStyle.unit(90)
        visible: block.kindName === "try" && block.round !== null && !block.round.listening && !block.round.solved
        interactive: true
        low: keys.low
        high: keys.high
        noteNames: keys.names
        accessibleName: block.texts.t("keyboard.name")
        markerText: block.texts.t("keyboard.marker")
        onActivated: function(midi) { block.ctl.earAnswer(midi) }
    }
    Flow {
        Layout.fillWidth: true
        spacing: KanteStyle.unit(12)
        visible: block.kindName === "try" && block.round !== null && block.round.played.length > 0
        Repeater {
            model: block.round ? block.round.played : []
            delegate: Row {
                required property var modelData
                spacing: KanteStyle.unit(4)
                KanteNoteMark {
                    anchors.verticalCenter: parent.verticalCenter
                    noteState: modelData.right ? "hit" : "wrong"
                }
                // A wrong note says what it was and which way to go: "C2 too high".
                QQC2.Label {
                    readonly property string direction: modelData.right ? "" : modelData.direction
                    color: KanteStyle.textColor
                    text: direction === "" ? modelData.note
                        : block.texts.t("ear.too." + direction, { note: modelData.note })
                    Accessible.name: direction !== "" ? text
                        : block.texts.t(modelData.right ? "ear.played.right" : "ear.played.wrong", { note: modelData.note })
                }
            }
        }
    }

    // Practice: the exercise, slow to full speed (one button per speed).
    QQC2.Label {
        Layout.fillWidth: true
        visible: block.kindName === "practice"
        wrapMode: Text.WordWrap
        color: KanteStyle.strongTextColor
        text: block.kindName === "practice" ? block.modelData.title : ""
    }
    Flow {
        Layout.fillWidth: true
        spacing: KanteStyle.unit(8)
        visible: block.kindName === "practice"
        Repeater {
            model: block.kindName === "practice" ? block.modelData.speeds : []
            delegate: KanteButton {
                required property int modelData
                required property int index
                text: block.texts.t("learn.practice.speed", { percent: modelData })
                emphasis: index === 0 ? KanteButton.Emphasis.Primary : KanteButton.Emphasis.Normal
                onClicked: {
                    if (!block.ctl.lessonExercise(block.lessonId, block.pageIndex, block.index)) {
                        return
                    }
                    block.ctl.changeSpeed(modelData / 100)
                    applicationWindow().show("practice")
                }
            }
        }
    }

    // Question: pick an answer; the pick gets a check or a cross and a word, not only colour.
    QQC2.Label {
        Layout.fillWidth: true
        visible: block.kindName === "question"
        wrapMode: Text.WordWrap
        textFormat: Text.StyledText
        linkColor: KanteStyle.infoColor
        color: KanteStyle.strongTextColor
        text: block.kindName === "question" ? block.modelData.html : ""
        onLinkActivated: function(target) { block.link(target) }
    }
    Repeater {
        id: options
        property int picked: -1
        model: block.kindName === "question" ? block.modelData.options : []
        delegate: RowLayout {
            required property string modelData
            required property int index
            readonly property bool chosen: options.picked === index
            readonly property bool correct: index === block.modelData.answer
            Layout.fillWidth: true
            spacing: KanteStyle.unit(8)
            KanteButton {
                text: modelData
                emphasis: KanteButton.Emphasis.Normal
                onClicked: {
                    // Only the first pick counts for review; later ones are for trying.
                    if (options.picked < 0 && block.lessonId !== "") {
                        block.answered(block.ctl.lessonAnswer(block.lessonId, block.pageIndex, block.index, index))
                    }
                    options.picked = index
                }
            }
            KanteNoteMark {
                visible: chosen
                noteState: correct ? "hit" : "wrong"
            }
            QQC2.Label {
                visible: chosen
                color: KanteStyle.textColor
                text: block.texts.t(correct ? "learn.question.right" : "learn.question.wrong")
            }
        }
    }

    KanteButton {
        visible: block.kindName === "tuner"
        text: block.texts.t("learn.tuner")
        onClicked: block.tuner()
    }

    // Videos only come with courses loaded later (LH-LRN-14); bundled ones have none. The
    // player sits in its own file: without Qt Multimedia it does not load, and a link remains.
    Loader {
        id: videoLoader
        Layout.fillWidth: true
        active: block.kindName === "video"
        source: "LessonVideo.qml"
        onLoaded: {
            item.cues = block.modelData.cues || []
            item.remote = block.modelData.src.indexOf("https://") === 0
            item.source = function() {
                return block.lessonId !== ""
                    ? block.ctl.lessonVideo(block.lessonId, block.pageIndex, block.index)
                    : (item.remote ? block.modelData.src : "")
            }
        }
    }
    KanteButton {
        visible: block.kindName === "video" && videoLoader.status === Loader.Error
            && block.modelData.src.indexOf("https://") === 0
        text: block.texts.t("learn.video")
        onClicked: {
            // QA: the browser stays shut; the log tells what would open.
            if (ctl.qaRun) {
                console.log("qa: open " + block.modelData.src)
                return
            }
            Qt.openUrlExternally(block.modelData.src)
        }
    }
}
