# Release steps

- Every release gets a detailed release log, written before the tag: a lead (what the release is about), then the changes grouped as New, Improved, Fixed and Note (breaking changes, steps the user must take). Each point says what changed for the user and why, not which code moved.
- Graphics wherever possible: for every visible change a screenshot from the demo world in every language of the landing page (`docs/shots/release-<version>-<name>-<lang>.webp`), for changes inside a diagram (Kante `.flow` or an inline SVG).
- The log goes into the release notes on Gitea and as a new entry at the top of `#changes` on the landing page (`docs/index.html`, Kante `.changelog`); from the fourth entry on, the oldest move into `details.changelog-more`. Bump the version in the footer. A release is done when the landing page shows it.

## Windows-Downloads

`.github/workflows/release.yml` läuft beim Tag `v*` auf GitHub (nie auf Gitea): es prüft, dass Tag und Version in
`Cargo.toml` stimmen und `CHANGELOG.md` den Abschnitt hat, baut mit `scripts/package-windows.sh` und
`scripts/installer-windows.sh` den Installer (`Kontra-<Version>-windows-setup.exe`, Inno Setup) und das portable ZIP
(`Kontra-<Version>-windows-portable.zip`) und hängt beide an das GitHub-Release (legt es mit den Notes an, falls es
fehlt). Vorher am gebauten ZIP nachsehen, dass keine Regeldateien darin liegen (das Skript prüft es). Der Eintrag
`kontra` in `shrippen.github.io/docs/versions.json` kommt mit der Landing-Page-Aktualisierung, sonst zeigt kein Kontra
den Update-Hinweis.
