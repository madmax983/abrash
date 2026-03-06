cargo run --example emboss_demo --features "nova parallel" &
PID=$!
sleep 5
kill $PID
