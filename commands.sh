#!/bin/bash

push() {
	if [ -z "$1" ]; then
		echo "Must supply commit message"
		return
	fi
	git add .
	git commit -m "$1"
	git push origin main
}

run_game_dev() {
	RUST_BACKTRACE=full cargo run -p game --features "dev observable"
}

run_game_release() {
	RUST_BACKTRACE=full cargo run -p game --release --features "observable"
}