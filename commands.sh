#!/bin/bash

#call these from ./Setback in shell

push() {
	branches=(main)
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

alias build_game='cargo build -p game --profile=game-debug'
alias run_game='cargo run -p game --profile=game-debug'

alias cloc_setback='cloc --include-ext=rs,html,js,css,sql,sh,toml,md --exclude-dir=target,.idea,.git .'

alias docker-build-website='docker build -t setback-website -f ./website/Dockerfile .'
alias docker-run-website='docker run --rm -p 80:8080 setback-website'

alias docker-build-server='docker build -t setback-server -f ./server/Dockerfile .'
alias docker-run-server='docker run --rm -p 42069:42069 setback-server'