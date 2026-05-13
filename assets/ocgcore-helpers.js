// OCG helpers for wasm-bindgen interop
let moduleInstance = null;
let cardRegistry = new Map();
let callbackPointers = null;

function ensureCallbackPointers() {
  if (!moduleInstance) {
    throw new Error("OCGCore not initialized");
  }

  if (callbackPointers) {
    return callbackPointers;
  }

  const cardReader = moduleInstance.addFunction((payload, code, dataPtr) => {
    void payload;
    if (!dataPtr) {
      return;
    }

    const card = cardRegistry.get(code >>> 0);
    if (!card) {
      return;
    }

    const buffer = moduleInstance.wasmMemory.buffer;
    const view = new DataView(buffer);
    const start = dataPtr >>> 0;
    new Uint8Array(buffer).fill(0, start, start + 56);

    view.setUint32(start + 0, code >>> 0, true);
    view.setUint32(start + 4, 0, true);
    view.setUint32(start + 8, 0, true);
    view.setUint32(start + 12, card.type >>> 0, true);
    view.setUint32(start + 16, card.level >>> 0, true);
    view.setUint32(start + 20, card.attribute >>> 0, true);
    view.setUint32(start + 24, card.race >>> 0, true);
    view.setUint32(start + 28, 0, true);
    view.setInt32(start + 32, card.atk | 0, true);
    view.setInt32(start + 36, card.def | 0, true);
    view.setUint32(start + 40, 0, true);
    view.setUint32(start + 44, 0, true);
    view.setUint32(start + 48, 0, true);
  }, "viii");

  const scriptReader = moduleInstance.addFunction((payload, duel, namePtr) => {
    void payload;
    void duel;
    void namePtr;
    return 0;
  }, "iiii");

  const logHandler = moduleInstance.addFunction((payload, stringPtr, type) => {
    void payload;
    void stringPtr;
    void type;
  }, "viii");

  const cardReaderDone = moduleInstance.addFunction((payload, dataPtr) => {
    void payload;
    void dataPtr;
  }, "vii");

  callbackPointers = {
    cardReader,
    scriptReader,
    logHandler,
    cardReaderDone,
  };

  return callbackPointers;
}

export function initOCGCore(module) {
  moduleInstance = module;
  cardRegistry = new Map();
  callbackPointers = null;
}

export function getOCGVersion() {
  if (!moduleInstance) {
    throw new Error("OCGCore not initialized. Call initOCGCore first.");
  }

  const major_ptr = moduleInstance._malloc(4);
  const minor_ptr = moduleInstance._malloc(4);

  moduleInstance._OCG_GetVersion(major_ptr, minor_ptr);

  const heap32 = new Int32Array(moduleInstance.wasmMemory.buffer);
  const major = heap32[major_ptr / 4];
  const minor = heap32[minor_ptr / 4];

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
  const pointers = ensureCallbackPointers();

  const optionsSize = 104;
  const options_ptr = moduleInstance._malloc(optionsSize);

  const heap8 = new Uint8Array(moduleInstance.wasmMemory.buffer, options_ptr, optionsSize);
  heap8.fill(0);

  const f64arr = new Float64Array(moduleInstance.wasmMemory.buffer, options_ptr, 4);
  f64arr[0] = seed1;
  f64arr[1] = seed2;
  f64arr[2] = seed3;
  f64arr[3] = seed4;

  const heap32 = new Int32Array(moduleInstance.wasmMemory.buffer, options_ptr, 24);
  heap32[8] = 0;
  heap32[10] = 8000;
  heap32[11] = 5;
  heap32[12] = 1;
  heap32[13] = 8000;
  heap32[14] = 5;
  heap32[15] = 1;

  const callbackSlots = new Uint32Array(moduleInstance.wasmMemory.buffer, options_ptr + 64, 8);
  callbackSlots[0] = pointers.cardReader;
  callbackSlots[1] = 0;
  callbackSlots[2] = pointers.scriptReader;
  callbackSlots[3] = 0;
  callbackSlots[4] = pointers.logHandler;
  callbackSlots[5] = 0;
  callbackSlots[6] = pointers.cardReaderDone;
  callbackSlots[7] = 0;

  new Uint8Array(moduleInstance.wasmMemory.buffer, options_ptr + 96, 1)[0] = 0;

  const duel_ptr = moduleInstance._malloc(4);
  const status = moduleInstance._OCG_CreateDuel(duel_ptr, options_ptr);

  const heap32_read = new Int32Array(moduleInstance.wasmMemory.buffer);
  const duel = heap32_read[duel_ptr / 4];

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
      6: "NULL_RNG_SEED",
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

  const info_ptr = moduleInstance._malloc(32);
  const heap8 = new Uint8Array(moduleInstance.wasmMemory.buffer, info_ptr, 32);
  heap8.fill(0);

  const heap32 = new Int32Array(moduleInstance.wasmMemory.buffer, info_ptr, 8);
  heap8[0] = team;
  heap8[1] = duelist;
  heap32[1] = code;
  heap8[8] = controller;
  heap32[3] = location;
  heap32[4] = sequence;
  heap32[5] = position || 1;

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

  const length_ptr = moduleInstance._malloc(4);
  const msg_ptr = moduleInstance._OCG_DuelGetMessage(duel, length_ptr);

  if (msg_ptr === 0) {
    moduleInstance._free(length_ptr);
    return null;
  }

  const heap32 = new Int32Array(moduleInstance.wasmMemory.buffer);
  const length = heap32[length_ptr / 4];
  const message = new Uint8Array(moduleInstance.wasmMemory.buffer, msg_ptr, length).slice();

  moduleInstance._free(length_ptr);

  return message;
}

// OCG_DuelSetResponse
export function duelSetResponse(duel, buffer) {
  if (!moduleInstance) throw new Error("OCGCore not initialized");

  const buf_ptr = moduleInstance._malloc(buffer.length);
  new Uint8Array(moduleInstance.wasmMemory.buffer, buf_ptr, buffer.length).set(buffer);

  moduleInstance._OCG_DuelSetResponse(duel, buf_ptr, buffer.length);
  moduleInstance._free(buf_ptr);
}

// OCG_LoadScript
export function loadScript(duel, scriptContent, scriptName) {
  if (!moduleInstance) throw new Error("OCGCore not initialized");

  const contentBytes = new TextEncoder().encode(scriptContent);
  const nameBytes = new TextEncoder().encode(scriptName);

  const content_ptr = moduleInstance._malloc(contentBytes.length);
  const name_ptr = moduleInstance._malloc(nameBytes.length + 1);

  new Uint8Array(moduleInstance.wasmMemory.buffer, content_ptr, contentBytes.length).set(contentBytes);
  new Uint8Array(moduleInstance.wasmMemory.buffer, name_ptr, nameBytes.length).set(nameBytes);
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
  const info_ptr = moduleInstance._malloc(20);

  const heap32 = new Int32Array(moduleInstance.wasmMemory.buffer, info_ptr, 5);
  heap32[0] = flags;

  const heap8 = new Uint8Array(moduleInstance.wasmMemory.buffer, info_ptr + 4, 1);
  heap8[0] = controller;

  heap32[2] = location;
  heap32[3] = sequence;
  heap32[4] = 0;

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
  const info_ptr = moduleInstance._malloc(20);

  const heap32 = new Int32Array(moduleInstance.wasmMemory.buffer, info_ptr, 5);
  heap32[0] = flags;

  const heap8 = new Uint8Array(moduleInstance.wasmMemory.buffer, info_ptr + 4, 1);
  heap8[0] = controller;

  heap32[2] = location;
  heap32[3] = 0;
  heap32[4] = 0;

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
  cardRegistry.set(code >>> 0, {
    type: type_ >>> 0,
    level: level >>> 0,
    attribute: attribute >>> 0,
    race: race >>> 0,
    atk: atk | 0,
    def: def | 0,
  });
}
