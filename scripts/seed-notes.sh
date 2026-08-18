#!/bin/sh
# SPDX-License-Identifier: MPL-2.0

set -eu

if [ "$#" -ne 2 ]; then
    echo "usage: $0 <puckctl> <document.puck>" >&2
    exit 2
fi

puckctl=$1
document=$2

"$puckctl" document new "$document"
"$puckctl" document note add "$document" 'Welcome to Puck
This document contains sample notes for development.'
"$puckctl" document note add "$document" 'Router
Address: 192.168.1.1'
"$puckctl" document note add "$document" 'Database
Port: 5432
Backups: daily'
"$puckctl" document note add "$document" 'Remember to test search with punctuation: alpha-01, café, 🦀.'
