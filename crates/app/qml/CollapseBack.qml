import QtQuick

// Gives the sidebar its collapse button back once it is inline again. Kirigami's drawer turns
// `collapsible` off when it turns modal (OverlayDrawer.onModalChanged) and never back on: after
// the presentation, or a window that was narrow, the sidebar had no "Close Sidebar" button and
// lost the width of its text (QA 28.10., B8, P4).
//
//   inline ──F5──► modal (Kirigami: collapsible = false) ──Esc──► inline, collapsible = true
Binding {
    required property var drawer

    target: drawer
    property: "collapsible"
    value: !drawer.modal
}
