#!/bin/bash
# Restart rwire example server
# Usage: ./restart-server.sh [example-name]
# Default: todo-combined

EXAMPLE=${1:-todo-combined}

fuser -k 9000/tcp 2>/dev/null
sleep 1
cargo run -p "$EXAMPLE" > /tmp/server.log 2>&1 &
sleep 2
cat /tmp/server.log
