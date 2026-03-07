#!/bin/bash
# 前台启动后端（开发模式，实时日志）
set -e
cd "$(dirname "$0")/.."
echo "▶ 启动后端 (前台模式) http://localhost:17823"
cargo run
