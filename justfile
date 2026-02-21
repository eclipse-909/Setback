CARGO := "cargo +nightly"

default:
	@just --list

cloc:
	cloc \
		--include-ext=rs,js,sh,sql,html,css,toml,yml,json,md \
		--exclude-dir=target,.idea,.git,.zed \
		--not-match-d=".*\.bak" \
		.

test package="all":
	if [[ "{{package}}" == "all" ]]; then
	{{CARGO}} test --workspace
	else
	{{CARGO}} test -p "{{package}}"
	fi

codecov package="all":
	if [[ "{{package}}" == "all" ]]; then
	{{CARGO}} tarpaulin --workspace
	else
	{{CARGO}} tarpaulin -p "{{package}}"
	fi

build package="all":
	if [[ "{{package}}" == "all" ]]; then
	{{CARGO}} build --workspace
	else
	{{CARGO}} build -p "{{package}}"
	fi

# Run setback game
run:
	{{CARGO}} run -p setback

# Interface to locally-running server container
# Only run this once. The container may get updated so watch out for that.
server-create:
	mkdir -p "$XDG_DATA_HOME/setback-dev/.local/bin"
	cp ./scripts/setup_server-dev.sh "$XDG_DATA_HOME/setback-dev/.local/bin"
	distrobox assemble create
	distrobox enter -n setback-dev -r -- /bin/bash "$HOME/.local/bin/setup_server-dev.sh"

# Interface to locally-running server container
server-up:
	podman compose up -d
	distrobox enter -n setback-dev -T

# Interface to locally-running server container
server-deploy:
	./scripts/deploy-dev.sh

# Interface to locally-running server container
server-down:
	distrobox-stop setback-dev
	podman compose down