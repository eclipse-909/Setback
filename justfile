set dotenv-load
compose := "sudo docker compose -f local/docker-compose.yml"
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

# start compose
dbup:
	{{compose}} up -d --wait

# stop compose
dbdown:
	{{compose}} down

# sqlx prepare allows offline compilation with sql queries
dbprep:
	cargo sqlx prepare --workspace

# reset database and run api + local migrations
dbreset:
	sqlx database reset --source api/migrations/
	sqlx migrate run --source local/migrations/

# run the api
api profile="dev":
	cargo run -p api --profile {{profile}}

# run the server
server profile="dev":
	cargo run -p server -- profile {{profile}}

# Check coverage; same check as CI
chcov:
	#!/usr/bin/env bash
	cargo llvm-cov --workspace --all-targets --locked \
		--fail-under-lines {{cov_threshold}} \
		--fail-under-functions {{cov_threshold}} \
		--fail-under-regions {{cov_threshold}}

# Generate HTML coverage report in ./coverage/html
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