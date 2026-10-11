#!/usr/bin/env bash

# TODO
# use dokploy cli to
# * update environment variable API_KEY with openssl
#   * openssl rand -base64 16 | tr -dc 'A-Za-z0-9'
# * update environment variable API_JWT_SECRET with openssl
#   * openssl rand -base64 64
# * deploy the app
#   * dokploy compose deploy --composeId app123