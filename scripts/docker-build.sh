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
IMAGES=( "setback_server" "setback_website" )

for IMG in "${IMAGES[@]}"; do
    (
        set -e
        VERSION=$(echo "$CARGO_METADATA" | jq -r --arg name "$IMG" '.packages[] | select(.name == $name) | .version')
        docker build \
            --platform linux/arm64 \
            -t eclipse909/private:"$IMG"-"$setback_env"-v"$VERSION" \
            --build-arg profile="$cargo_profile" \
            "$IMG"/
        docker push eclipse909/private:"$IMG"-"$setback_env"-v"$VERSION"
    ) &
done

wait