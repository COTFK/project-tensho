use super::constants::*;
use super::data::OCGDuelOptions;
use crate::ocgcore::OCGCore;
use crate::ocgcore::memory::CoreMemoryAllocation;
use crate::ocgcore::memory::CorePointer;
use anyhow::anyhow;
use js_sys::Int32Array;
use js_sys::Uint8Array;
use js_sys::WebAssembly::Memory;
use std::cell::RefCell;
use std::mem::size_of;
use wasm_bindgen::JsValue;
use wasm_bindgen::closure::Closure;

#[derive(Debug, Clone)]
pub struct Duel<'a> {
    handle: CorePointer,
    core: &'a OCGCore,
}

impl Duel<'_> {
    pub fn new(core: &OCGCore) -> anyhow::Result<Duel> {
        let ocg_create_duel = core.get_function("_OCG_CreateDuel")?;
        let (card_reader, script_reader, log_handler, card_reader_done) =
            ensure_duel_callbacks(&core)?;

        let mut options = OCGDuelOptions::default();
        options.card_reader = card_reader;
        options.script_reader = script_reader;
        options.log_handler = log_handler;
        options.card_reader_done = card_reader_done;

        let options_alloc = core.allocate_memory(size_of::<OCGDuelOptions>() as u32)?;
        let options_ptr = options_alloc.get_pointer();
        let options_memory = core.get_wasm_memory()?;
        let options_buffer = options_memory.buffer();
        let options_bytes = unsafe {
            std::slice::from_raw_parts(
                (&options as *const OCGDuelOptions) as *const u8,
                size_of::<OCGDuelOptions>(),
            )
        };
        let options_view =
            Uint8Array::new_with_byte_offset(&options_buffer, usize::from(options_ptr) as u32);
        for (index, byte) in options_bytes.iter().enumerate() {
            options_view.set_index(index as u32, *byte);
        }

        let duel_alloc = core.allocate_memory(4)?;
        let duel_ptr = duel_alloc.get_pointer();

        let status = ocg_create_duel
            .call2(&JsValue::undefined(), &duel_ptr.into(), &options_ptr.into())
            .map_err(|e| anyhow!("Failed to create duel: {e:#?}"))?;

        let status = status
            .as_f64()
            .ok_or_else(|| anyhow!("Create duel returned a non-numeric status"))?
            as i32;

        if status != 0 {
            return Err(anyhow!("_OCG_CreateDuel failed with status {status}"));
        }

        Ok(Duel {
            handle: duel_ptr,
            core,
        })
    }

    pub fn start(&self) -> anyhow::Result<()> {
        let ocg_start_duel = self.core.get_function("_OCG_StartDuel")?;

        ocg_start_duel
            .call1(&JsValue::undefined(), &self.handle.into())
            .map_err(|e| anyhow!("_OCG_StartDuel call failed: {e:?}"))?;

        Ok(())
    }

    pub fn set_response(&self, buffer: &[u8]) -> anyhow::Result<()> {
        let ocg_set_response = self.core.get_function("_OCG_DuelSetResponse")?;

        let buf_ptr = self.core.allocate_memory(buffer.len() as u32)?;

        let memory = self.core.get_wasm_memory()?;
        let memory_buf = memory.buffer();
        let u8_view = Uint8Array::new_with_byte_offset(&memory_buf, buf_ptr.get_pointer().into());
        u8_view.copy_from(buffer);

        ocg_set_response
            .call2(
                &JsValue::undefined(),
                &self.handle.into(),
                &buf_ptr.get_pointer().into(),
            )
            .map_err(|e| anyhow!("_OCG_DuelSetResponse call failed: {e:?}"))?;

        Ok(())
    }

    pub fn get_message(&self) -> anyhow::Result<Option<Uint8Array>> {
        let ocg_get_message = self.core.get_function("_OCG_DuelGetMessage")?;

        let msg_ptr = ocg_get_message
            .call1(&JsValue::undefined(), &self.handle.into())
            .map_err(|e| anyhow!("_OCG_DuelGetMessage call failed: {e:?}"))?
            .as_f64()
            .ok_or(anyhow!("_OCG_DuelGetMessage did not return a number"))?
            as u32;

        if msg_ptr == 0 {
            return Ok(None);
        }

        let memory = self.core.get_wasm_memory()?;
        let memory_buf = memory.buffer();

        let msg_view = Uint8Array::new_with_byte_offset(&memory_buf, msg_ptr);
        Ok(Some(msg_view))
    }

    pub fn poll_messages(&self) -> Option<js_sys::Array> {
        match self.get_message().ok().flatten() {
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

    pub fn process(&self) -> anyhow::Result<i32> {
        let ocg_duel_process = self.core.get_function("_OCG_DuelProcess")?;

        let result = ocg_duel_process
            .call1(&JsValue::undefined(), &self.handle.into())
            .map_err(|e| anyhow!("_OCG_DuelProcess call failed: {e:?}"))?
            .as_f64()
            .ok_or(anyhow!("_OCG_DuelProcess did not return a number"))?
            as i32;

        Ok(result)
    }

    pub fn destroy(&self) -> anyhow::Result<()> {
        let ocg_destroy_duel = self.core.get_function("_OCG_DestroyDuel")?;

        ocg_destroy_duel
            .call1(&JsValue::undefined(), &self.handle.into())
            .map_err(|e| anyhow!("_OCG_DestroyDuel call failed: {e:?}"))?;

        Ok(())
    }

    fn count_location(&self, team: u8, location: u32) -> anyhow::Result<u32> {
        let ocg_query_count = self.core.get_function("_OCG_DuelQueryCount")?;

        let result = ocg_query_count
            .call3(
                &JsValue::undefined(),
                &self.handle.into(),
                &(team as f64).into(),
                &(location as f64).into(),
            )
            .map_err(|e| anyhow!("_OCG_DuelQueryCount call failed: {e:?}"))?
            .as_f64()
            .ok_or(anyhow!("_OCG_DuelQueryCount did not return a number"))?
            as u32;

        Ok(result)
    }

    pub fn count_deck(&self, team: u8) -> u32 {
        self.count_location(team, LOCATION_DECK).unwrap_or(0)
    }

    pub fn count_extra_deck(&self, team: u8) -> u32 {
        self.count_location(team, LOCATION_EXTRA).unwrap_or(0)
    }

    fn query_location(
        &self,
        flags: u32,
        team: u8,
        location: u32,
    ) -> anyhow::Result<Option<Uint8Array>> {
        let ocg_query_location = self.core.get_function("_OCG_DuelQueryLocation")?;

        let data_ptr = ocg_query_location
            .call4(
                &JsValue::undefined(),
                &self.handle.into(),
                &(flags as f64).into(),
                &(team as f64).into(),
                &(location as f64).into(),
            )
            .map_err(|e| anyhow!("_OCG_DuelQueryLocation call failed: {e:?}"))?
            .as_f64()
            .ok_or(anyhow!("_OCG_DuelQueryLocation did not return a number"))?
            as u32;

        if data_ptr == 0 {
            return Ok(None);
        }

        let memory = self.core.get_wasm_memory()?;
        let memory_buf = memory.buffer();

        let data_view = Uint8Array::new_with_byte_offset(&memory_buf, data_ptr);
        Ok(Some(data_view))
    }

    pub fn query_location_codes(&self, team: u8, location: u32) -> Vec<u32> {
        match self
            .query_location(0xFFFFFFFF, team, location)
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

    pub fn query_hand(&self, team: u8) -> Vec<String> {
        self.query_location_codes(team, 0x02u32)
            .into_iter()
            .map(|code| code.to_string())
            .collect()
    }

    pub fn add_card(
        &self,
        team: u8,
        _duelist: u8,
        code: u32,
        controller: u8,
        location: u32,
        sequence: u32,
        position: u32,
    ) -> anyhow::Result<()> {
        let ocg_new_card = self.core.get_function("_OCG_DuelNewCard")?;

        let info_ptr = self.core.allocate_memory(32)?;

        let memory = self.core.get_wasm_memory()?;
        let memory_buf = memory.buffer();

        // Set card info fields
        let i32_view = Int32Array::new_with_byte_offset(&memory_buf, info_ptr.get_pointer().into());
        i32_view.set_index(0, team as i32);
        i32_view.set_index(1, code as i32);
        i32_view.set_index(2, controller as i32);
        i32_view.set_index(3, location as i32);
        i32_view.set_index(4, sequence as i32);
        i32_view.set_index(5, (position | 1) as i32);

        ocg_new_card
            .call2(
                &JsValue::undefined(),
                &self.handle.into(),
                &info_ptr.get_pointer().into(),
            )
            .map_err(|e| anyhow!("_OCG_DuelNewCard call failed: {e:?}"))?;

        Ok(())
    }

    pub fn load_script(&self, script: &str, name: &str) -> anyhow::Result<i32> {
        let ocg_load_script = self.core.get_function("_OCG_LoadScript")?;

        let script_bytes = script.as_bytes();
        let name_bytes = name.as_bytes();

        let script_alloc = self.core.allocate_memory(script_bytes.len() as u32)?;
        let script_ptr = script_alloc.get_pointer();

        let name_alloc = self.core.allocate_memory((name_bytes.len() + 1) as u32)?;
        let name_ptr = name_alloc.get_pointer();

        let memory = self.core.get_wasm_memory()?;
        let memory_buf = memory.buffer();

        // Copy script bytes
        let script_view =
            Uint8Array::new_with_byte_offset(&memory_buf, usize::from(script_ptr) as u32);
        script_view.copy_from(script_bytes);

        // Copy name bytes and null-terminate
        let name_view = Uint8Array::new_with_byte_offset(&memory_buf, usize::from(name_ptr) as u32);
        name_view.copy_from(name_bytes);
        name_view.set_index(name_bytes.len() as u32, 0);

        // Call the OCG function
        let result = ocg_load_script
            .call4(
                &JsValue::undefined(),
                &self.handle.into(),
                &script_ptr.into(),
                &(script_bytes.len() as f64).into(),
                &name_ptr.into(),
            )
            .map_err(|e| anyhow!("_OCG_LoadScript call failed: {e:?}"))?
            .as_f64()
            .ok_or(anyhow!("_OCG_LoadScript did not return a number"))? as i32;

        Ok(result)
    }
}

thread_local! {
    // Hold JsValue representations of closures so they are not dropped and stay registered with the wasm runtime.
    static CALLBACK_KEEP_ALIVE: RefCell<Vec<JsValue>> = RefCell::new(Vec::new());
    // Store only numeric callback indices returned by `addFunction`.
    static DUEL_CALLBACKS: RefCell<Option<(u32, u32, u32, u32)>> = RefCell::new(None);
}

fn ensure_duel_callbacks(core: &OCGCore) -> anyhow::Result<(u32, u32, u32, u32)> {
    DUEL_CALLBACKS.with(|slot| -> anyhow::Result<(u32, u32, u32, u32)> {
        let mut slot = slot.borrow_mut();

        if let Some(indices) = *slot {
            return Ok(indices);
        }

        // Build closures
        let card_reader = Closure::wrap(Box::new(move |payload: f64, code: f64, data_ptr: f64| {
            web_sys::console::log_1(
                &format!(
                    "card_reader called: payload={}, code={}, data_ptr={}",
                    payload, code, data_ptr
                )
                .into(),
            );
        }) as Box<dyn FnMut(f64, f64, f64)>);


        let script_reader =
            Closure::wrap(Box::new(move |payload: f64, duel: f64, name: f64| -> i32 {
                let memory = core.get_wasm_memory().unwrap();
                let script_name = read_c_string(memory, name).unwrap_or_else(|_| format!("<ptr:{name}>") );
                web_sys::console::warn_1(
                    &format!(
                        "script_reader asked for '{}' (payload={}, duel={}) but dynamic loading is not wired; returning failure",
                        script_name, payload, duel
                    )
                    .into(),
                );
                0
            }) as Box<dyn FnMut(f64, f64, f64) -> i32>);

        let log_handler =
            Closure::wrap(Box::new(move |_payload: f64, _string: f64, _type: f64| {
                web_sys::console::log_1(
                    &format!(
                        "log_handler called: payload={}, _string={}, _type={}",
                        _payload, _string, _type
                    )
                    .into(),
                );
            })
                as Box<dyn FnMut(f64, f64, f64)>);

        let card_reader_done = Closure::wrap(
            Box::new(move |_payload: f64, _data_ptr: f64| {
                web_sys::console::log_1(
                    &format!(
                        "card_reader_done called: payload={}, _data_ptr={}",
                        _payload, _data_ptr
                    )
                    .into(),
                );
            }) as Box<dyn FnMut(f64, f64)>
        );

        let add_function = core.get_function("addFunction")?;

        let card_reader_index = add_function
            .call2(&JsValue::undefined(), card_reader.as_ref(), &"viii".into())
            .map_err(|e| anyhow!("Failed to register card reader callback: {e:#?}"))?
            .as_f64()
            .ok_or_else(|| anyhow!("Card reader callback index was not numeric"))?
            as u32;
        let script_reader_index = add_function
            .call2(
                &JsValue::undefined(),
                script_reader.as_ref(),
                &"iiii".into(),
            )
            .map_err(|e| anyhow!("Failed to register script reader callback: {e:#?}"))?
            .as_f64()
            .ok_or_else(|| anyhow!("Script reader callback index was not numeric"))?
            as u32;
        let log_handler_index = add_function
            .call2(&JsValue::undefined(), log_handler.as_ref(), &"viii".into())
            .map_err(|e| anyhow!("Failed to register log handler callback: {e:#?}"))?
            .as_f64()
            .ok_or_else(|| anyhow!("Log handler callback index was not numeric"))?
            as u32;
        let card_reader_done_index = add_function
            .call2(
                &JsValue::undefined(),
                card_reader_done.as_ref(),
                &"vii".into(),
            )
            .map_err(|e| anyhow!("Failed to register card reader done callback: {e:#?}"))?
            .as_f64()
            .ok_or_else(|| anyhow!("Card reader done callback index was not numeric"))?
            as u32;

        // Keep closures alive by storing their JsValue representations in CALLBACK_KEEP_ALIVE
        CALLBACK_KEEP_ALIVE.with(|vec| {
            let mut v = vec.borrow_mut();
            v.push(card_reader.into_js_value());
            v.push(script_reader.into_js_value());
            v.push(log_handler.into_js_value());
            v.push(card_reader_done.into_js_value());
        });

        let indices = (
            card_reader_index,
            script_reader_index,
            log_handler_index,
            card_reader_done_index,
        );
        *slot = Some(indices);
        Ok(indices)
    })
}

fn read_c_string(memory: Memory, ptr: f64) -> anyhow::Result<String> {
    let buffer = memory.buffer();
    let bytes = Uint8Array::new(&buffer);
    let mut offset = ptr as u32;
    let mut out = Vec::new();

    while offset < bytes.length() {
        let byte = bytes.get_index(offset);
        if byte == 0 {
            break;
        }

        out.push(byte);
        offset += 1;
    }

    String::from_utf8(out).map_err(|e| anyhow!("Invalid UTF-8 in script name: {e}"))
}
