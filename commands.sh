#!/bin/bash

alias run_game='prime-run cargo run -p setback --profile game-debug'
alias cloc_setback='cloc --include-ext=rs,html,js,css,sql,sh,toml,md --exclude-dir=target,.idea,.git .'