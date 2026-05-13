use crate::wasm::WASMMemoryAllocation;
use crate::wasm::get_memory_view;
use crate::wasm::get_module_function;
use crate::wasm::get_wasm_memory;
use js_sys::Int32Array;
use js_sys::Uint8Array;
use wasm_bindgen::JsValue;

pub fn get_version() -> anyhow::Result<(i32, i32)> {
    let ocg_get_version = get_module_function("_OCG_GetVersion")?;

    // Allocate 8 bytes (two ints) once instead of 4 bytes (one int) twice
    let version_alloc = WASMMemoryAllocation::new(8)?;
    let major_version_offset = version_alloc.get_pointer();
    let minor_version_offset = major_version_offset + 4.0;

    ocg_get_version.call2(
        &JsValue::undefined(),
        &JsValue::from_f64(major_version_offset),
        &JsValue::from_f64(minor_version_offset),
    ).map_err(|e| anyhow::anyhow!("Failed to get version: {e:#?}"))?;

    let view = get_memory_view()?;

    // Read sequentially from the single allocation
    let major = view.get_int32_endian(major_version_offset as usize, true);
    let minor = view.get_int32_endian(minor_version_offset as usize, true);

    Ok((major, minor))
}

pub fn create_duel() -> anyhow::Result<u32> {
    let ocg_create_duel = get_module_function("_OCG_CreateDuel")?;

    let result = ocg_create_duel
        .call0(&JsValue::undefined())
        .map_err(|e| anyhow::anyhow!("Failed to create duel: {e:#?}"))?;

    Ok(result.as_f64().unwrap() as u32)
}

pub fn destroy_duel(duel: u32) -> anyhow::Result<()> {
    let ocg_destroy_duel = get_module_function("_OCG_DestroyDuel")?;

    ocg_destroy_duel
        .call1(&JsValue::undefined(), &(duel as f64).into())
        .map_err(|e| anyhow::anyhow!("_OCG_DestroyDuel call failed: {e:?}"))?;

    Ok(())
}

pub fn duel_new_card(
    duel: u32,
    team: u8,
    _duelist: u8,
    code: u32,
    controller: u8,
    location: u32,
    sequence: u32,
    position: u32,
) -> anyhow::Result<()> {
    let ocg_new_card = get_module_function("_OCG_DuelNewCard")?;
    let malloc = get_module_function("_malloc")?;
    let free = get_module_function("_free")?;

    let info_ptr = malloc
        .call1(&JsValue::undefined(), &32.0.into())
        .map_err(|e| format!("malloc failed: {e:?}"))?
        .as_f64()
        .ok_or("malloc did not return a number")? as u32;

    let memory = get_wasm_memory()?;
    let memory_buf = memory.buffer();

    // Zero-fill the card info structure
    let u8_view = Uint8Array::new_with_byte_offset(&memory_buf, info_ptr);
    for i in 0..32 {
        u8_view.set_index(i, 0);
    }

    // Set card info fields
    let i32_view = Int32Array::new_with_byte_offset(&memory_buf, info_ptr);
    i32_view.set_index(0, team as i32);
    i32_view.set_index(1, code as i32);
    i32_view.set_index(2, controller as i32);
    i32_view.set_index(3, location as i32);
    i32_view.set_index(4, sequence as i32);
    i32_view.set_index(5, (position | 1) as i32);

    ocg_new_card
        .call2(
            &JsValue::undefined(),
            &(duel as f64).into(),
            &(info_ptr as f64).into(),
        )
        .map_err(|e| format!("_OCG_DuelNewCard call failed: {e:?}"))?;

    let _ = free.call1(&JsValue::undefined(), &(info_ptr as f64).into());

    Ok(())
}

pub fn start_duel(duel: u32) -> Result<(), String> {
    let ocg_start_duel = get_module_function("_OCG_StartDuel")?;

    ocg_start_duel
        .call1(&JsValue::undefined(), &(duel as f64).into())
        .map_err(|e| format!("_OCG_StartDuel call failed: {e:?}"))?;

    Ok(())
}

pub fn duel_process(duel: u32) -> Result<i32, String> {
    let ocg_duel_process = get_module_function("_OCG_DuelProcess")?;

    let result = ocg_duel_process
        .call1(&JsValue::undefined(), &(duel as f64).into())
        .map_err(|e| format!("_OCG_DuelProcess call failed: {e:?}"))?
        .as_f64()
        .ok_or("_OCG_DuelProcess did not return a number")? as i32;

    Ok(result)
}

pub fn duel_get_message(duel: u32) -> Result<Option<Uint8Array>, String> {
    let ocg_get_message = get_module_function("_OCG_DuelGetMessage")?;

    let msg_ptr = ocg_get_message
        .call1(&JsValue::undefined(), &(duel as f64).into())
        .map_err(|e| format!("_OCG_DuelGetMessage call failed: {e:?}"))?
        .as_f64()
        .ok_or("_OCG_DuelGetMessage did not return a number")? as u32;

    if msg_ptr == 0 {
        return Ok(None);
    }

    let memory = get_wasm_memory()?;
    let memory_buf = memory.buffer();

    let msg_view = Uint8Array::new_with_byte_offset(&memory_buf, msg_ptr);
    Ok(Some(msg_view))
}

pub fn duel_set_response(duel: u32, buffer: &[u8]) -> Result<(), String> {
    let ocg_set_response = get_module_function("_OCG_DuelSetResponse")?;
    let malloc = get_module_function("_malloc")?;
    let free = get_module_function("_free")?;

    let buf_ptr = malloc
        .call1(&JsValue::undefined(), &(buffer.len() as f64).into())
        .map_err(|e| format!("malloc failed: {e:?}"))?
        .as_f64()
        .ok_or("malloc did not return a number")? as u32;

    let memory = get_wasm_memory()?;
    let memory_buf = memory.buffer();
    let u8_view = Uint8Array::new_with_byte_offset(&memory_buf, buf_ptr);
    u8_view.copy_from(buffer);

    ocg_set_response
        .call2(
            &JsValue::undefined(),
            &(duel as f64).into(),
            &(buf_ptr as f64).into(),
        )
        .map_err(|e| format!("_OCG_DuelSetResponse call failed: {e:?}"))?;

    let _ = free.call1(&JsValue::undefined(), &(buf_ptr as f64).into());

    Ok(())
}

pub fn load_script(duel: u32, script: &str, name: &str) -> Result<i32, String> {
    let ocg_load_script = get_module_function("_OCG_LoadScript")?;
    let malloc = get_module_function("_malloc")?;
    let free = get_module_function("_free")?;

    let script_bytes = script.as_bytes();
    let name_bytes = name.as_bytes();

    let script_ptr = malloc
        .call1(&JsValue::undefined(), &(script_bytes.len() as f64).into())
        .map_err(|e| format!("malloc for script failed: {e:?}"))?
        .as_f64()
        .ok_or("malloc did not return a number")? as u32;

    let name_ptr = malloc
        .call1(
            &JsValue::undefined(),
            &((name_bytes.len() + 1) as f64).into(),
        )
        .map_err(|e| format!("malloc for name failed: {e:?}"))?
        .as_f64()
        .ok_or("malloc did not return a number")? as u32;

    let memory = get_wasm_memory()?;
    let memory_buf = memory.buffer();

    let script_view = Uint8Array::new_with_byte_offset(&memory_buf, script_ptr);
    script_view.copy_from(script_bytes);

    let name_view = Uint8Array::new_with_byte_offset(&memory_buf, name_ptr);
    name_view.copy_from(name_bytes);
    name_view.set_index(name_bytes.len() as u32, 0);

    let result = ocg_load_script
        .call4(
            &JsValue::undefined(),
            &(duel as f64).into(),
            &(script_ptr as f64).into(),
            &(script_bytes.len() as f64).into(),
            &(name_ptr as f64).into(),
        )
        .map_err(|e| format!("_OCG_LoadScript call failed: {e:?}"))?
        .as_f64()
        .ok_or("_OCG_LoadScript did not return a number")? as i32;

    let _ = free.call1(&JsValue::undefined(), &(script_ptr as f64).into());
    let _ = free.call1(&JsValue::undefined(), &(name_ptr as f64).into());

    Ok(result)
}

fn duel_query_count(duel: u32, team: u8, location: u32) -> Result<u32, String> {
    let ocg_query_count = get_module_function("_OCG_DuelQueryCount")?;

    let result = ocg_query_count
        .call3(
            &JsValue::undefined(),
            &(duel as f64).into(),
            &(team as f64).into(),
            &(location as f64).into(),
        )
        .map_err(|e| format!("_OCG_DuelQueryCount call failed: {e:?}"))?
        .as_f64()
        .ok_or("_OCG_DuelQueryCount did not return a number")? as u32;

    Ok(result)
}

fn duel_query_location(
    duel: u32,
    flags: u32,
    team: u8,
    location: u32,
) -> Result<Option<Uint8Array>, String> {
    let ocg_query_location = get_module_function("_OCG_DuelQueryLocation")?;

    let data_ptr = ocg_query_location
        .call4(
            &JsValue::undefined(),
            &(duel as f64).into(),
            &(flags as f64).into(),
            &(team as f64).into(),
            &(location as f64).into(),
        )
        .map_err(|e| format!("_OCG_DuelQueryLocation call failed: {e:?}"))?
        .as_f64()
        .ok_or("_OCG_DuelQueryLocation did not return a number")? as u32;

    if data_ptr == 0 {
        return Ok(None);
    }

    let memory = get_wasm_memory()?;
    let memory_buf = memory.buffer();

    let data_view = Uint8Array::new_with_byte_offset(&memory_buf, data_ptr);
    Ok(Some(data_view))
}

// ============================================================================
// Higher-level Query Functions
// ============================================================================

pub fn query_hand(duel: u32, team: u8) -> Vec<String> {
    query_location_codes(duel, team, 0x02u32)
        .into_iter()
        .map(|code| code.to_string())
        .collect()
}

pub fn query_location_codes(duel: u32, team: u8, location: u32) -> Vec<u32> {
    match duel_query_location(duel, 0xFFFFFFFF, team, location)
        .ok()
        .flatten()
    {
        Some(buf) => {
            let data_vec = buf.to_vec();
            let mut codes = Vec::new();
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
                let mut found_code = None;

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

                    let value_size = if curr_field_len >= 4 {
                        curr_field_len - 4
                    } else {
                        0
                    };

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

                    curr_field_len =
                        u16::from_le_bytes([data_vec[offset], data_vec[offset + 1]]) as usize;
                    offset += 2;
                }

                codes.push(found_code.unwrap_or(0));
            }

            codes
        }
        None => Vec::new(),
    }
}

pub fn get_deck_count(duel: u32, team: u8) -> u32 {
    duel_query_count(duel, team, 0x01u32).unwrap_or(0)
}

pub fn get_extra_deck_count(duel: u32, team: u8) -> u32 {
    duel_query_count(duel, team, 0x40u32).unwrap_or(0)
}

// ============================================================================
// Main.rs-style Wrappers
// ============================================================================

pub fn send_response_u32(duel: u32, value: u32) {
    duel_set_response(duel, &value.to_le_bytes());
}

pub fn poll_messages(duel: u32) -> Option<js_sys::Array> {
    match duel_get_message(duel).ok().flatten() {
        Some(buf) => {
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

                let slice = &data_vec[offset..offset + size];
                let ua = Uint8Array::new_with_length(size as u32);
                ua.copy_from(slice);
                out.push(&ua);
                offset += size;
            }

            Some(out)
        }
        None => None,
    }
}
