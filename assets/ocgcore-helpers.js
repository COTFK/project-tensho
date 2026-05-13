// OCG helpers for wasm-bindgen interop
let moduleInstance = null;

export function initOCGCore(module) {
  moduleInstance = module;
}

export function getOCGVersion() {
  if (!moduleInstance) {
    throw new Error("OCGCore not initialized. Call initOCGCore first.");
  }

  // Allocate memory in WASM linear memory for two i32s (4 bytes each)
  const major_ptr = moduleInstance._malloc(4);
  const minor_ptr = moduleInstance._malloc(4);

  // Call OCG_GetVersion
  moduleInstance._OCG_GetVersion(major_ptr, minor_ptr);

  // Read values from WASM memory
  const heap32 = new Int32Array(moduleInstance.wasmMemory.buffer);
  const major = heap32[major_ptr / 4];
  const minor = heap32[minor_ptr / 4];

  // Free allocated memory
  moduleInstance._free(major_ptr);
  moduleInstance._free(minor_ptr);

  return [major, minor];
}

// OCG_CreateDuel: returns duel handle (0 on failure)
export function createDuel(seed1, seed2, seed3, seed4) {
  if (!moduleInstance) {
    throw new Error("OCGCore not initialized");
  }

  const OCG_DUEL_CREATION_SUCCESS = 0;

  // Allocate OCG_DuelOptions struct
  // struct size: 8*4 (seeds) + 8 (flags) + 12*2 (players) + 8 (cardReader+payload1) + 8 (scriptReader+payload2) + 8 (logHandler+payload3) + 8 (cardReaderDone+payload4) + 1 (enableUnsafeLibraries)
  const optionsSize = 96;
  const options_ptr = moduleInstance._malloc(optionsSize);

  // Zero out the struct
  const heap8 = new Uint8Array(moduleInstance.wasmMemory.buffer, options_ptr, optionsSize);
  heap8.fill(0);

  // Set seeds (first 32 bytes: 4x uint64_t)
  const f64arr = new Float64Array(moduleInstance.wasmMemory.buffer, options_ptr, 4);
  f64arr[0] = seed1;
  f64arr[1] = seed2;
  f64arr[2] = seed3;
  f64arr[3] = seed4;

  // Set flags to 0 (offset 32 in struct)
  const heap32 = new Int32Array(moduleInstance.wasmMemory.buffer, options_ptr, 24);
  heap32[8] = 0;  // flags at offset 32

  // Set team1 starting LP to 8000 (offset 40, u32)
  heap32[10] = 8000;
  // Set team1 starting draw count to 5 (offset 44, u32)
  heap32[11] = 5;
  // Set team1 draw count per turn to 1 (offset 48, u32)
  heap32[12] = 1;

  // Set team2 starting LP to 8000 (offset 52, u32)
  heap32[13] = 8000;
  // Set team2 starting draw count to 5 (offset 56, u32)
  heap32[14] = 5;
  // Set team2 draw count per turn to 1 (offset 60, u32)
  heap32[15] = 1;

  // For now, we'll leave callbacks as NULL to trigger the appropriate error handling
  // or we can provide stub callbacks. Let's just try with NULLs first.

  // Allocate output duel pointer
  const duel_ptr = moduleInstance._malloc(4);

  // Call OCG_CreateDuelWithStubs (uses stub callbacks)
  const status = moduleInstance._OCG_CreateDuelWithStubs(duel_ptr, options_ptr);

  // Read the duel handle
  const heap32_read = new Int32Array(moduleInstance.wasmMemory.buffer);
  const duel = heap32_read[duel_ptr / 4];

  // Free temporary allocations
  moduleInstance._free(options_ptr);
  moduleInstance._free(duel_ptr);

  if (status !== OCG_DUEL_CREATION_SUCCESS) {
    const statusNames = {
      0: "SUCCESS",
      1: "NO_OUTPUT",
      2: "NOT_CREATED",
      3: "NULL_DATA_READER",
      4: "NULL_SCRIPT_READER",
      5: "INCOMPATIBLE_LUA_API",
      6: "NULL_RNG_SEED"
    };
    const statusName = statusNames[status] || "UNKNOWN";
    throw new Error(`OCG_CreateDuel failed with status ${status} (${statusName})`);
  }

  return duel;
}

// OCG_DestroyDuel
export function destroyDuel(duel) {
  if (!moduleInstance) throw new Error("OCGCore not initialized");
  moduleInstance._OCG_DestroyDuel(duel);
}

// OCG_DuelNewCard
export function duelNewCard(duel, team, duelist, code, controller, location, sequence, position) {
  if (!moduleInstance) throw new Error("OCGCore not initialized");

  // Allocate OCG_NewCardInfo struct
  const info_ptr = moduleInstance._malloc(32);

  // Zero out the entire struct
  const heap8 = new Uint8Array(moduleInstance.wasmMemory.buffer, info_ptr, 32);
  heap8.fill(0);

  // Now set fields
  const heap32 = new Int32Array(moduleInstance.wasmMemory.buffer, info_ptr, 8);

  heap8[0] = team;        // offset 0
  heap8[1] = duelist;     // offset 1
  heap32[1] = code;       // offset 4
  heap8[8] = controller;  // offset 8
  heap32[3] = location;   // offset 12
  heap32[4] = sequence;   // offset 16
  heap32[5] = position || 1;  // offset 20, default to 1 (face-up) if not specified

  moduleInstance._OCG_DuelNewCard(duel, info_ptr);
  moduleInstance._free(info_ptr);
}

// OCG_StartDuel
export function startDuel(duel) {
  if (!moduleInstance) throw new Error("OCGCore not initialized");
  moduleInstance._OCG_StartDuel(duel);
}

// OCG_DuelProcess: returns status code
export function duelProcess(duel) {
  if (!moduleInstance) throw new Error("OCGCore not initialized");
  return moduleInstance._OCG_DuelProcess(duel);
}

// OCG_DuelGetMessage: returns message buffer as Uint8Array or null
export function duelGetMessage(duel) {
  if (!moduleInstance) throw new Error("OCGCore not initialized");

  // Allocate length pointer
  const length_ptr = moduleInstance._malloc(4);

  // Call OCG_DuelGetMessage
  const msg_ptr = moduleInstance._OCG_DuelGetMessage(duel, length_ptr);

  if (msg_ptr === 0) {
    moduleInstance._free(length_ptr);
    return null;
  }

  // Read length
  const heap32 = new Int32Array(moduleInstance.wasmMemory.buffer);
  const length = heap32[length_ptr / 4];

  // Copy message data
  const message = new Uint8Array(moduleInstance.wasmMemory.buffer, msg_ptr, length).slice();

  moduleInstance._free(length_ptr);

  return message;
}

// OCG_DuelSetResponse
export function duelSetResponse(duel, buffer) {
  if (!moduleInstance) throw new Error("OCGCore not initialized");

  // Allocate and copy buffer
  const buf_ptr = moduleInstance._malloc(buffer.length);
  new Uint8Array(moduleInstance.wasmMemory.buffer, buf_ptr, buffer.length).set(buffer);

  moduleInstance._OCG_DuelSetResponse(duel, buf_ptr, buffer.length);
  moduleInstance._free(buf_ptr);
}

// OCG_LoadScript
export function loadScript(duel, scriptContent, scriptName) {
  if (!moduleInstance) throw new Error("OCGCore not initialized");

  // Encode strings first to get actual byte length
  const contentBytes = new TextEncoder().encode(scriptContent);
  const nameBytes = new TextEncoder().encode(scriptName);

  const content_ptr = moduleInstance._malloc(contentBytes.length);
  const name_ptr = moduleInstance._malloc(nameBytes.length + 1);

  // Copy encoded bytes to WASM memory
  new Uint8Array(moduleInstance.wasmMemory.buffer, content_ptr, contentBytes.length).set(contentBytes);
  new Uint8Array(moduleInstance.wasmMemory.buffer, name_ptr, nameBytes.length).set(nameBytes);
  // Write null terminator for name
  new Uint8Array(moduleInstance.wasmMemory.buffer, name_ptr + nameBytes.length, 1)[0] = 0;

  const result = moduleInstance._OCG_LoadScript(duel, content_ptr, contentBytes.length, name_ptr);

  moduleInstance._free(content_ptr);
  moduleInstance._free(name_ptr);

  return result;
}

// OCG_DuelQueryCount
export function duelQueryCount(duel, team, location) {
  if (!moduleInstance) throw new Error("OCGCore not initialized");
  return moduleInstance._OCG_DuelQueryCount(duel, team, location);
}

// OCG_DuelQuery
export function duelQuery(duel, flags, controller, location, sequence) {
  if (!moduleInstance) throw new Error("OCGCore not initialized");

  const length_ptr = moduleInstance._malloc(4);
  const info_ptr = moduleInstance._malloc(20); // OCG_QueryInfo struct size

  // Set QueryInfo fields
  const heap32 = new Int32Array(moduleInstance.wasmMemory.buffer, info_ptr, 5);
  heap32[0] = flags;

  const heap8 = new Uint8Array(moduleInstance.wasmMemory.buffer, info_ptr + 4, 1);
  heap8[0] = controller;

  heap32[2] = location;  // offset 8
  heap32[3] = sequence;  // offset 12
  heap32[4] = 0;        // overlay_seq, offset 16

  const result_ptr = moduleInstance._OCG_DuelQuery(duel, length_ptr, info_ptr);

  if (result_ptr === 0) {
    moduleInstance._free(length_ptr);
    moduleInstance._free(info_ptr);
    return null;
  }

  const heap32_read = new Int32Array(moduleInstance.wasmMemory.buffer);
  const length = heap32_read[length_ptr / 4];
  const data = new Uint8Array(moduleInstance.wasmMemory.buffer, result_ptr, length).slice();

  moduleInstance._free(length_ptr);
  moduleInstance._free(info_ptr);

  return data;
}

// OCG_DuelQueryLocation - get all cards in a location
export function duelQueryLocation(duel, flags, controller, location) {
  if (!moduleInstance) throw new Error("OCGCore not initialized");

  const length_ptr = moduleInstance._malloc(4);
  const info_ptr = moduleInstance._malloc(20); // OCG_QueryInfo struct size

  // Set QueryInfo fields
  const heap32 = new Int32Array(moduleInstance.wasmMemory.buffer, info_ptr, 5);
  heap32[0] = flags;

  const heap8 = new Uint8Array(moduleInstance.wasmMemory.buffer, info_ptr + 4, 1);
  heap8[0] = controller;

  heap32[2] = location;  // offset 8
  heap32[3] = 0;        // sequence (not used)
  heap32[4] = 0;        // overlay_seq

  const result_ptr = moduleInstance._OCG_DuelQueryLocation(duel, length_ptr, info_ptr);

  if (result_ptr === 0) {
    moduleInstance._free(length_ptr);
    moduleInstance._free(info_ptr);
    return null;
  }

  const heap32_read = new Int32Array(moduleInstance.wasmMemory.buffer);
  const length = heap32_read[length_ptr / 4];
  const data = new Uint8Array(moduleInstance.wasmMemory.buffer, result_ptr, length).slice();

  moduleInstance._free(length_ptr);
  moduleInstance._free(info_ptr);

  return data;
}

// OCG_RegisterCardData
export function registerCardData(code, type_, level, attribute, race, atk, def) {
  if (!moduleInstance) throw new Error("OCGCore not initialized");
  moduleInstance._OCG_RegisterCardData(code, type_, level, attribute, race, atk, def);
}
