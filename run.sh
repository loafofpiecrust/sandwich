#!/usr/bin/env bash

# Run the project, directing to the default display.
# This allows us to get sandwich going over ssh.
RUST_BACKTRACE=1 DISPLAY=:0 cargo run

