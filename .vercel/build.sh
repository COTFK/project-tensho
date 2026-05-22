#!/bin/sh

curl https://sh.rustup.rs -sSf | sh -s -- -y
source "/rust/env"
curl -sSL https://dioxus.dev/install.sh | bash
/rust/bin/dx bundle --platform web --debug-symbols=false --out-dir bundle