#!/bin/bash
cargo run /Users/davidnascimento/Downloads/pokemon-red.gb &
EMULATOR_PID=$!
sleep 3
kill $EMULATOR_PID
wait $EMULATOR_PID 2>/dev/null
echo "Emulator stopped after 3 seconds"

