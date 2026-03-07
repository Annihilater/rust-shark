#!/bin/bash
# 重启前端
SCRIPT_DIR="$(dirname "$0")"
"$SCRIPT_DIR/stop.sh"
sleep 1
"$SCRIPT_DIR/start.sh"
