use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    let out_dir = env::var("OUT_DIR").expect("OUT_DIR not set");
    let ocgcore_dir = Path::new("ocgcore");
    let lua_dir = ocgcore_dir.join("lua").join("src");

    println!("cargo:rerun-if-changed={}", ocgcore_dir.display());
    println!("cargo:rerun-if-changed={}", lua_dir.display());
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());

    println!("cargo:rerun-if-env-changed=EMSDK");

    let mut candidates = Vec::new();
    if let Ok(path) = env::var("EMSDK") {
        candidates.push(PathBuf::from(path));
    }
    candidates.push(manifest_dir.join("tools").join("emsdk"));

    let emsdk = candidates
        .into_iter()
        .find(|path| {
            path.join("upstream")
                .join("emscripten")
                .join("em++")
                .exists()
        })
        .unwrap_or_else(|| panic!("emsdk not found. Install it under tools/emsdk or set EMSDK"));

    let empp = emsdk.join("upstream").join("emscripten").join("em++");
    if !empp.exists() {
        panic!("em++ not found at {}", empp.display());
    }

    let out_dir = PathBuf::from(out_dir);
    let output_js = out_dir.join("ocgcore.js");
    let mut cmd = Command::new(empp);
    cmd.arg("-std=c++17")
        .arg("-O2")
        .arg("-fexceptions")
        .arg("-sMODULARIZE=1")
        .arg("-sEXPORT_ES6=1")
        .arg("-sEXPORT_NAME=ocgcore")
        .arg("-sEXPORTED_FUNCTIONS=['_OCG_GetVersion','_OCG_CreateDuel','_OCG_DestroyDuel','_OCG_DuelNewCard','_OCG_StartDuel','_OCG_DuelProcess','_OCG_DuelGetMessage','_OCG_DuelSetResponse','_OCG_LoadScript','_OCG_DuelQueryCount','_OCG_DuelQuery','_OCG_DuelQueryLocation','_malloc','_free']")
        .arg("-sEXPORTED_RUNTIME_METHODS=['getValue','setValue','ccall','cwrap','wasmMemory','addFunction']")
        .arg("-sALLOW_MEMORY_GROWTH=1")
        .arg("-sALLOW_TABLE_GROWTH=1")
        .arg("-sENVIRONMENT=web")
        .arg("-I")
        .arg(lua_dir.as_os_str())
        .arg("-I")
        .arg(ocgcore_dir.as_os_str())
        .arg("-DMAKE_LIB")
        .arg("-o")
        .arg(&output_js);

    let cpp_sources = [
        "card.cpp",
        "duel.cpp",
        "effect.cpp",
        "field.cpp",
        "interpreter.cpp",
        "libcard.cpp",
        "libdebug.cpp",
        "libduel.cpp",
        "libeffect.cpp",
        "libgroup.cpp",
        "ocgapi.cpp",
        "operations.cpp",
        "playerop.cpp",
        "processor.cpp",
        "processor_visit.cpp",
        "scriptlib.cpp",
    ];

    for src in cpp_sources {
        cmd.arg(ocgcore_dir.join(src));
    }

    cmd.arg(lua_dir.join("onelua.c"));
    
    let status = cmd.status().expect("failed to invoke em++");
    if !status.success() {
        panic!("em++ failed with status {status}");
    }

    let assets_dir = manifest_dir.join("assets");
    fs::create_dir_all(&assets_dir).expect("failed to create assets directory");
    let assets_wasm = assets_dir.join("ocgcore.wasm");
    let assets_js = assets_dir.join("ocgcore.js");
    let output_wasm = out_dir.join("ocgcore.wasm");
    fs::copy(&output_wasm, &assets_wasm).expect("failed to copy ocgcore.wasm to assets");
    fs::copy(&output_js, &assets_js).expect("failed to copy ocgcore.js to assets");
}
