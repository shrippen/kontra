import QtQuick
import QtQuick.Layouts
import Kante

// Notes density (round 5): Kontra suggests, the player decides, between runs. Shown while
// stopped and kept through the next run; a suggestion during a run (after a loop's pass)
// waits for its end. Coming or going while playing would move the notes (owner 12.11.).
KanteCallout {
    id: hint

    readonly property var ctl: applicationWindow().ctl
    readonly property var texts: applicationWindow().texts
    property bool shown: ctl.densitySuggestion > 0 && !ctl.playing

    visible: shown
    kind: KanteCallout.Kind.Info
    autoHide: false
    title: ctl.densityText
    Accessible.name: ctl.densityText
    onVisibleChanged: if (visible) Accessible.announce(ctl.densityText)
    // Once set, `shown` follows only these: the binding above is the start value.
    Component.onCompleted: shown = shown

    Connections {
        target: hint.ctl
        function onDensitySuggestionChanged() {
            if (hint.ctl.densitySuggestion <= 0) {
                hint.shown = false
                return
            }
            if (!hint.ctl.playing) {
                hint.shown = true
            }
        }
        function onPlayingChanged() {
            if (!hint.ctl.playing && hint.ctl.densitySuggestion > 0) {
                hint.shown = true
            }
        }
    }

    RowLayout {
        KanteButton {
            enabled: !hint.ctl.playing
            text: hint.texts.t("density.accept")
            emphasis: KanteButton.Emphasis.Primary
            onClicked: hint.ctl.changeDensity(hint.ctl.densitySuggestion, "free")
        }
        KanteButton {
            enabled: !hint.ctl.playing
            text: hint.texts.t("density.decline")
            emphasis: KanteButton.Emphasis.Quiet
            onClicked: hint.ctl.declineDensity()
        }
    }
}
