#!/bin/bash
# 查看后端状态
BACKEND_PORT=17823

echo "=== 后端状态 ==="

PIDS=$(pgrep -f "target/debug/rust-shark" 2>/dev/null || true)
if [ -n "$PIDS" ]; then
    echo "✓ 进程运行中 (PID: $PIDS)"
else
    echo "✗ 进程未运行"
fi

PORT_PID=$(lsof -ti:$BACKEND_PORT 2>/dev/null || true)
if [ -n "$PORT_PID" ]; then
    echo "✓ 端口 $BACKEND_PORT 监听中 (PID: $PORT_PID)"
else
    echo "✗ 端口 $BACKEND_PORT 未监听"
fi

HEALTH=$(curl -s --max-time 2 http://localhost:$BACKEND_PORT/api/health 2>/dev/null || true)
if [ -n "$HEALTH" ]; then
    echo "✓ 健康检查: $HEALTH"
else
    echo "✗ 健康检查失败"
fi
