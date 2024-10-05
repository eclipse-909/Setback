#!/bin/bash

push() {
	branches=(main testing)
	if [ -z "$1" ]; then
		echo "Error: Github branch name is required. Branches include:"
        echo "${branches[@]}"
		return
	fi
	found=false
	for b in "${branches[@]}"; do
		if [ "$b" == "$1" ]; then
			found=true
		fi
	done
	if [ "$found" == false ]; then
		echo "Error: Unknown branch name provided. Branches include:"
		echo "${branches[@]}"
		return
	fi
	if [ -z "$2" ]; then
		echo "Must supply commit message"
		return
	fi
	git add .
	git commit -m "$2"
	git push origin main:"$1"
}

run_game_dev() {
	RUST_BACKTRACE=full cargo run -p game --features dev
}

run_game_release() {
	RUST_BACKTRACE=full cargo run -p game --release
}