#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(pwd)"
TOOLS_DIR="$ROOT_DIR/tools"
EMSDK_DIR="$TOOLS_DIR/emsdk"

mkdir -p "$TOOLS_DIR"

if [[ ! -d "$EMSDK_DIR" ]]; then
  git clone https://github.com/emscripten-core/emsdk.git "$EMSDK_DIR"
fi

pushd "$EMSDK_DIR" >/dev/null

EMSDK_VERSION="6.0.0"

./emsdk install "$EMSDK_VERSION"
./emsdk activate "$EMSDK_VERSION"

popd >/dev/null

echo "Installed emsdk $EMSDK_VERSION to $EMSDK_DIR"
