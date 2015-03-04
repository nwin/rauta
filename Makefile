release:
	cargo build --release
run:
	cargo run --release
debug:
	RUST_BACKTRACE=1 RUST_LOG=rauta=debug cargo run
check:
	rustc --no-trans src/main.rs
