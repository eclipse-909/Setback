set dotenv-path := '.env.local'
compose := "docker compose -f ./local/docker-compose.yml"
cov_threshold := "80"

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

prepare:
	cargo sqlx prepare --workspace

# start compose
dbup:
	{{compose}} up -d --wait

# run the api
api:
	cargo run -p api

# run the server
server:
	cargo run -p server

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

# Same check as CI
cov-check:
	#!/usr/bin/env bash
	cargo llvm-cov --workspace --all-targets --locked \
		--fail-under-lines {{cov_threshold}} \
		--fail-under-functions {{cov_threshold}} \
		--fail-under-regions {{cov_threshold}}

# Generate HTML report in ./coverage/html
cov:
    cargo llvm-cov --workspace --all-targets --html --output-dir local/coverage
    @echo "Report: local/coverage/html/index.html"

# Generate the report and open it in your browser
cov-open:
    cargo llvm-cov --workspace --all-targets --html --output-dir local/coverage --open

# generate self-signed certificates (necessary for running setbackd locally)
gencerts:
	#!/usr/bin/env bash
	set -xe
	CERTS_DIR="./local/certs"
	rm -rf "$CERTS_DIR"
	mkdir -p "$CERTS_DIR"
	openssl req -x509 -newkey rsa:4096 -days 365 -nodes \
		-keyout "$CERTS_DIR/key.pem" \
		-out    "$CERTS_DIR/cert.pem" \
		-subj   "/CN=localhost" \
		-addext "subjectAltName=DNS:localhost,IP:127.0.0.1"
	echo "Done! Certificates written to $CERTS_DIR"