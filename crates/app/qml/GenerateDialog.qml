import QtQuick
import QtQuick.Controls as QQC2
import QtQuick.Layouts
import Kante

// A new piece from a style (LH-BIB-14): style, difficulty, key, length and a variant. The same
// choices always give the same piece; "Another variant" takes the next seed, so a piece that
// does not please can be swapped for a sibling.
KanteDialog {
    id: dialog

    required property var ctl
    required property var texts

    // Choices as the generator takes them (`ctl.generatePiece`).
    readonly property var styles: ["ballad", "pop", "rock", "funk", "shuffle"]
    readonly property var levels: ["easy", "medium", "hard"]
    readonly property var lengths: ["short", "normal", "long"]
    readonly property var keyNames: ["C", "C♯/D♭", "D", "E♭", "E", "F", "F♯", "G", "A♭", "A", "B♭", texts.t("exercises.key.b")]
    // A plain tempo; the workshop slows a piece down.
    readonly property int tempoBpm: 80
    property int style: 1
    property int level: 1
    property int key: 4
    property int length: 1
    // 0 is the plain piece, any other number a variant.
    property int variant: 0

    // The piece was made and opened: the page goes to practice.
    signal generated()

    parent: QQC2.Overlay.overlay
    anchors.centerIn: parent
    modal: true
    size: KanteDialog.Size.Small
    title: texts.t("library.gen.title")
    standardButtons: QQC2.Dialog.Ok | QQC2.Dialog.Cancel
    acceptText: texts.t("library.gen.make")
    onAccepted: {
        if (ctl.generatePiece(styles[style], levels[level], key, tempoBpm, variant, lengths[length])) {
            generated()
        }
    }

    // A piece from a wish in words (LH-BIB-16) instead of a style: PieceWish takes over.
    footerStart: [
        KanteButton {
            text: dialog.texts.t("library.gen.wish")
            emphasis: KanteButton.Emphasis.Quiet
            Accessible.name: dialog.texts.t("library.gen.wish")
            onClicked: {
                dialog.close()
                wish.active = true
                wish.item.open()
            }
        }
    ]
    ColumnLayout {
        anchors.fill: parent
        spacing: KanteStyle.unit(8)

        // Loaded at the first wish: the dialog needs the window's `ctl`. Inside the layout, so
        // the dialog keeps one content item.
        Loader {
            id: wish
            active: false
            sourceComponent: PieceWish {}
        }

        KanteSettingRow {
            Layout.fillWidth: true
            title: dialog.texts.t("exercises.style")
            KanteComboBox {
                Accessible.name: dialog.texts.t("exercises.style")
                width: KanteStyle.unit(200)
                model: dialog.styles.map(function(s) { return dialog.texts.t("exercises.style." + s) })
                currentIndex: dialog.style
                onActivated: function(index) { dialog.style = index }
            }
        }
        KanteSettingRow {
            Layout.fillWidth: true
            title: dialog.texts.t("exercises.level")
            KanteSegmented {
                Accessible.name: dialog.texts.t("exercises.level")
                model: dialog.levels.map(function(l) { return dialog.texts.t("exercises.level." + l) })
                currentIndex: dialog.level
                onActivated: function(index) { dialog.level = index }
            }
        }
        KanteSettingRow {
            Layout.fillWidth: true
            title: dialog.texts.t("exercises.key")
            KanteComboBox {
                Accessible.name: dialog.texts.t("exercises.key")
                width: KanteStyle.unit(200)
                model: dialog.keyNames
                currentIndex: dialog.key
                onActivated: function(index) { dialog.key = index }
            }
        }
        KanteSettingRow {
            Layout.fillWidth: true
            title: dialog.texts.t("library.gen.length")
            KanteSegmented {
                Accessible.name: dialog.texts.t("library.gen.length")
                model: dialog.lengths.map(function(l) { return dialog.texts.t("library.gen.length." + l) })
                currentIndex: dialog.length
                onActivated: function(index) { dialog.length = index }
            }
        }
        KanteSettingRow {
            Layout.fillWidth: true
            title: dialog.texts.t("library.gen.variant")
            Row {
                spacing: KanteStyle.unit(8)
                QQC2.Label {
                    anchors.verticalCenter: parent.verticalCenter
                    text: dialog.variant === 0 ? dialog.texts.t("library.gen.variant.plain")
                        : dialog.texts.t("library.gen.variant.n", { n: dialog.variant })
                    color: KanteStyle.strongTextColor
                }
                // The reset comes first: "Another variant" keeps its place at the right edge, so
                // clicking it again never hits the reset (QA 13.11.).
                KanteButton {
                    visible: dialog.variant > 0
                    text: dialog.texts.t("library.gen.variant.plain")
                    emphasis: KanteButton.Emphasis.Quiet
                    onClicked: dialog.variant = 0
                }
                KanteButton {
                    objectName: "otherVariant"
                    text: dialog.texts.t("library.gen.other")
                    emphasis: KanteButton.Emphasis.Quiet
                    onClicked: dialog.variant += 1
                }
            }
        }
    }
}
