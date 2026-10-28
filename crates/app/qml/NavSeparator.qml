import org.kde.kirigami as Kirigami

// A line between groups of the sidebar. Disabled, so Tab passes over it: Kirigami makes every
// entry a tab stop, the lines too (QA 27.10., S3).
Kirigami.Action {
    separator: true
    enabled: false
}
