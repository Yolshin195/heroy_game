build-wasm:
	cargo build --release --target wasm32-unknown-unknown
	wasm-bindgen --out-dir ./out --target web ./target/wasm32-unknown-unknown/release/heroy_game.wasm

serve:
	python3 -m http.server 8000