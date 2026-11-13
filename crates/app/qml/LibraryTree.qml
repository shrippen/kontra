import QtQuick
import QtQuick.Layouts
import Kante

// The library's sources (LH-BIB-17): all, recently played, the origins, the linked folders with
// their subfolders and the setlists, each with its count. Rows of KanteListRow in a list; every
// origin has a sign and a word, never a colour alone.
//
//   ALLE STÜCKE  214
//   ZULETZT GESPIELT
//   HERKUNFT
//     ◆ Übungen · ◇ Generiert · ↓ Heruntergeladen · ● Importiert
//   ORDNER AUF DER FESTPLATTE
//     □ Band · Rock · Slap      (× removes the link, the files stay)
//     + Ordner hinzufügen …
//   SETLISTEN
//     Freitag · + Neue Setliste
ListView {
    id: tree

    required property var texts
    // `counts` of Core::library_json.
    required property var counts
    // The source shown: a key as library_json takes it ("all", "folder:/path", "setlist:name").
    property string source: "all"

    readonly property var signs: ({ exercise: "◆", generated: "◇", download: "↓", imported: "●", folder: "□" })
    readonly property var origins: ["exercise", "generated", "download", "imported"]
    readonly property var entries: build()

    signal picked(string key)
    signal addFolderRequested()
    signal newSetlistRequested()
    signal unlinkRequested(string path)

    function item(key, text, count, sign) {
        return { kind: "item", key: key, text: text, count: count, sign: sign || "", depth: 0, top: false, path: "" }
    }

    function folderEntries(out, node, depth) {
        var gone = node.missing === true
        var text = gone ? node.name + " (" + texts.t("library.folder.missing") + ")"
            : node.cutOff === true ? node.name + " (" + texts.t("library.folder.cutoff") + ")" : node.name
        var entry = item("folder:" + node.path, text, node.count, signs.folder)
        entry.depth = depth
        entry.top = depth === 0
        entry.path = node.path
        entry.name = node.name
        entry.missing = gone
        out.push(entry)
        node.children.forEach(function(child) { folderEntries(out, child, depth + 1) })
    }

    function build() {
        var out = [
            item("all", texts.t("library.source.all"), counts.all),
            item("recent", texts.t("library.source.recent"), counts.recent),
            { kind: "heading", text: texts.t("library.heading.origin") }
        ]
        origins.forEach(function(o) {
            out.push(item(o, texts.t("library.origin." + o), counts.origins[o], signs[o]))
        })
        out.push({ kind: "heading", text: texts.t("library.heading.folders") })
        counts.folders.forEach(function(node) { folderEntries(out, node, 0) })
        out.push({ kind: "action", id: "folder", text: texts.t("library.folder.add") })
        out.push({ kind: "heading", text: texts.t("setlist.title") })
        counts.setlists.forEach(function(s) {
            out.push(item("setlist:" + s.name, s.name, s.count))
        })
        out.push({ kind: "action", id: "setlist", text: texts.t("library.setlist.add") })
        return out
    }

    clip: true
    model: entries
    Accessible.role: Accessible.List
    Accessible.name: texts.t("library.tree.name")

    delegate: Loader {
        id: cell
        required property var modelData
        width: ListView.view.width
        sourceComponent: modelData.kind === "heading" ? headingPart : rowPart

        Component {
            id: headingPart
            KanteSectionLabel {
                text: cell.modelData.text
                topPadding: KanteStyle.unit(16)
                bottomPadding: KanteStyle.unit(4)
                leftPadding: KanteStyle.unit(12)
                Accessible.role: Accessible.Heading
                Accessible.name: text
            }
        }

        Component {
            id: rowPart
            Item {
                implicitHeight: row.implicitHeight
                readonly property var entry: cell.modelData
                readonly property bool action: entry.kind === "action"
                readonly property real indent: (entry.depth || 0) * KanteStyle.unit(14)

                KanteListRow {
                    id: row
                    x: parent.indent
                    width: parent.width - parent.indent
                    density: KanteListRow.Density.Dense
                    rule: false
                    text: parent.entry.text
                    leadingText: parent.entry.sign || ""
                    count: parent.action ? "" : String(parent.entry.count)
                    selected: !parent.action && parent.entry.key === tree.source
                    Accessible.name: parent.action ? text
                        : tree.texts.t("library.tree.item", { name: text, n: parent.entry.count })
                    onClicked: {
                        if (!parent.action) {
                            tree.picked(parent.entry.key)
                        } else if (parent.entry.id === "folder") {
                            tree.addFolderRequested()
                        } else {
                            tree.newSetlistRequested()
                        }
                    }
                    trailing: KanteToolButton {
                        visible: row.parent.entry.top === true
                        text: "×"
                        Accessible.name: tree.texts.t("library.folder.unlink", { name: row.parent.entry.name || "" })
                        onClicked: tree.unlinkRequested(row.parent.entry.path)
                    }
                }
            }
        }
    }
}
