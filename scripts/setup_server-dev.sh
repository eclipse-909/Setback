#!/bin/bash

# Sets up the backend server.
# Creates users and groups without passwords.

set -e
if [ $(id -u) -ne 0 ]; then
	echo Please run this script as root or using sudo!
	exit
fi

useradd -Umr setback
useradd -Um cdeploy

./setup_server.sh