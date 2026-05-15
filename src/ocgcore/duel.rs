use super::constants::*;
use crate::ocgcore::OCGCore;
use crate::ocgcore::memory::CorePointer;
use anyhow::anyhow;
use js_sys::ArrayBuffer;
use js_sys::DataView;
use js_sys::Int32Array;
use js_sys::Uint8Array;
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;

#[derive(Debug, Clone)]
pub struct Duel<'a> {
    handle: CorePointer,
    core: &'a OCGCore,
}

impl<'a> Duel<'a> {
    pub fn new(handle: CorePointer, core: &'a OCGCore) -> Self {
        Self { handle, core }
    }

    pub fn start(&self) -> anyhow::Result<()> {
        self.core.0.start_duel(self.handle.0);

        Ok(())
    }

    pub fn set_response(&self, buffer: &[u8]) -> anyhow::Result<()> {
        let buf_len = buffer.len() as u32;
        let buf_alloc = self.core.allocate_memory(buf_len)?;
        let buf_ptr = buf_alloc.get_pointer();

        let memory = self.core.get_wasm_memory()?;
        let dest_view = js_sys::Uint8Array::new_with_byte_offset_and_length(
            &memory.buffer(),
            buf_ptr.into(),
            buf_len,
        );

        dest_view.set(&js_sys::Uint8Array::from(buffer), 0);

        self.core.0.set_response(self.handle.0, buf_ptr.into());

        Ok(())
    }

    pub fn get_message(&self) -> anyhow::Result<Option<Uint8Array>> {
        let msg_ptr = self.core.0.get_message(self.handle.0);
        if msg_ptr == 0 {
            return Ok(None);
        }

        let memory = self.core.get_wasm_memory()?;
        let buffer: ArrayBuffer = memory.buffer().unchecked_into();

        let data_view = js_sys::DataView::new(&buffer, msg_ptr as usize, 2);
        let len = data_view.get_uint16(0) as u32;

        if len == 0 {
            return Ok(None);
        }

        let msg_view = Uint8Array::new_with_byte_offset_and_length(&buffer, msg_ptr + 2, len);

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

    pub fn process(&self) -> u32 {
        self.core.0.process(self.handle.0)
    }

    pub fn destroy(&self) -> anyhow::Result<()> {
        self.core.0.destroy_duel(self.handle.0);
        Ok(())
    }

    fn count_location(&self, team: u8, location: u32) -> anyhow::Result<u32> {
        let result = self.core.0.query_count(self.handle.0, team, location);

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
        let data_ptr = self
            .core
            .0
            .query_location(self.handle.0, flags, team, location)
            .map_err(|e| anyhow!("failed to query: {e:#?}"))?;

        if data_ptr == 0 {
            return Ok(None);
        }

        let memory = self.core.get_wasm_memory()?;
        let buffer: ArrayBuffer = memory.buffer().unchecked_into();

        if (data_ptr as usize + 4) > buffer.byte_length() as usize {
            return Err(anyhow::anyhow!(
                "OCGCore returned pointer {} out of WASM bounds {}",
                data_ptr,
                buffer.byte_length()
            ));
        }

        let header_view = DataView::new(&buffer, data_ptr as usize, 4);
        let len = header_view.get_uint32(0);

        if len == 0 {
            return Ok(None);
        }

        if (data_ptr as usize + 4 + len as usize) > buffer.byte_length() as usize {
            return Err(anyhow::anyhow!(
                "OCGCore message length {} exceeds WASM bounds",
                len
            ));
        }

        let data_view = Uint8Array::new_with_byte_offset_and_length(&buffer, data_ptr + 4, len);

        Ok(Some(data_view.slice(0, len)))
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

        self.core
            .0
            .add_card(self.handle.into(), info_ptr.get_pointer().into());

        Ok(())
    }

    pub fn load_script(&self, script: &str, name: &str) -> anyhow::Result<i32> {
        let script_bytes = script.as_bytes();
        let name_bytes = name.as_bytes();

        // allocate memory for script and name (name needs +1 for null terminator)
        let script_alloc = self.core.allocate_memory(script_bytes.len() as u32)?;
        let name_alloc = self.core.allocate_memory((name_bytes.len() + 1) as u32)?;

        let script_ptr = script_alloc.get_pointer();
        let name_ptr = name_alloc.get_pointer();

        let memory = self.core.get_wasm_memory()?;
        let buffer: ArrayBuffer = memory.buffer().unchecked_into();

        let script_dest = Uint8Array::new_with_byte_offset_and_length(
            &buffer,
            script_ptr.into(),
            script_bytes.len() as u32,
        );
        script_dest.set(&Uint8Array::from(script_bytes), 0);

        let name_dest = Uint8Array::new_with_byte_offset_and_length(
            &buffer,
            name_ptr.into(),
            (name_bytes.len() + 1) as u32,
        );
        name_dest.set(&Uint8Array::from(name_bytes), 0);
        name_dest.set_index(name_bytes.len() as u32, 0); // Null terminator

        // 5. Direct call to the typed method
        let result = self.core.0.load_script(
            self.handle.0,
            script_ptr.into(),
            script_bytes.len() as u32,
            name_ptr.into(),
        );

        Ok(result)
    }
}

impl Drop for Duel<'_> {
    fn drop(&mut self) {
        let _ = self.destroy();
    }
}
