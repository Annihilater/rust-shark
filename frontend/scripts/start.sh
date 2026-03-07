#!/bin/bash
# 后台启动前端开发服务器
cd "$(dirname "$0")/.."

PID_FILE="/tmp/rust-shark-frontend.pid"

if [ -f "$PID_FILE" ] && kill -0 "$(cat $PID_FILE)" 2>/dev/null; then
    echo "⚠ 前端已在运行 (PID $(cat $PID_FILE))"
    exit 0
fi

echo "▶ 后台启动前端 http://localhost:18934"
nohup trunk serve > /tmp/rust-shark-frontend.log 2>&1 &
echo $! > "$PID_FILE"
echo "✓ 前端已启动 (PID $!), 日志: /tmp/rust-shark-frontend.log"
