debug:
	cargo build && RUST_LOG=debug ./target/rauta
run:
	cargo build && ./target/rauta
check:
	rustc --no-trans src/main.rs
