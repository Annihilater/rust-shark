#!/bin/bash
# 前台启动前端开发服务器（带热更新）
set -e
cd "$(dirname "$0")/.."
echo "▶ 启动前端开发服务器 http://localhost:18934"
trunk serve
