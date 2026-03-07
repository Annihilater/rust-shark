#!/bin/bash
# 后台启动后端
cd "$(dirname "$0")/.."

PID_FILE="/tmp/rust-shark-backend.pid"

if [ -f "$PID_FILE" ] && kill -0 "$(cat $PID_FILE)" 2>/dev/null; then
    echo "⚠ 后端已在运行 (PID $(cat $PID_FILE))"
    exit 0
fi

echo "▶ 后台启动后端 http://localhost:17823"
nohup cargo run > /tmp/rust-shark-backend.log 2>&1 &
echo $! > "$PID_FILE"
echo "✓ 后端已启动 (PID $!), 日志: /tmp/rust-shark-backend.log"
