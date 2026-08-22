#!/usr/bin/env bash

set -xe

setback_env="${ENV:-dev}"
if [ "$setback_env" == "dev" ]; then
	cargo_profile="debug"
elif [ "$setback_env" == "prod" ]; then
	cargo_profile="release"
else
	echo "Environment variable 'ENV' must be either 'dev' or 'prod'"
	exit 1
fi

CARGO_METADATA=$(cargo metadata --no-deps --format-version=1)

setback_docker_build_and_push() {
	# $1 = package
	if [ -z "$1" ]; then
		echo "please specify package name"
		exit 1
	fi
	# $2 = port
	if [ -z "$1" ]; then
		echo "please specify port number"
		exit 1
	fi

	VERSION=$(echo "$CARGO_METADATA" | jq -r --arg name "$1" '.packages[] | select(.name == $name) | .version')

	docker build \
		--platform linux/arm64 \
		-t eclipse909/private:setback_"$1"-"$setback_env"-v"$VERSION" \
		--build-arg PROFILE="$cargo_profile" \
		--build-arg PACKAGE=setback_"$1" \
		--build-arg PORT="$2" \
		.

	docker push eclipse909/private:setback_"$1"-"$setback_env"-v"$VERSION"
}

#                             <package> <port>
setback_docker_build_and_push "server"  "42069" &
setback_docker_build_and_push "api"     "8080"  &

wait