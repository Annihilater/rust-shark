#!/bin/bash
# 查看前端状态
FRONTEND_PORT=18934

echo "=== 前端状态 ==="

PIDS=$(pgrep -f "trunk serve" 2>/dev/null || true)
if [ -n "$PIDS" ]; then
    echo "✓ 进程运行中 (PID: $PIDS)"
else
    echo "✗ 进程未运行"
fi

PORT_PID=$(lsof -ti:$FRONTEND_PORT 2>/dev/null || true)
if [ -n "$PORT_PID" ]; then
    echo "✓ 端口 $FRONTEND_PORT 监听中 (PID: $PORT_PID)"
else
    echo "✗ 端口 $FRONTEND_PORT 未监听"
fi
