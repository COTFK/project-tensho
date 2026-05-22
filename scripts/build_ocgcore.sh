#!/usr/bin/env bash

set -euo pipefail

script_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
root_dir=$(cd -- "$script_dir/.." && pwd)

ocgcore_dir="$root_dir/ocgcore"
lua_dir="$ocgcore_dir/lua/src"
assets_dir="$root_dir/public"

candidate_emsdks=()
if [[ -n "${EMSDK:-}" ]]; then
    candidate_emsdks+=("$EMSDK")
fi
candidate_emsdks+=("$root_dir/tools/emsdk")

emsdk_dir=""
for candidate in "${candidate_emsdks[@]}"; do
    if [[ -x "$candidate/upstream/emscripten/em++" ]]; then
        emsdk_dir="$candidate"
        break
    fi
done

if [[ -z "$emsdk_dir" ]]; then
    echo "emsdk not found. Install it under tools/emsdk or set EMSDK" >&2
    exit 1
fi

empp="$emsdk_dir/upstream/emscripten/em++"
if [[ ! -x "$empp" ]]; then
    echo "em++ not found at $empp" >&2
    exit 1
fi

mkdir -p "$assets_dir"

"$empp" \
    -std=c++17 \
    -O2 \
    -fexceptions \
    -sMODULARIZE=1 \
    -sEXPORT_ES6=1 \
    -sEXPORT_NAME=ocgcore \
    -sEXPORTED_FUNCTIONS=['_OCG_GetVersion','_OCG_CreateDuel','_OCG_DestroyDuel','_OCG_DuelNewCard','_OCG_StartDuel','_OCG_DuelProcess','_OCG_DuelGetMessage','_OCG_DuelSetResponse','_OCG_LoadScript','_OCG_DuelQueryCount','_OCG_DuelQuery','_OCG_DuelQueryLocation','_malloc','_free'] \
    -sEXPORTED_RUNTIME_METHODS=['getValue','setValue','ccall','cwrap','wasmMemory','addFunction'] \
    -sALLOW_MEMORY_GROWTH=1 \
    -sALLOW_TABLE_GROWTH=1 \
    -sENVIRONMENT=web \
    -I \
    "$lua_dir" \
    -I \
    "$ocgcore_dir" \
    -DMAKE_LIB \
    -o \
    "$assets_dir/ocgcore.js" \
    "$ocgcore_dir/card.cpp" \
    "$ocgcore_dir/duel.cpp" \
    "$ocgcore_dir/effect.cpp" \
    "$ocgcore_dir/field.cpp" \
    "$ocgcore_dir/interpreter.cpp" \
    "$ocgcore_dir/libcard.cpp" \
    "$ocgcore_dir/libdebug.cpp" \
    "$ocgcore_dir/libduel.cpp" \
    "$ocgcore_dir/libeffect.cpp" \
    "$ocgcore_dir/libgroup.cpp" \
    "$ocgcore_dir/ocgapi.cpp" \
    "$ocgcore_dir/operations.cpp" \
    "$ocgcore_dir/playerop.cpp" \
    "$ocgcore_dir/processor.cpp" \
    "$ocgcore_dir/processor_visit.cpp" \
    "$ocgcore_dir/scriptlib.cpp" \
    "$lua_dir/onelua.c"