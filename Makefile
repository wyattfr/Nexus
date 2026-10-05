default: build

build:
	cargo build

test:
	cargo test

run:
	cargo run --bin nexus -- demo

dev:
	cargo watch -x "run --bin nexus -- demo"

check:
	cargo check

fmt:
	cargo fmt

install:
	cargo install --path crates/nexus-cli --force

uninstall:
	cargo uninstall nexus

demo:
	nexus demo
