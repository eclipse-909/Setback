#!/bin/bash

# Installs the Setback backend services
# Assumes users setback and cdeploy exist.
# Assumes groups setback and cdeploy exist.

UNIT_DIR=/etc/systemd/system
SETBACK_BIN_DIR=/var/lib/setback/bin
SETBACK_LIB_DIR=/var/lib/setback/lib
LOCAL_BIN_DIR=/usr/local/bin
LOCAL_LIB_DIR=/usr/local/lib
STAGE_DIR=/var/lib/setback/stage

mkdir -p -m 700 "$BIN_DIR"
mkdir -p -m 420 "$STAGE_DIR"
chown -R setback:root "$BIN_DIR"
chown -R setback:cdeploy "$STAGE_DIR"