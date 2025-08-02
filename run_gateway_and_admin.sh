#!/usr/bin/env bash

set -e

# Start the gateway server
(cd gateway_server && cargo run) &
SERVER_PID=$!

# Start the admin web UI
(cd webui && npm run dev) &
WEB_PID=$!

trap "kill $SERVER_PID $WEB_PID" EXIT

wait $SERVER_PID $WEB_PID

