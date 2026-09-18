CARGO := "cargo +nightly"

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

# run setbackd with docker compose
setbackd:
	docker compose -f compose/compose.local.yml up -d --build
	xdg-open http://localhost:8080/ # Traefik dashboard
	xdg-open http://localhost:8081  # phpmyadmin
	xdg-open https://localhost      # setback_api

build-lib $RUSTFLAGS="-Awarnings":
	{{CARGO}} build -q -p libsetback
	{{CARGO}} build -q -p libsetback --target wasm32-unknown-unknown
	{{CARGO}} build -q -p libsetback -F server

# generate self-signed certificates (necessary for running setbackd locally)
gencerts:
	#!/usr/bin/env bash
	set -xe
	CERTS_DIR="./compose/certs"
	rm -rf "$CERTS_DIR"
	mkdir -p "$CERTS_DIR"
	openssl req -x509 -newkey rsa:4096 -days 365 -nodes \
		-keyout "$CERTS_DIR/key.pem" \
		-out    "$CERTS_DIR/cert.pem" \
		-subj   "/CN=localhost" \
		-addext "subjectAltName=DNS:localhost,IP:127.0.0.1"
	echo "Done! Certificates written to $CERTS_DIR"