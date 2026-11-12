import QtQuick
import QtQuick.Layouts
import Kante

// Song map (LH-FOR-06): hits per bar over the last runs (`songMapJson` in appcore); dragging
// over bars chooses them. Between runs only: hidden while playing and before the first run.
ColumnLayout {
    id: songMap

    signal chosen(int from, int to)

    readonly property var ctl: applicationWindow().ctl
    readonly property var texts: applicationWindow().texts
    property var summary: ({ values: [], labels: [], tips: [], loop: null, played: false })
    /// The place shows it (not presenting, room enough).
    property bool shown: true

    function refresh() {
        summary = JSON.parse(ctl.songMap())
    }

    visible: shown && !ctl.playing && summary.played
    spacing: KanteStyle.unit(4)
    // Only as high as its content: a nested layout fills by default and took half the
    // notes' room on the practice page, also with nothing shown (owner 12.11.).
    Layout.fillHeight: false
    Component.onCompleted: refresh()

    Connections {
        target: songMap.ctl
        function onPlayingChanged() {
            if (!songMap.ctl.playing) {
                songMap.refresh()
            }
        }
        function onArrangementLoaded() { songMap.refresh() }
        function onLoopStartChanged() { songMap.refresh() }
    }

    KanteHud { fullText: songMap.texts.t("songmap.title"); color: KanteStyle.mutedTextColor }
    KanteSongMap {
        Layout.fillWidth: true
        values: songMap.summary.values
        labels: songMap.summary.labels
        tips: songMap.summary.tips
        selectionFrom: songMap.summary.loop ? songMap.summary.loop[0] : -1
        selectionTo: songMap.summary.loop ? songMap.summary.loop[1] : -1
        accessibleName: songMap.texts.t("songmap.name")
        Accessible.description: songMap.texts.t("songmap.hint")
        onRangeChosen: (from, to) => songMap.chosen(from, to)
    }
}
