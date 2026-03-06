#!/usr/bin/env bash
set -e
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/common.sh"
"$SCRIPT_DIR/stop.sh"
"$SCRIPT_DIR/start.sh"
