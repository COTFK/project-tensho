use wasm_bindgen::prelude::*;

#[wasm_bindgen(module = "/assets/ocgcore-helpers.js")]
extern "C" {
    pub fn initOCGCore(module: &JsValue);
    pub fn getOCGVersion() -> js_sys::Array;
    pub fn createDuel(seed1: f64, seed2: f64, seed3: f64, seed4: f64) -> u32;
    pub fn destroyDuel(duel: u32);
    pub fn duelNewCard(
        duel: u32,
        team: u8,
        duelist: u8,
        code: u32,
        controller: u8,
        location: u32,
        sequence: u32,
        position: u32,
    );
    pub fn startDuel(duel: u32);
    pub fn duelProcess(duel: u32) -> i32;
    pub fn duelGetMessage(duel: u32) -> Option<js_sys::Uint8Array>;
    pub fn duelSetResponse(duel: u32, buffer: &[u8]);
    pub fn loadScript(duel: u32, script_content: &str, script_name: &str) -> i32;
    pub fn duelQueryCount(duel: u32, team: u8, location: u32) -> u32;
    pub fn duelQuery(
        duel: u32,
        flags: u32,
        controller: u8,
        location: u32,
        sequence: u32,
    ) -> Option<js_sys::Uint8Array>;
    pub fn duelQueryLocation(
        duel: u32,
        flags: u32,
        controller: u8,
        location: u32,
    ) -> Option<js_sys::Uint8Array>;
}

pub async fn load_ocgcore() -> Result<(), String> {
    let import_fn = js_sys::Function::new_with_args("u", "return import(u);");
    let promise = import_fn
        .call1(&js_sys::global(), &"/assets/ocgcore.js".into())
        .map_err(|err| format!("dynamic import failed: {err:?}"))?;
    let promise: js_sys::Promise = promise
        .dyn_into()
        .map_err(|_| "import did not return a promise")?;
    let module = wasm_bindgen_futures::JsFuture::from(promise)
        .await
        .map_err(|err| format!("import await failed: {err:?}"))?;

    let default_export = js_sys::Reflect::get(&module, &"default".into())
        .map_err(|_| "missing default export")?;
    let init_fn: js_sys::Function = default_export
        .dyn_into()
        .map_err(|_| "default export is not a function")?;
    let init_promise = init_fn
        .call0(&js_sys::global())
        .map_err(|err| format!("module init failed: {err:?}"))?;
    let init_promise: js_sys::Promise = init_promise
        .dyn_into()
        .map_err(|_| "init did not return a promise")?;
    let module_obj = wasm_bindgen_futures::JsFuture::from(init_promise)
        .await
        .map_err(|err| format!("init await failed: {err:?}"))?;

    // Pass the initialized Module object to helpers
    initOCGCore(&module_obj);

    Ok(())
}

pub fn get_version() -> Result<(i32, i32), String> {
    let arr = getOCGVersion();
    let major = arr
        .get(0)
        .as_f64()
        .ok_or("major version not a number")? as i32;
    let minor = arr
        .get(1)
        .as_f64()
        .ok_or("minor version not a number")? as i32;
    Ok((major, minor))
}

pub fn create_duel_with_random_seed() -> Result<u32, String> {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let seed = [
        rng.r#gen::<u64>() as f64,
        rng.r#gen::<u64>() as f64,
        rng.r#gen::<u64>() as f64,
        rng.r#gen::<u64>() as f64,
    ];
    println!("Creating duel with seed");
    let duel = createDuel(seed[0], seed[1], seed[2], seed[3]);
    println!("Duel created: {}", duel);

    // Fire King deck - main deck (40 cards)
    let main_deck = [
        66431519u32, 66431519, 66431519, 23015896, 44455560, 44455560, 44455560, 90681088,
        90681088, 90681088, 18621798, 2526224, 2526224, 2526224, 14558128, 14558128,
        14558128, 97268402, 97268402, 97268402, 24508238, 33854624, 6637331, 65305978,
        65305978, 65305978, 57554544, 91703676, 91703676, 84211599, 49238328, 49238328,
        49238328, 24224830, 10045474, 10045474, 10045474, 40366667, 40366667, 40366667,
    ];

    // Fire King deck - extra deck (16 cards)
    let extra_deck = [
        94259633u32, 60303245, 87871125, 2772337, 2772337, 2772337, 48815792, 48815792,
        8264361, 29301450, 29301450, 93039339, 64182380, 64182380, 64182380, 0u32,
    ];

    println!("Populating main deck ({} cards)", main_deck.len());
    for (idx, card_id) in main_deck.iter().enumerate() {
        // LOCATION_DECK = 0x01, sequence is the position in deck
        duelNewCard(duel, 0, 0, *card_id, 0, 0x01, idx as u32, 0);
    }

    println!("Populating extra deck ({} cards)", extra_deck.len());
    for (idx, card_id) in extra_deck.iter().enumerate() {
        if *card_id == 0 {
            continue; // Skip empty slots
        }
        // LOCATION_EXTRA = 0x40, extra deck cards use sequence for ordering
        duelNewCard(duel, 0, 0, *card_id, 0, 0x40, idx as u32, 0);
    }

    // Start the duel - let the frontend loop take it from here
    println!("Starting duel");
    startDuel(duel);
    println!("Duel started");

    Ok(duel)
}

pub fn query_hand(duel: u32, team: u8) -> Vec<String> {
    let location = 2u32;  // Hand location (LOCATION_HAND == 0x02)
    
    // Get count of cards in hand
    let count = duelQueryCount(duel, team, location);
    println!("Hand count: {}", count);
    
    let mut cards = Vec::new();
    
    // Query all cards in location at once - gets full structure for each card
    if let Some(query_data) = duelQueryLocation(duel, 0xFFFFFFFF, team, location) {
        let data_vec = query_data.to_vec();
        println!("QueryLocation data length: {} bytes", data_vec.len());

        // Format: first 4 bytes = total size (u32, little-endian)
        // Then for each card: a sequence of fields. Each field is:
        //   u16 field_len (includes 4 bytes of flag + value size)
        //   u32 flag
        //   value (field_len - 4 bytes)
        // Fields repeat until a field with flag == QUERY_END (0x80000000) is encountered.
        // A null card is encoded as a single int16==0.

        let mut offset = 0usize;
        // skip leading total size u32 if present
        if data_vec.len() >= 4 {
            offset += 4;
        }

        for card_idx in 0..count as usize {
            if offset + 2 > data_vec.len() {
                println!("Not enough data for card {} header", card_idx);
                break;
            }
            // read int16 length for the first field (or 0 for null card)
            let field_len = i16::from_le_bytes([data_vec[offset], data_vec[offset + 1]]);
            offset += 2;
            if field_len == 0 {
                // null card
                println!("Card {}: null slot", card_idx);
                cards.push("0".to_string());
                continue;
            }

            // field_len is for the first field; we'll loop reading fields until QUERY_END
            // Note: each field has its own u16 length prefix
            let mut found_code: Option<u32> = None;
            // We already consumed the first field_len, so process fields in a loop
            let mut curr_field_len = field_len as usize;
            loop {
                if offset + 4 > data_vec.len() {
                    println!("Truncated field flag for card {}", card_idx);
                    break;
                }
                // read flag
                let flag = u32::from_le_bytes([
                    data_vec[offset],
                    data_vec[offset + 1],
                    data_vec[offset + 2],
                    data_vec[offset + 3],
                ]);
                offset += 4;

                let value_size = if curr_field_len >= 4 { curr_field_len - 4 } else { 0 };
                if offset + value_size > data_vec.len() {
                    println!("Truncated field value for card {}", card_idx);
                    break;
                }

                if flag == 0x80000000 {
                    // QUERY_END
                    println!("Card {}: end of fields", card_idx);
                    // no value bytes for QUERY_END
                } else if flag == 0x1 {
                    // QUERY_CODE: value is u32
                    if value_size >= 4 {
                        let code = u32::from_le_bytes([
                            data_vec[offset],
                            data_vec[offset + 1],
                            data_vec[offset + 2],
                            data_vec[offset + 3],
                        ]);
                        println!("Card {}: code={} (0x{:08x})", card_idx, code, code);
                        found_code = Some(code);
                    }
                } else {
                    // skip value_size bytes for other flags
                }

                // advance past the value bytes for this field
                offset += value_size;

                // if this field was QUERY_END, stop card loop, otherwise read next field_len
                if flag == 0x80000000 {
                    break;
                }

                // read next field_len for this card
                if offset + 2 > data_vec.len() {
                    println!("Missing next field length for card {}", card_idx);
                    break;
                }
                curr_field_len = u16::from_le_bytes([data_vec[offset], data_vec[offset + 1]]) as usize;
                offset += 2;
            }

            if let Some(code) = found_code {
                cards.push(code.to_string());
            } else {
                // if no code field found, push 0 or empty
                cards.push("0".to_string());
            }
        }
    }
    
    cards
}

pub fn query_location_codes(duel: u32, team: u8, location: u32) -> Vec<u32> {
    let mut codes: Vec<u32> = Vec::new();
    if let Some(query_data) = duelQueryLocation(duel, 0xFFFFFFFF, team, location) {
        let data_vec = query_data.to_vec();
        let mut offset = 0usize;
        if data_vec.len() >= 4 {
            offset += 4;
        }
        while offset + 2 <= data_vec.len() {
            let field_len = i16::from_le_bytes([data_vec[offset], data_vec[offset + 1]]);
            offset += 2;
            if field_len == 0 {
                codes.push(0);
                continue;
            }
            let mut curr_field_len = field_len as usize;
            let mut found_code: Option<u32> = None;
            loop {
                if offset + 4 > data_vec.len() {
                    break;
                }
                let flag = u32::from_le_bytes([
                    data_vec[offset],
                    data_vec[offset + 1],
                    data_vec[offset + 2],
                    data_vec[offset + 3],
                ]);
                offset += 4;
                let value_size = if curr_field_len >= 4 { curr_field_len - 4 } else { 0 };
                if offset + value_size > data_vec.len() {
                    break;
                }
                if flag == 0x1 && value_size >= 4 {
                    let code = u32::from_le_bytes([
                        data_vec[offset],
                        data_vec[offset + 1],
                        data_vec[offset + 2],
                        data_vec[offset + 3],
                    ]);
                    found_code = Some(code);
                }
                offset += value_size;
                if flag == 0x80000000 {
                    break;
                }
                if offset + 2 > data_vec.len() {
                    break;
                }
                curr_field_len = u16::from_le_bytes([data_vec[offset], data_vec[offset + 1]]) as usize;
                offset += 2;
            }
            codes.push(found_code.unwrap_or(0));
        }
    }
    codes
}

#[wasm_bindgen]
pub fn get_hand_count(duel: u32, team: u8) -> u32 {
    duelQueryCount(duel, team, 2u32)
}

pub fn get_deck_count(duel: u32, team: u8) -> u32 {
    duelQueryCount(duel, team, 0x01u32)
}

pub fn get_extra_deck_count(duel: u32, team: u8) -> u32 {
    duelQueryCount(duel, team, 0x40u32)
}

#[wasm_bindgen]
pub fn get_hand_codes(duel: u32, team: u8) -> js_sys::Uint32Array {
    let codes = query_location_codes(duel, team, 2u32);
    let out = js_sys::Uint32Array::new_with_length(codes.len() as u32);
    out.copy_from(&codes);
    out
}

#[wasm_bindgen]
pub fn send_response_u32(duel: u32, value: u32) {
    duelSetResponse(duel, &value.to_le_bytes());
}

#[wasm_bindgen]
pub fn process_duel_step(duel: u32) -> i32 {
    duelProcess(duel)
}

/// Polls `duelGetMessage` and splits the returned buffer into individual messages.
/// Each message is returned as a `Uint8Array` inside a JS `Array` (or `None` if no messages).
#[wasm_bindgen]
pub fn poll_messages(duel: u32) -> Option<js_sys::Array> {
    if let Some(buf) = duelGetMessage(duel) {
        let data_vec = buf.to_vec();
        let out = js_sys::Array::new();
        let mut offset = 0usize;
        while offset + 4 <= data_vec.len() {
            let size = u32::from_le_bytes([
                data_vec[offset],
                data_vec[offset + 1],
                data_vec[offset + 2],
                data_vec[offset + 3],
            ]) as usize;
            offset += 4;
            if offset + size > data_vec.len() {
                break;
            }
            // Inspect message id for phase/turn updates
            if size > 0 {
                let msg_id = data_vec[offset];
                let _ = msg_id;
            }
            let slice = &data_vec[offset..offset + size];
            let ua = js_sys::Uint8Array::new_with_length(size as u32);
            ua.copy_from(slice);
            out.push(&ua);
            offset += size;
        }
        Some(out)
    } else {
        None
    }
}

