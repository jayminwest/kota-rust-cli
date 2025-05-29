#!/bin/bash
# run_kota.sh - Wrapper script for KOTA with auto-rebuild/restart on self-modification

SUCCESS_EXIT_CODE=0 # Normal exit
RESTART_REQUEST_CODE=123 # Special exit code for KOTA to request a restart

while true; do
    echo "Building KOTA Rust tool..."
    cargo build --quiet
    if [ $? -eq 0 ]; then
        echo "Build successful. Starting KOTA..."
        ./target/debug/kota-rust-cli # Using the actual binary name
        
        exit_code=$?
        if [ $exit_code -eq $RESTART_REQUEST_CODE ]; then
            echo ""
            echo "KOTA requested restart after self-modification. Rebuilding and restarting..."
            echo ""
            # Loop continues
        elif [ $exit_code -eq $SUCCESS_EXIT_CODE ]; then
            echo "KOTA exited normally."
            break # Exit loop for normal termination
        else
            echo "KOTA exited with code $exit_code."
            break # Exit loop for other errors
        fi
    else
        echo "Build failed. Please fix errors and restart manually."
        break # Exit loop on build failure
    fi
    sleep 1 # Small delay before restarting
done