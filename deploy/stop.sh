#!/usr/bin/env bash
set -e
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/common.sh"
log_info "停止 rust-shark..."
cd "$SCRIPT_DIR"
$DOCKER_COMPOSE down
log_info "已停止"
