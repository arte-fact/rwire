#!/bin/bash
fuser -k 9000/tcp 2>/dev/null
sleep 1
cargo run -p todo-combined > /tmp/server.log 2>&1 &
sleep 2
cat /tmp/server.log
