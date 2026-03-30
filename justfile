CARGO := "cargo +nightly"

default:
	@just --list

cloc:
	cloc \
		--include-ext=rs,js,sh,sql,html,css,toml,yml,json,md \
		--exclude-dir=target,.idea,.git,.zed \
		--not-match-d=".*\.bak" \
		.

cargo cmd package="all":
	if [[ "{{package}}" == "all" ]]; then
	{{CARGO}} {{cmd}} --workspace
	else
	{{CARGO}} {{cmd}} -p "{{package}}"
	fi

docker-build image tag:
	docker build \
		--platform linux/amd64,linux/arm64 \
		-t eclipse909/private:{{image}}-{{tag}} \
		{{image}}/
	docker push eclipse909/private:{{image}}-{{tag}} &

docker-build-release image version:
	docker build \
		--platform linux/amd64,linux/arm64 \
		-t eclipse909/private:{{image}}-latest \
		{{image}}/
	docker tag eclipse909/private:{{image}}-latest eclipse909/private:{{image}}-v{{version}}
	docker push eclipse909/private:{{image}}-latest &

run-game:
	{{CARGO}} run -p setback

run-server tag="latest":
	docker run -d \
		--name setback_server \
		--network setback-services_default \
		eclipse909/private:setback_server-{{tag}}
	xdg-open http://127.0.0.1:8081
	xdg-open https://127.0.0.1

gencerts:
	set -e
	CERTS_DIR="./certs"
	mkdir -p "$CERTS_DIR"
	openssl req -x509 -newkey rsa:4096 -days 365 -nodes \
		-keyout "$CERTS_DIR/key.pem" \
		-out    "$CERTS_DIR/cert.pem" \
		-subj   "/CN=localhost" \
		-addext "subjectAltName=DNS:localhost,IP:127.0.0.1"
	echo "Certificates written to $CERTS_DIR"