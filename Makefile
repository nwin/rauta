release:
	cargo build --release
run:
	cargo run --release
debug:
	RUST_LOG=rauta=debug cargo run
check:
	rustc --no-trans src/main.rs
