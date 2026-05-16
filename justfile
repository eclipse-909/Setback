CARGO := "cargo +nightly"

# list available recipes
default:
	@just --list

# count lines of code
cloc:
	cloc \
		--include-ext=rs,js,sh,sql,html,css,toml,yml,json,md \
		--exclude-dir=target,.idea,.git,.zed,.vscode \
		--not-match-d=".*\.bak" \
		.

# run a cargo command with the default cargo options and args
cargo cmd package="all":
	if [[ "{{package}}" == "all" ]]; then
	{{CARGO}} {{cmd}} --workspace
	else
	{{CARGO}} {{cmd}} -p "{{package}}"
	fi

# run the Setback game client
run-game: (cargo "run" "setback")

# run setbackd with docker compose
run-server:
	docker compose -f compose/compose.local.yml up -d --build
	xdg-open http://localhost:8080/ # Traefik dashboard
	xdg-open http://localhost:8081  # phpmyadmin
	xdg-open https://localhost      # setback_website

# generate self-signed certificates (necessary for running setbackd locally)
gencerts:
	#!/usr/bin/env bash
	set -xe
	CERTS_DIR="./certs"
	rm -rf "$CERTS_DIR"
	mkdir -p "$CERTS_DIR"
	openssl req -x509 -newkey rsa:4096 -days 365 -nodes \
		-keyout "$CERTS_DIR/key.pem" \
		-out    "$CERTS_DIR/cert.pem" \
		-subj   "/CN=localhost" \
		-addext "subjectAltName=DNS:localhost,IP:127.0.0.1"
	echo "Certificates written to $CERTS_DIR"