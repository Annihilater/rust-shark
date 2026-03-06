#!/usr/bin/env bash
set -e
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/common.sh"
log_info "拉取最新镜像..."
cd "$SCRIPT_DIR"
$DOCKER_COMPOSE pull
log_info "拉取完成"
