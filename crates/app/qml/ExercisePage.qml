import QtQuick
import QtQuick.Layouts
import QtQuick.Controls as QQC2
import org.kde.kirigami as Kirigami
import Kante

// Exercises: the bundled set by level (LH-BIB-08) and generated ones (LH-LRN-04, LH-LRN-01:
// grooves on roots). Both open like any piece: practice page, progress, workshop.
Kirigami.ScrollablePage {
    id: page

    readonly property var ctl: applicationWindow().ctl
    readonly property var texts: applicationWindow().texts
    property var levels: []

    // Choices of the generator; values go to ctl.generateExercise.
    readonly property var kinds: ["groove", "scale", "arpeggio", "triads", "walking", "chromatic", "song"]
    // Pieces in a form to play through.
    readonly property var styles: ["ballad", "pop", "rock", "funk", "shuffle"]
    readonly property var songLevels: ["easy", "medium", "hard"]
    // Grooves judged by pitch and time, or by the attack only (DIDAKTIK I4).
    readonly property var judgings: ["pitch", "time"]
    readonly property var scales: ["major", "minor", "majorpenta", "minorpenta", "blues", "dorian",
        "phrygian", "lydian", "mixolydian", "locrian", "harmonicminor"]
    readonly property var progressions: ["tonic", "251", "blues", "minor", "145", "pop", "1625", "minor251"]
    readonly property var rhythms: ["quarters", "eighths", "sixteenths", "rests", "shuffle", "octaves", "ghost",
        "rootfifth", "rootfifthoctave", "dead", "offbeat", "tresillo", "triplets", "funk", "slap",
        "34", "68", "54", "78"]
    // Scale patterns ("" = up and down) and finger orders of the finger exercise.
    readonly property var patterns: ["", "thirds", "fours"]
    readonly property var fingerOrders: ["1234", "1324", "1243", "1423", "1342", "4321"]
    readonly property var keyNames: ["C", "C♯/D♭", "D", "E♭", "E", "F", "F♯", "G", "A♭", "A", "B♭", texts.t("exercises.key.b")]
    property int kind: 0
    property int variant: 0
    property int rhythm: 0
    property int pattern: 0
    property int songLevel: 1
    property int judging: 0
    property int key: 4
    property int fret: 5
    property int bpm: 80
    readonly property bool isScale: kinds[kind] === "scale"
    readonly property bool isFingers: kinds[kind] === "chromatic"
    readonly property bool isSong: kinds[kind] === "song"

    function reload() { levels = JSON.parse(ctl.exercisesJson()) }
    // The level being worked on (the first with an exercise that does not sit yet) stands open;
    // the others fold to their heading and count (QA 14.10.: 144 exercises in one list).
    readonly property int currentLevel: {
        for (var i = 0; i < levels.length; i++) {
            if (levels[i].mastered < levels[i].items.length)
                return i
        }
        return levels.length - 1
    }
    // Levels opened or closed by hand: index → bool.
    property var folded: ({})
    function isOpen(i) { return folded[i] !== undefined ? folded[i] : i === currentLevel }
    function toggle(i) {
        var f = Object.assign({}, folded)
        f[i] = !isOpen(i)
        folded = f
    }

    background: Rectangle { color: KanteStyle.backgroundColor }
    title: texts.t("exercises.title")
    padding: KanteStyle.unit(16)

    Component.onCompleted: {
        reload()
        sectionTimer.running = ctl.startSection === "make"
    }

    // QA `--section make`: open scrolled to the generator (screenshots).
    Timer {
        id: sectionTimer
        interval: 500
        onTriggered: page.flickable.contentY = makeLabel.mapToItem(page.flickable.contentItem, 0, 0).y
    }

    ColumnLayout {
        spacing: KanteStyle.unit(16)

        KanteHeading { text: texts.t("exercises.title"); pageTitle: true }

        // ── Bundled set ───────────────────────────────────────────────────
        // First: a beginner starts at the top of the plan; the generator is for later (QA 13.10.).
        KanteSectionLabel { text: texts.t("exercises.bundled") }
        QQC2.Label {
            Layout.fillWidth: true
            wrapMode: Text.WordWrap
            color: KanteStyle.textColor
            text: texts.t("exercises.bundled.hint", {
                count: page.levels.reduce(function(n, l) { return n + l.items.length }, 0),
                levels: page.levels.length
            })
        }
        Repeater {
            model: page.levels
            delegate: ColumnLayout {
                id: level
                required property var modelData
                required property int index
                readonly property bool open: page.isOpen(index)
                Layout.fillWidth: true
                spacing: 0
                RowLayout {
                    Layout.fillWidth: true
                    Layout.topMargin: KanteStyle.unit(8)
                    KanteHeading { text: level.modelData.level; Layout.fillWidth: true }
                    // How far the level is: "3 of 8 mastered" (P1 sees the plan move).
                    QQC2.Label {
                        text: texts.t("exercises.level.mastered", {
                            done: level.modelData.mastered, count: level.modelData.items.length
                        })
                        color: KanteStyle.mutedTextColor
                        font: KanteStyle.labelFont()
                    }
                    KanteToolButton {
                        id: foldButton
                        text: level.open ? texts.t("exercises.fold") : texts.t("exercises.unfold")
                        display: QQC2.AbstractButton.TextOnly
                        implicitWidth: Math.ceil(leftPadding + foldWords.advanceWidth + rightPadding + KanteStyle.unit(2))
                        focusPolicy: Qt.StrongFocus
                        Accessible.name: text + ": " + level.modelData.level
                        onClicked: page.toggle(level.index)
                        TextMetrics { id: foldWords; font: foldButton.font; text: foldButton.text }
                    }
                }
                Repeater {
                    model: level.open ? level.modelData.items : []
                    delegate: KanteListRow {
                        required property var modelData
                        Layout.fillWidth: true
                        text: modelData.title
                        subtitle: modelData.goal
                        meta: modelData.best >= 0
                            ? texts.t("library.best", { percent: Math.round(modelData.best * 100) })
                            : texts.t("exercises.new")
                        Accessible.name: text + ", " + meta
                            + (modelData.mastered ? ", " + texts.t("exercises.mastered") : "")
                        // Mastered: a tick, as for a hit note (not colour alone).
                        trailing: KanteNoteMark {
                            noteState: modelData.mastered ? "hit" : "pending"
                            width: KanteStyle.unit(16)
                            height: width
                            Accessible.ignored: true
                        }
                        onClicked: {
                            if (ctl.openFile(modelData.path))
                                applicationWindow().show("practice")
                        }
                    }
                }
            }
        }

        // ── Generator ─────────────────────────────────────────────────────
        KanteSectionLabel { id: makeLabel; text: texts.t("exercises.make"); Layout.topMargin: KanteStyle.unit(16) }
        QQC2.Label {
            Layout.fillWidth: true
            wrapMode: Text.WordWrap
            color: KanteStyle.textColor
            text: texts.t("exercises.make.hint")
        }
        KanteSettingRow {
            Layout.fillWidth: true
            title: texts.t("exercises.kind")
            // A list, not segments: seven kinds ran out of a narrow window (700 px, 24.10.).
            // The "?" before the list, so the lists stay flush right.
            Row {
                spacing: KanteStyle.unit(8)
                TermTip {
                    anchors.verticalCenter: parent.verticalCenter
                    topic: "kind." + page.kinds[page.kind]
                }
                KanteComboBox {
                    Accessible.name: texts.t("exercises.kind")
                    width: KanteStyle.unit(220)
                    model: page.kinds.map(function(k) { return texts.t("exercises.kind." + k) })
                    currentIndex: page.kind
                    onActivated: function(index) { page.kind = index; page.variant = 0 }
                }
            }
        }
        KanteSettingRow {
            id: variantRow
            Layout.fillWidth: true
            title: page.isScale ? texts.t("exercises.scale")
                : page.isFingers ? texts.t("exercises.fingers")
                : page.isSong ? texts.t("exercises.style") : texts.t("exercises.progression")
            Row {
                spacing: KanteStyle.unit(8)
                TermTip {
                    anchors.verticalCenter: parent.verticalCenter
                    topic: page.isScale ? "scale." + page.scales[page.variant]
                        : page.isFingers ? ""
                        : page.isSong ? "style." + page.styles[page.variant]
                        : "progression." + page.progressions[page.variant]
                }
                KanteComboBox {
                    Accessible.name: variantRow.title
                    width: KanteStyle.unit(220)
                    model: page.isScale
                        ? page.scales.map(function(s) { return texts.t("exercises.scale." + s) })
                        : page.isFingers
                            ? page.fingerOrders.map(function(f) { return f.split("").join("-") })
                        : page.isSong
                            ? page.styles.map(function(s) { return texts.t("exercises.style." + s) })
                            : page.progressions.map(function(p) { return texts.t("exercises.progression." + p) })
                    currentIndex: page.variant
                    onActivated: function(index) { page.variant = index }
                }
            }
        }
        KanteSettingRow {
            Layout.fillWidth: true
            visible: page.isScale
            title: texts.t("exercises.pattern")
            KanteSegmented {
                Accessible.name: texts.t("exercises.pattern")
                model: ["updown", "thirds", "fours"].map(function(p) { return texts.t("exercises.pattern." + p) })
                currentIndex: page.pattern
                onActivated: function(index) { page.pattern = index }
            }
        }
        KanteSettingRow {
            Layout.fillWidth: true
            visible: page.isSong
            title: texts.t("exercises.level")
            hint: texts.t("exercises.level.hint")
            KanteSegmented {
                Accessible.name: texts.t("exercises.level")
                model: page.songLevels.map(function(l) { return texts.t("exercises.level." + l) })
                currentIndex: page.songLevel
                onActivated: function(index) { page.songLevel = index }
            }
        }
        KanteSettingRow {
            Layout.fillWidth: true
            visible: page.kinds[page.kind] === "groove"
            title: texts.t("exercises.rhythm")
            hint: texts.t("exercises.rhythm.hint")
            Row {
                spacing: KanteStyle.unit(8)
                TermTip {
                    anchors.verticalCenter: parent.verticalCenter
                    topic: "rhythm." + page.rhythms[page.rhythm]
                }
                KanteComboBox {
                    Accessible.name: texts.t("exercises.rhythm")
                    width: KanteStyle.unit(220)
                    model: page.rhythms.map(function(r) { return texts.t("exercises.rhythm." + r) })
                    currentIndex: page.rhythm
                    onActivated: function(index) { page.rhythm = index }
                }
            }
        }
        KanteSettingRow {
            Layout.fillWidth: true
            visible: page.kinds[page.kind] === "groove"
            title: texts.t("exercises.judging")
            hint: texts.t("exercises.judging.hint")
            KanteSegmented {
                Accessible.name: texts.t("exercises.judging")
                model: page.judgings.map(function(j) { return texts.t("exercises.judging." + j) })
                currentIndex: page.judging
                onActivated: function(index) { page.judging = index }
            }
        }
        KanteSettingRow {
            Layout.fillWidth: true
            visible: page.isFingers
            title: texts.t("exercises.fret")
            hint: texts.t("exercises.fret.hint")
            KanteSpinBox {
                Accessible.name: texts.t("exercises.fret")
                from: 1
                to: 9
                value: page.fret
                onValueModified: page.fret = value
            }
        }
        KanteSettingRow {
            Layout.fillWidth: true
            visible: !page.isFingers
            title: texts.t("exercises.key")
            KanteComboBox {
                Accessible.name: texts.t("exercises.key")
                width: KanteStyle.unit(220)
                model: page.keyNames
                currentIndex: page.key
                onActivated: function(index) { page.key = index }
            }
        }
        KanteSettingRow {
            Layout.fillWidth: true
            title: texts.t("exercises.tempo")
            KanteSpinBox {
                Accessible.name: texts.t("exercises.tempo")
                from: 40
                to: 200
                stepSize: 5
                value: page.bpm
                editable: true
                onValueModified: page.bpm = value
                textFromValue: function(v) { return v + " BPM" }
                valueFromText: function(t) { return parseInt(t) }
            }
        }
        KanteButton {
            text: texts.t("exercises.generate")
            emphasis: KanteButton.Emphasis.Primary
            Accessible.name: texts.t("exercises.generate")
            onClicked: {
                var variant = page.isScale ? page.scales[page.variant]
                    : page.isFingers ? page.fingerOrders[page.variant]
                    : page.isSong ? page.styles[page.variant] : page.progressions[page.variant]
                // Scales take their pattern where grooves take the rhythm, pieces their level;
                // finger exercises a fret.
                var rhythm = page.isScale ? page.patterns[page.pattern]
                    : page.isSong ? page.songLevels[page.songLevel] : page.rhythms[page.rhythm]
                var key = page.isFingers ? page.fret : page.key
                var judging = page.kinds[page.kind] === "groove" ? page.judgings[page.judging] : ""
                if (ctl.generateExercise(page.kinds[page.kind], variant, rhythm, key, page.bpm, judging))
                    applicationWindow().show("practice")
            }
        }
    }
}
