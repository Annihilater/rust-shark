#!/usr/bin/env bash
set -e
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/common.sh"
mkdir -p "$SCRIPT_DIR/data/captures"
log_info "拉取最新镜像..."
cd "$SCRIPT_DIR"
$DOCKER_COMPOSE pull
log_info "启动 rust-shark..."
$DOCKER_COMPOSE up -d
log_info "启动完成"
