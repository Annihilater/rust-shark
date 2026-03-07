#!/bin/bash
# 停止前端（通过 pid 文件、进程名、端口三重查找）
FRONTEND_PORT=18934
PID_FILE="/tmp/rust-shark-frontend.pid"
STOPPED=0

# 1. 通过 pid 文件
if [ -f "$PID_FILE" ]; then
    PID=$(cat "$PID_FILE")
    if kill -0 "$PID" 2>/dev/null; then
        echo "⏹ 停止前端进程 (PID $PID)"
        kill "$PID"
        rm -f "$PID_FILE"
        STOPPED=1
    else
        rm -f "$PID_FILE"
    fi
fi

# 2. 通过进程名
PIDS=$(pgrep -f "trunk serve" 2>/dev/null || true)
if [ -n "$PIDS" ]; then
    echo "⏹ 停止残留 trunk 进程: $PIDS"
    kill $PIDS 2>/dev/null || true
    STOPPED=1
fi

# 3. 通过端口
PORT_PID=$(lsof -ti:$FRONTEND_PORT 2>/dev/null || true)
if [ -n "$PORT_PID" ]; then
    echo "⏹ 释放端口 $FRONTEND_PORT (PID $PORT_PID)"
    kill $PORT_PID 2>/dev/null || true
    STOPPED=1
fi

if [ $STOPPED -eq 1 ]; then
    echo "✓ 前端已停止"
else
    echo "ℹ 前端未在运行"
fi
