#!/bin/bash

# Sets up the backend server.
# Assumes users setback and cdeploy exist.
# Assumes groups setback and cdeploy exist.

set -e
if [ $(id -u) -ne 0 ]; then
	echo Please run this script as root or using sudo!
	exit
fi

mkdir -p -m 660 /var/lib/setback
chown -R setback:setback /var/lib/setback
mkdir /var/lib/setback/lib
mkdir -m 760 /var/lib/setback/bin