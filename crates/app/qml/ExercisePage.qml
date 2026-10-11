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
    readonly property var kinds: ["groove", "scale", "arpeggio", "walking"]
    readonly property var scales: ["major", "minor", "majorpenta", "minorpenta", "blues"]
    readonly property var progressions: ["tonic", "251", "blues", "minor"]
    readonly property var rhythms: ["quarters", "eighths", "sixteenths", "rests", "shuffle", "octaves"]
    readonly property var keyNames: ["C", "C♯/D♭", "D", "E♭", "E", "F", "F♯", "G", "A♭", "A", "B♭", texts.t("exercises.key.b")]
    property int kind: 0
    property int variant: 0
    property int rhythm: 0
    property int key: 4
    property int bpm: 80
    readonly property bool isScale: kinds[kind] === "scale"

    function reload() { levels = JSON.parse(ctl.exercisesJson()) }

    background: Rectangle { color: KanteStyle.backgroundColor }
    title: texts.t("exercises.title")
    padding: KanteStyle.unit(16)

    Component.onCompleted: reload()

    ColumnLayout {
        spacing: KanteStyle.unit(16)

        KanteHeading { text: texts.t("exercises.title"); pageTitle: true }

        // ── Generator ─────────────────────────────────────────────────────
        KanteSectionLabel { text: texts.t("exercises.make") }
        QQC2.Label {
            Layout.fillWidth: true
            wrapMode: Text.WordWrap
            color: KanteStyle.textColor
            text: texts.t("exercises.make.hint")
        }
        KanteSettingRow {
            Layout.fillWidth: true
            title: texts.t("exercises.kind")
            KanteSegmented {
                Accessible.name: texts.t("exercises.kind")
                model: page.kinds.map(function(k) { return texts.t("exercises.kind." + k) })
                currentIndex: page.kind
                onActivated: function(index) { page.kind = index; page.variant = 0 }
            }
        }
        KanteSettingRow {
            Layout.fillWidth: true
            title: page.isScale ? texts.t("exercises.scale") : texts.t("exercises.progression")
            KanteSegmented {
                Accessible.name: page.isScale ? texts.t("exercises.scale") : texts.t("exercises.progression")
                model: page.isScale
                    ? page.scales.map(function(s) { return texts.t("exercises.scale." + s) })
                    : page.progressions.map(function(p) { return texts.t("exercises.progression." + p) })
                currentIndex: page.variant
                onActivated: function(index) { page.variant = index }
            }
        }
        KanteSettingRow {
            Layout.fillWidth: true
            visible: page.kinds[page.kind] === "groove"
            title: texts.t("exercises.rhythm")
            hint: texts.t("exercises.rhythm.hint")
            KanteComboBox {
                Accessible.name: texts.t("exercises.rhythm")
                Layout.preferredWidth: KanteStyle.unit(220)
                model: page.rhythms.map(function(r) { return texts.t("exercises.rhythm." + r) })
                currentIndex: page.rhythm
                onActivated: function(index) { page.rhythm = index }
            }
        }
        KanteSettingRow {
            Layout.fillWidth: true
            title: texts.t("exercises.key")
            KanteComboBox {
                Accessible.name: texts.t("exercises.key")
                Layout.preferredWidth: KanteStyle.unit(220)
                model: page.keyNames
                currentIndex: page.key
                onActivated: function(index) { page.key = index }
            }
        }
        KanteSettingRow {
            Layout.fillWidth: true
            title: texts.t("exercises.tempo")
            QQC2.SpinBox {
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
                var variant = page.isScale ? page.scales[page.variant] : page.progressions[page.variant]
                if (ctl.generateExercise(page.kinds[page.kind], variant, page.rhythms[page.rhythm], page.key, page.bpm))
                    applicationWindow().show("practice")
            }
        }

        // ── Bundled set ───────────────────────────────────────────────────
        KanteSectionLabel { text: texts.t("exercises.bundled"); Layout.topMargin: KanteStyle.unit(8) }
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
                Layout.fillWidth: true
                spacing: 0
                KanteHeading { text: level.modelData.level; Layout.topMargin: KanteStyle.unit(8) }
                Repeater {
                    model: level.modelData.items
                    delegate: KanteListRow {
                        required property var modelData
                        Layout.fillWidth: true
                        text: modelData.title
                        meta: modelData.best >= 0
                            ? texts.t("library.best", { percent: Math.round(modelData.best * 100) })
                            : texts.t("exercises.new")
                        Accessible.name: text + ", " + meta
                        onClicked: {
                            if (ctl.openFile(modelData.path))
                                applicationWindow().show("practice")
                        }
                    }
                }
            }
        }
    }
}
