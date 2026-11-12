import QtQuick
import QtQuick.Layouts
import QtQuick.Controls as QQC2
import Kante

// Mixer (LH-WIE-03, LH-MOD-05): volumes of backing track (and its removed bass, LH-BIB-09),
// metronome, reference tone and drums (LH-LRN-01), and the bass played through Kontra (off by default: the interface's direct
// monitoring has no delay). While it plays, three tone bands, a compressor, warmth (LH-EIN-08),
// the effect chain (LH-EIN-19: built-in effects and plugins, free order, on or off, own values) shapes it, a limiter (on by default) catches peaks (LH-EIN-17);
// "Neutral" resets them.
// Changes apply at once, also while playing. On the practice page and in the settings.
ColumnLayout {
    id: mixer

    readonly property var ctl: applicationWindow().ctl
    readonly property var texts: applicationWindow().texts
    // On the practice page: without the ear training's prompt, which plays only there (QA 13.10.).
    property bool practice: false
    // The backing plays without its bass (LH-BIB-09): its original bass can come back quietly.
    readonly property bool withoutBass: practice && ctl.stemsJson !== ""
        && JSON.parse(ctl.stemsJson).state === "on"

    spacing: 0

    // One volume: title, slider and the value in percent (not only the slider's position).
    component Volume: KanteSettingRow {
        id: row
        property string bus
        property real value
        Layout.fillWidth: true
        RowLayout {
            spacing: KanteStyle.unit(8)
            QQC2.Slider {
                id: slider
                Layout.fillWidth: true
                Layout.preferredWidth: KanteStyle.unit(220)
                from: 0
                to: 1
                stepSize: 0.05
                value: row.value
                focusPolicy: Qt.StrongFocus
                Accessible.name: row.title
                onMoved: mixer.ctl.changeVolume(row.bus, value)
                KanteSliderSkin { control: slider }
            }
            QQC2.Label {
                Layout.preferredWidth: KanteStyle.unit(48)
                horizontalAlignment: Text.AlignRight
                color: KanteStyle.textColor
                text: mixer.texts.t("mixer.percent", { percent: Math.round(slider.value * 100) })
            }
        }
    }

    Volume {
        title: mixer.texts.t("mixer.backing")
        bus: "backing"
        value: mixer.ctl.backingVolume
    }
    Volume {
        visible: mixer.withoutBass
        title: mixer.texts.t("mixer.bass")
        hint: mixer.texts.t("mixer.bass.hint")
        bus: "bass"
        value: mixer.ctl.bassVolume
    }
    Volume {
        title: mixer.texts.t("mixer.metronome")
        bus: "metronome"
        value: mixer.ctl.metronomeVolume
    }
    Volume {
        title: mixer.texts.t("mixer.tone")
        bus: "tone"
        value: mixer.ctl.toneVolume
    }
    // One tone band of the monitored bass in whole dB, or an effect's amount in percent.
    component Shaping: KanteSettingRow {
        id: shapingRow
        property string band
        property real value
        readonly property bool isAmount: band !== "low" && band !== "mid" && band !== "high"
        readonly property real range: mixer.ctl.shapingRangeDb
        Layout.fillWidth: true
        visible: mixer.ctl.monitor
        RowLayout {
            spacing: KanteStyle.unit(8)
            QQC2.Slider {
                id: shapingSlider
                Layout.fillWidth: true
                Layout.preferredWidth: KanteStyle.unit(220)
                from: shapingRow.isAmount ? 0 : -shapingRow.range
                to: shapingRow.isAmount ? 1 : shapingRow.range
                stepSize: shapingRow.isAmount ? 0.05 : 1
                value: shapingRow.value
                focusPolicy: Qt.StrongFocus
                Accessible.name: shapingRow.title
                onMoved: mixer.ctl.changeShaping(shapingRow.band, value)
                KanteSliderSkin { control: shapingSlider }
            }
            QQC2.Label {
                Layout.preferredWidth: KanteStyle.unit(48)
                horizontalAlignment: Text.AlignRight
                color: KanteStyle.textColor
                text: shapingRow.isAmount
                    ? (shapingSlider.value === 0 ? mixer.texts.t("mixer.shaping.off")
                        : mixer.texts.t("mixer.percent", { percent: Math.round(shapingSlider.value * 100) }))
                    : mixer.texts.t("mixer.shaping.db", { db: (shapingSlider.value > 0 ? "+" : "") + Math.round(shapingSlider.value) })
            }
        }
    }

    Volume {
        title: mixer.texts.t("mixer.drums")
        bus: "drums"
        value: mixer.ctl.drumsVolume
    }
    Volume {
        visible: !mixer.practice
        title: mixer.texts.t("mixer.prompt")
        bus: "prompt"
        value: mixer.ctl.promptVolume
    }
    KanteSettingRow {
        Layout.fillWidth: true
        title: mixer.texts.t("mixer.monitor")
        hint: mixer.texts.t("mixer.monitor.hint")
        KanteSwitch {
            Accessible.name: mixer.texts.t("mixer.monitor")
            Accessible.description: mixer.texts.t("mixer.monitor.hint")
            focusPolicy: Qt.StrongFocus
            checked: mixer.ctl.monitor
            onToggled: mixer.ctl.changeMonitor(checked)
        }
    }
    Volume {
        visible: mixer.ctl.monitor
        title: mixer.texts.t("mixer.monitor.volume")
        bus: "monitor"
        value: mixer.ctl.monitorVolume
    }
    Shaping {
        section: mixer.texts.t("mixer.shaping")
        title: mixer.texts.t("mixer.shaping.low")
        hint: mixer.texts.t("mixer.shaping.hint")
        band: "low"
        value: mixer.ctl.monitorLowDb
    }
    Shaping {
        title: mixer.texts.t("mixer.shaping.mid")
        band: "mid"
        value: mixer.ctl.monitorMidDb
    }
    Shaping {
        title: mixer.texts.t("mixer.shaping.high")
        band: "high"
        value: mixer.ctl.monitorHighDb
    }
    Shaping {
        title: mixer.texts.t("mixer.shaping.compress")
        hint: mixer.texts.t("mixer.shaping.compress.hint")
        band: "compress"
        value: mixer.ctl.monitorCompress
    }
    Shaping {
        title: mixer.texts.t("mixer.shaping.warmth")
        hint: mixer.texts.t("mixer.shaping.warmth.hint")
        band: "warmth"
        value: mixer.ctl.monitorWarmth
    }
    // The effect chain of the monitored bass (LH-EIN-19): free order, each slot on or off,
    // with its own values. Everything is a button or a slider (no dragging); the list comes
    // from the core, the page keeps no logic.
    readonly property var fx: JSON.parse(ctl.effectsJson)
    // What "add" offers (picker): the built-in effects, then the plugins the search found, as
    // rows with the group they belong to. The page holds the chosen group, search and row.
    readonly property var addable: fx.kinds.map(function(k) {
        return { key: k.key, name: k.name, vendor: mixer.texts.t("fx.vendor.builtin"), format: mixer.texts.t("fx.format.builtin"), group: k.group, builtIn: true }
    }).concat(fx.plugins.map(function(p) {
        return { key: p.key, name: p.name, vendor: p.vendor, format: p.format, group: p.group, builtIn: false }
    }))
    // "all", "builtin", or a group key.
    property string pickGroup: "all"
    property string pickQuery: ""
    property string pickKey: ""
    readonly property var picks: addable.filter(function(e) {
        if (mixer.pickGroup === "builtin" && !e.builtIn) {
            return false
        }
        if (mixer.pickGroup !== "all" && mixer.pickGroup !== "builtin" && e.group !== mixer.pickGroup) {
            return false
        }
        var q = mixer.pickQuery.trim().toLowerCase()
        return q === "" || e.name.toLowerCase().indexOf(q) >= 0 || e.vendor.toLowerCase().indexOf(q) >= 0
    })
    readonly property int pickIndex: picks.findIndex(function(e) { return e.key === mixer.pickKey })
    // Adds the chosen row, or the first one when none is chosen (a typed name plus Enter),
    // through the dialog's accept, which closes it.
    function pickAdd() {
        if (pickIndex < 0 && picks.length > 0) {
            pickKey = picks[0].key
        }
        if (pickIndex >= 0 && !fx.full) {
            pickDialog.accept()
        }
    }
    // Puts the keyboard focus on the chosen row (the first when none is chosen); without rows
    // on the search field.
    function pickFocus() {
        if (picks.length === 0) {
            pickSearch.forceActiveFocus()
            return
        }
        if (pickIndex < 0) {
            pickKey = picks[0].key
        }
        Qt.callLater(function() {
            var row = pickList.itemAtIndex(mixer.pickIndex)
            if (row) {
                row.forceActiveFocus()
            }
        })
    }
    function pickStep(by) {
        if (picks.length === 0) {
            return
        }
        var at = Math.max(0, Math.min(picks.length - 1, (pickIndex < 0 ? (by > 0 ? -1 : picks.length) : pickIndex) + by))
        pickKey = picks[at].key
        pickList.positionViewAtIndex(at, ListView.Contain)
        pickFocus()
    }
    // Where a slot's move buttons stand, so keyboard focus follows the slot that moved.
    function focusMoved(to) {
        var slot = slots.itemAt(to)
        if (slot) {
            slot.focusMove()
        }
    }

    KanteSettingRow {
        Layout.fillWidth: true
        visible: mixer.ctl.monitor
        section: mixer.texts.t("fx.title")
        title: mixer.texts.t("fx.add")
        // While a plugin loads to be added (FX4), the row says so instead of its usual hint.
        hint: mixer.fx.adding !== "" ? mixer.fx.adding : mixer.texts.t("fx.hint")
        KanteButton {
            text: mixer.texts.t("fx.add.open")
            enabled: !mixer.fx.full
            onClicked: pickDialog.open()
        }
    }
    // The picker: groups, search and list in a dialog, so the page stays short.
    KanteDialog {
        id: pickDialog
        parent: QQC2.Overlay.overlay
        anchors.centerIn: parent
        modal: true
        size: KanteDialog.Size.Medium
        title: mixer.texts.t("fx.add")
        standardButtons: QQC2.Dialog.Ok | QQC2.Dialog.Cancel
        acceptText: mixer.texts.t("fx.add.accept")
        acceptEnabled: !mixer.fx.full && mixer.pickIndex >= 0
        onAccepted: mixer.ctl.editEffects("add", 0, mixer.pickKey, 0)
        // Each opening starts with an empty search, typing at once.
        onOpened: {
            pickSearch.text = ""
            pickSearch.forceActiveFocus()
        }
        ColumnLayout {
            anchors.fill: parent
            spacing: KanteStyle.unit(8)
            // The group chips: a checked chip is a solid fill with inverted text, an unchecked
            // one a frame; screen readers get the checked state from the chip.
            Flow {
                Layout.fillWidth: true
                spacing: KanteStyle.unit(8)
                Accessible.role: Accessible.Grouping
                Accessible.name: mixer.texts.t("fx.group.label")
                KanteChip {
                    choice: true
                    checkable: true
                    text: mixer.texts.t("fx.group.all") + " (" + mixer.addable.length + ")"
                    checked: mixer.pickGroup === "all"
                    onClicked: mixer.pickGroup = "all"
                }
                KanteChip {
                    choice: true
                    checkable: true
                    text: mixer.texts.t("fx.group.builtin") + " (" + mixer.fx.kinds.length + ")"
                    checked: mixer.pickGroup === "builtin"
                    onClicked: mixer.pickGroup = "builtin"
                }
                Repeater {
                    model: mixer.fx.groups
                    delegate: KanteChip {
                        required property var modelData
                        choice: true
                        checkable: true
                        text: modelData.name + " (" + modelData.count + ")"
                        checked: mixer.pickGroup === modelData.key
                        onClicked: mixer.pickGroup = modelData.key
                    }
                }
            }
            KanteCallout {
                Layout.fillWidth: true
                visible: mixer.fx.needsRescan
                kind: KanteCallout.Kind.Info
                text: mixer.texts.t("fx.rescan")
                autoHide: false
            }
            KanteTextField {
                id: pickSearch
                Layout.fillWidth: true
                placeholderText: mixer.texts.t("fx.search.placeholder")
                Accessible.name: mixer.texts.t("fx.search")
                onTextChanged: mixer.pickQuery = text
                onAccepted: mixer.pickAdd()
                Keys.onDownPressed: mixer.pickFocus()
            }
            // Shows how far the list is scrolled; only when rows lie beyond the edge.
            KanteScrollMeter {
                Layout.fillWidth: true
                visible: KanteStyle.active && pickList.contentHeight > pickList.height
                flickable: pickList
            }
            // The list ends in the middle of a row, so more rows are seen to lie below. The
            // chosen row is the one Tab stop; Up and Down move it, Enter adds it.
            ListView {
                id: pickList
                readonly property real rowHeight: count > 0 ? contentHeight / count : 0
                readonly property real shownRows: 8.5
                Layout.fillWidth: true
                Layout.fillHeight: true
                Layout.preferredHeight: Math.min(contentHeight, rowHeight * shownRows)
                clip: true
                model: mixer.picks
                currentIndex: Math.max(0, mixer.pickIndex)
                enabled: !mixer.fx.full
                Accessible.role: Accessible.List
                Accessible.name: mixer.texts.t("fx.list.label")
                delegate: KanteListRow {
                    required property var modelData
                    width: ListView.view.width
                    focusOnClick: false
                    activeFocusOnTab: ListView.isCurrentItem
                    // Reaching the list with nothing chosen chooses its first row.
                    onActiveFocusChanged: {
                        if (activeFocus && mixer.pickIndex < 0) {
                            mixer.pickKey = modelData.key
                        }
                    }
                    Keys.onUpPressed: mixer.pickStep(-1)
                    Keys.onDownPressed: mixer.pickStep(1)
                    Keys.onReturnPressed: mixer.pickAdd()
                    text: modelData.name
                    subtitle: modelData.vendor
                    meta: modelData.format
                    selected: modelData.key === mixer.pickKey
                    onClicked: mixer.pickKey = modelData.key
                }
            }
            Text {
                visible: mixer.picks.length === 0
                color: KanteStyle.mutedTextColor
                font: KanteStyle.smallFont
                text: mixer.texts.t("fx.none")
            }
        }
    }
    // Named chains (LH-EIN-19, Runde 19): save the chain under a name, call one up again.
    KanteSettingRow {
        Layout.fillWidth: true
        visible: mixer.ctl.monitor
        title: mixer.texts.t("fx.preset.save")
        hint: mixer.texts.t("fx.preset.hint")
        RowLayout {
            spacing: KanteStyle.unit(8)
            KanteTextField {
                id: presetName
                Layout.preferredWidth: KanteStyle.unit(180)
                placeholderText: mixer.texts.t("fx.preset.name")
                Accessible.name: mixer.texts.t("fx.preset.name")
                onAccepted: saveButton.clicked()
            }
            KanteButton {
                id: saveButton
                text: mixer.texts.t("fx.preset.save.label")
                Accessible.name: mixer.texts.t("fx.preset.save")
                enabled: presetName.text.trim() !== ""
                onClicked: {
                    mixer.ctl.editEffects("save", 0, presetName.text, 0)
                    presetName.text = ""
                }
            }
        }
    }
    KanteSettingRow {
        Layout.fillWidth: true
        visible: mixer.ctl.monitor
        title: mixer.texts.t("fx.preset.load")
        hint: mixer.fx.presets.length === 0 ? mixer.texts.t("fx.preset.none") : ""
        RowLayout {
            spacing: KanteStyle.unit(8)
            KanteComboBox {
                id: presetChoice
                Accessible.name: mixer.texts.t("fx.preset.load")
                Layout.preferredWidth: KanteStyle.unit(180)
                enabled: mixer.fx.presets.length > 0
                model: mixer.fx.presets
            }
            KanteButton {
                text: mixer.texts.t("fx.preset.load.label")
                Accessible.name: mixer.texts.t("fx.preset.load.button", { name: presetChoice.currentText })
                enabled: mixer.fx.presets.length > 0
                onClicked: mixer.ctl.editEffects("load", presetChoice.currentIndex, "", 0)
            }
            KanteButton {
                text: mixer.texts.t("fx.preset.delete.label")
                Accessible.name: mixer.texts.t("fx.preset.delete", { name: presetChoice.currentText })
                emphasis: KanteButton.Emphasis.Quiet
                enabled: mixer.fx.presets.length > 0
                onClicked: mixer.ctl.editEffects("delete", presetChoice.currentIndex, "", 0)
            }
        }
    }
    // The plugin search (LH-EIN-20, -21): the result in words, files that are broken by name.
    KanteSettingRow {
        Layout.fillWidth: true
        visible: mixer.ctl.monitor
        title: mixer.texts.t("fx.scan.button")
        hint: mixer.fx.scan.text
        KanteButton {
            text: mixer.texts.t("fx.scan.button")
            Accessible.name: mixer.texts.t("fx.scan.button")
            Accessible.description: mixer.texts.t("fx.scan.button.hint")
            emphasis: KanteButton.Emphasis.Quiet
            enabled: !mixer.fx.scan.running
            onClicked: mixer.ctl.editEffects("scan", 0, "", 0)
        }
    }
    // Files probed so far of all files; before the first file is known: indeterminate.
    KanteProgressBar {
        Layout.fillWidth: true
        visible: mixer.ctl.monitor && mixer.fx.scan.running
        value: mixer.fx.scan.total > 0 ? mixer.fx.scan.done / mixer.fx.scan.total : 0
        indeterminate: mixer.fx.scan.total <= 0
        Accessible.role: Accessible.ProgressBar
        Accessible.name: mixer.texts.t("fx.scan.bar")
        Accessible.description: mixer.fx.scan.text
    }
    Repeater {
        model: mixer.ctl.monitor ? mixer.fx.scan.broken : []
        delegate: Text {
            required property var modelData
            Layout.fillWidth: true
            wrapMode: Text.WordWrap
            color: KanteStyle.mutedTextColor
            font: KanteStyle.smallFont
            text: mixer.texts.t("fx.scan.broken", { file: modelData.file, reason: modelData.reason })
        }
    }
    // A full chain says so in words; the add buttons are off then.
    KanteCallout {
        Layout.fillWidth: true
        Layout.topMargin: KanteStyle.unit(8)
        visible: mixer.ctl.monitor && mixer.fx.full
        kind: KanteCallout.Kind.Warn
        text: mixer.fx.fullText
    }
    Text {
        Layout.fillWidth: true
        Layout.topMargin: KanteStyle.unit(8)
        visible: mixer.ctl.monitor && mixer.fx.empty
        color: KanteStyle.mutedTextColor
        font: KanteStyle.smallFont
        text: mixer.texts.t("fx.empty")
    }
    // A plugin slot (LH-EIN-20, -21) lists the plugin's parameters as sliders, like the values
    // of a built-in effect; a plugin that does not load says why and stays silent.
    Repeater {
        id: slots
        model: mixer.fx.slots.length
        delegate: ColumnLayout {
            id: slotItem
            readonly property var slot: mixer.fx.slots[index] || ({ values: [], name: "", hint: "", problem: "" })
            readonly property string slotName: mixer.texts.t("fx.slot", { number: index + 1, name: slot.name })
            Layout.fillWidth: true
            visible: mixer.ctl.monitor
            spacing: 0

            // After a move the focus goes to the same button at the slot's new place.
            function focusMove() {
                var button = slot.first ? downButton : upButton
                button.forceActiveFocus()
            }

            KanteSettingRow {
                Layout.fillWidth: true
                title: slotItem.slotName
                hint: slotItem.slot.hint
                RowLayout {
                    spacing: KanteStyle.unit(8)
                    // FX5: the plugin's own window, where it has one.
                    KanteButton {
                        visible: slotItem.slot.kind === "plugin"
                        text: mixer.texts.t("fx.window.label")
                        Accessible.name: mixer.texts.t("fx.window", { name: slotItem.slot.name })
                        emphasis: KanteButton.Emphasis.Quiet
                        size: KanteButton.Size.Small
                        onClicked: mixer.ctl.editEffects("window", index, "", 0)
                    }
                    KanteSwitch {
                        Accessible.name: mixer.texts.t("fx.on", { name: slotItem.slotName })
                        focusPolicy: Qt.StrongFocus
                        checked: slotItem.slot.on === true
                        onToggled: mixer.ctl.editEffects("on", index, "", checked ? 1 : 0)
                    }
                    KanteButton {
                        id: upButton
                        text: mixer.texts.t("fx.up.label")
                        Accessible.name: mixer.texts.t("fx.up", { name: slotItem.slotName })
                        emphasis: KanteButton.Emphasis.Quiet
                        size: KanteButton.Size.Small
                        enabled: slotItem.slot.first !== true
                        onClicked: {
                            mixer.ctl.editEffects("up", index, "", 0)
                            mixer.focusMoved(index - 1)
                        }
                    }
                    KanteButton {
                        id: downButton
                        text: mixer.texts.t("fx.down.label")
                        Accessible.name: mixer.texts.t("fx.down", { name: slotItem.slotName })
                        emphasis: KanteButton.Emphasis.Quiet
                        size: KanteButton.Size.Small
                        enabled: slotItem.slot.last !== true
                        onClicked: {
                            mixer.ctl.editEffects("down", index, "", 0)
                            mixer.focusMoved(index + 1)
                        }
                    }
                    KanteButton {
                        text: mixer.texts.t("fx.remove.label")
                        Accessible.name: mixer.texts.t("fx.remove", { name: slotItem.slotName })
                        emphasis: KanteButton.Emphasis.Quiet
                        size: KanteButton.Size.Small
                        destructive: true
                        onClicked: {
                            mixer.ctl.editEffects("remove", index, "", 0)
                            mixer.pickFocus()
                        }
                    }
                }
            }
            KanteCallout {
                Layout.fillWidth: true
                Layout.topMargin: KanteStyle.unit(4)
                visible: slotItem.slot.problem !== ""
                kind: KanteCallout.Kind.Warn
                text: slotItem.slot.problem
            }
            Repeater {
                model: slotItem.slot.values.length
                delegate: KanteSettingRow {
                    id: valueRow
                    readonly property var entry: slotItem.slot.values[index] || ({ key: "", label: "", value: 0 })
                    Layout.fillWidth: true
                    title: entry.label
                    RowLayout {
                        spacing: KanteStyle.unit(8)
                        QQC2.Slider {
                            id: valueSlider
                            Layout.fillWidth: true
                            Layout.preferredWidth: KanteStyle.unit(220)
                            from: 0
                            to: 1
                            stepSize: slotItem.slot.kind === "plugin" ? 0.01 : 0.05
                            value: valueRow.entry.value
                            focusPolicy: Qt.StrongFocus
                            Accessible.name: slotItem.slotName + ": " + valueRow.entry.label
                            onMoved: mixer.ctl.editEffects("set", slotItem.slot.index, valueRow.entry.key, value)
                            KanteSliderSkin { control: valueSlider }
                        }
                        QQC2.Label {
                            Layout.preferredWidth: KanteStyle.unit(64)
                            horizontalAlignment: Text.AlignRight
                            color: KanteStyle.textColor
                            // A plugin's value is in its own numbers (plugins name no unit here); the core formats it.
                            text: slotItem.slot.kind === "plugin" ? valueRow.entry.shown
                                : mixer.texts.t("mixer.percent", { percent: Math.round(valueSlider.value * 100) })
                        }
                    }
                }
            }
        }
    }
    // What the chain adds to the delay of the heard bass; empty when it adds none.
    Text {
        Layout.fillWidth: true
        Layout.topMargin: KanteStyle.unit(8)
        visible: mixer.ctl.monitor && mixer.fx.latencyText !== ""
        color: KanteStyle.mutedTextColor
        font: KanteStyle.smallFont
        wrapMode: Text.WordWrap
        text: mixer.fx.latencyText
    }
    KanteSettingRow {
        Layout.fillWidth: true
        visible: mixer.ctl.monitor
        title: mixer.texts.t("mixer.shaping.limit")
        hint: mixer.texts.t("mixer.shaping.limit.hint")
        KanteSwitch {
            Accessible.name: mixer.texts.t("mixer.shaping.limit")
            Accessible.description: mixer.texts.t("mixer.shaping.limit.hint")
            focusPolicy: Qt.StrongFocus
            checked: mixer.ctl.monitorLimit
            onToggled: mixer.ctl.changeLimit(checked)
        }
    }
    KanteSettingRow {
        Layout.fillWidth: true
        visible: mixer.ctl.monitor
        title: mixer.texts.t("mixer.shaping.neutral.title")
        KanteButton {
            text: mixer.texts.t("mixer.shaping.neutral")
            Accessible.name: mixer.texts.t("mixer.shaping.neutral.title")
            enabled: mixer.ctl.monitorLowDb !== 0 || mixer.ctl.monitorMidDb !== 0
                || mixer.ctl.monitorHighDb !== 0 || mixer.ctl.monitorCompress !== 0
                || mixer.ctl.monitorWarmth !== 0 || !mixer.fx.empty || !mixer.ctl.monitorLimit
            onClicked: mixer.ctl.resetShaping()
        }
    }
}
