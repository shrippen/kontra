.PHONY: check fmt lint test deny design korpus run cli qt-env

# Everything CI runs; run before every commit (agent.md).
check: fmt lint test deny design

fmt:
	cargo fmt --all --check

lint:
	cargo clippy --workspace --all-targets -- -D warnings

test:
	cargo test --workspace

# Licence policy (deny.toml); `cargo install --locked cargo-deny` once.
deny:
	cargo deny check licenses bans sources

# Kante only: no own colours, sizes or fonts in the app's QML (LH-NF-10).
design:
	scripts/check-design.sh

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
