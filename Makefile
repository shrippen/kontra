.PHONY: check fmt lint test deny advisories design video licences licences-check korpus run cli qt-env

# Everything CI runs; run before every commit (agent.md).
check: fmt lint test deny design video licences-check

fmt:
	cargo fmt --all --check

lint:
	cargo clippy --workspace --all-targets -- -D warnings

test:
	cargo test --workspace

# Licence policy (deny.toml); `cargo install --locked cargo-deny` once.
deny:
	cargo deny check licenses bans sources

# Known vulnerabilities (needs the network; CI: .github/workflows/advisories.yml).
advisories:
	cargo deny check advisories

# Kante only: no own colours, sizes or fonts in the app's QML (LH-NF-10).
design:
	scripts/check-design.sh

# Course videos play with subtitles (LH-LRN-14); needs Qt Multimedia.
video:
	scripts/check-video.sh

# Licence list in the About section (LH-NF-11); regenerate after dependency changes.
licences:
	scripts/licences.py

licences-check:
	scripts/licences.py --check

# Detection quality on the synthetic corpus (LH-NF-02/03).
korpus:
	cargo run -q -p kontra-cli --release -- korpus check

cli:
	cargo build -p kontra-cli --release

# GUI (needs Qt 6.10 + Kirigami: scripts/setup-qt-env.sh, then `source scripts/qt-env.sh`).
run:
	cargo run -p kontra-app

qt-env:
	sudo scripts/setup-qt-env.sh
