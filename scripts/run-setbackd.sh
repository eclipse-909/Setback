#!/usr/bin/env bash

set -xe

setback_env="${ENV:-dev}"

docker compose -f compose.yml -f compose."$setback_env".yml --env-file .env."$setback_env" up -d