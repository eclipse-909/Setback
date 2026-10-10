set dotenv-path := '.env.local'
compose := "docker compose -f compose.local.yml"

# list available recipes
default:
	@just --list

# count lines of code
cloc:
	cloc \
		--include-ext=rs,js,sh,sql,html,css,toml,yml,json,md \
		--exclude-dir=target,.idea,.git,.zed,.vscode,node_modules,dist \
		--not-match-d=".*\.bak" \
		.

# start compose
dbup:
	{{compose}} up -d --wait

api:
	cargo run -p api

server:
	cargo run -p api

# run setbackd with docker compose
dev: dbup
	#!/usr/bin/env bash
    set -euo pipefail
	cargo build -p api -p server
	cleanup() {
        trap - EXIT
        trap 'kill -KILL $(jobs -p) 2>/dev/null || true; exit 130' INT TERM
        kill $(jobs -p) 2>/dev/null || true
        wait
    }
    trap cleanup EXIT
    trap 'exit 130' INT TERM
    cargo run -p api &
    sleep 1
    cargo run -p server &
	xdg-open http://localhost:8080/ # Traefik dashboard
	xdg-open http://localhost:8081  # phpmyadmin
	#xdg-open https://localhost      # website
    wait
    echo "db is still running, use `dbdown` to stop it"

# stop compose
dbdown:
	{{compose}} down

# check libsetback for compiler errors only, does not show warnings
check-lib $RUSTFLAGS="-Awarnings":
	cargo check -q -p libsetback --all-targets
	cargo check -q -p libsetback --target wasm32-unknown-unknown
	cargo check -q -p libsetback -F server

# generate self-signed certificates (necessary for running setbackd locally)
gencerts:
	#!/usr/bin/env bash
	set -xe
	CERTS_DIR="./docker/certs"
	rm -rf "$CERTS_DIR"
	mkdir -p "$CERTS_DIR"
	openssl req -x509 -newkey rsa:4096 -days 365 -nodes \
		-keyout "$CERTS_DIR/key.pem" \
		-out    "$CERTS_DIR/cert.pem" \
		-subj   "/CN=localhost" \
		-addext "subjectAltName=DNS:localhost,IP:127.0.0.1"
	echo "Done! Certificates written to $CERTS_DIR"