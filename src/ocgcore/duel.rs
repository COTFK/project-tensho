use super::constants::*;
use crate::ocgcore::OCGCore;
use crate::ocgcore::memory::CorePointer;
use anyhow::anyhow;
use js_sys::ArrayBuffer;
use js_sys::Uint8Array;
use js_sys::Uint32Array;
use wasm_bindgen::JsCast;

#[derive(Debug, Clone)]
pub struct Duel<'a> {
    handle: CorePointer,
    core: &'a OCGCore,
}

/// Helper to write little-endian bytes to a Uint8Array view
fn write_le_bytes(view: &Uint8Array, offset: u32, bytes: &[u8]) {
    for (i, byte) in bytes.iter().enumerate() {
        view.set_index(offset + i as u32, *byte);
    }
}

/// Helper to read a u32 from a byte slice as little-endian
fn read_u32_le(bytes: &[u8]) -> u32 {
    u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
}

/// Parse card code from OCG query data field
fn extract_card_code(data: &[u8], offset: &mut usize, field_len: usize) -> u32 {
    let mut remaining = field_len;
    let mut code = 0;

    while remaining > 0 && *offset + 4 <= data.len() {
        let flag = read_u32_le(&data[*offset..*offset + 4]);
        *offset += 4;
        remaining = remaining.saturating_sub(4);

        let value_size = remaining;
        if *offset + value_size > data.len() {
            break;
        }

        // Flag 0x1 = card code
        if flag == 0x1 && value_size >= 4 {
            code = read_u32_le(&data[*offset..*offset + 4]);
        }

        *offset += value_size;

        // Flag 0x80000000 = end of card
        if flag == 0x80000000 {
            break;
        }

        // Read next field length
        if *offset + 2 <= data.len() {
            remaining = u16::from_le_bytes([data[*offset], data[*offset + 1]]) as usize;
            *offset += 2;
        } else {
            break;
        }
    }

    code
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
        let length_alloc = self.core.allocate_memory(4)?;
        let length_ptr = length_alloc.get_pointer();

        let msg_ptr = self.core.0.get_message(self.handle.0, length_ptr.into());
        if msg_ptr == 0 {
            return Ok(None);
        }

        let memory = self.core.get_wasm_memory()?;
        let buffer: ArrayBuffer = memory.buffer().unchecked_into();

        let len = Uint32Array::new_with_byte_offset_and_length(&buffer, length_ptr.into(), 1)
            .get_index(0);

        if len == 0 {
            return Ok(None);
        }

        Ok(Some(Uint8Array::new_with_byte_offset_and_length(
            &buffer, msg_ptr, len,
        )))
    }

    pub fn poll_messages(&self) -> Option<js_sys::Array> {
        let buf = self.get_message().ok().flatten()?;
        let data_vec = buf.to_vec();
        let out = js_sys::Array::new();
        let mut offset = 0usize;

        while offset + 4 <= data_vec.len() {
            let size = read_u32_le(&data_vec[offset..offset + 4]) as usize;
            offset += 4;

            if offset + size > data_vec.len() {
                break;
            }

            let ua = Uint8Array::new_with_length(size as u32);
            ua.copy_from(&data_vec[offset..offset + size]);
            out.push(&ua);
            offset += size;
        }

        Some(out)
    }

    pub fn process(&self) -> u32 {
        self.core.0.process(self.handle.0)
    }

    pub fn destroy(&self) -> anyhow::Result<()> {
        self.core.0.destroy_duel(self.handle.0);
        Ok(())
    }

    pub fn count_location(&self, team: u8, location: u32) -> u32 {
        self.core.0.query_count(self.handle.0, team, location)
    }

    fn query_location(
        &self,
        flags: u32,
        team: u8,
        location: u32,
    ) -> anyhow::Result<Option<Uint8Array>> {
        let memory = self.core.get_wasm_memory()?;
        let buffer: ArrayBuffer = memory.buffer().unchecked_into();

        // Allocate OCG_QueryInfo struct (20 bytes)
        let info_alloc = self.core.allocate_memory(20)?;
        let length_alloc = self.core.allocate_memory(4)?;

        let info_ptr = info_alloc.get_pointer();
        let length_ptr = length_alloc.get_pointer();

        // Build OCG_QueryInfo struct: flags (u32) + team (u8) + pad (3) + location (u32) + seq (u32) + overlay_seq (u32)
        let info_view = Uint8Array::new_with_byte_offset_and_length(&buffer, info_ptr.into(), 20);
        write_le_bytes(&info_view, 0, &flags.to_le_bytes());
        info_view.set_index(4, team);
        write_le_bytes(&info_view, 8, &location.to_le_bytes());

        let data_ptr = self
            .core
            .0
            .query_location(self.handle.0, length_ptr.into(), info_ptr.into())
            .map_err(|e| anyhow!("query_location failed: {e:?}"))?;

        // Read returned length
        let length_view =
            Uint8Array::new_with_byte_offset_and_length(&buffer, length_ptr.into(), 4);
        let query_length = read_u32_le(&length_view.to_vec());

        // Empty result
        if query_length <= 4 || data_ptr == 0 {
            return Ok(None);
        }

        // Bounds check
        if (data_ptr as usize + query_length as usize) > buffer.byte_length() as usize {
            return Err(anyhow!(
                "OCGCore returned pointer {} with length {} exceeds WASM bounds {}",
                data_ptr,
                query_length,
                buffer.byte_length()
            ));
        }

        // Skip 4-byte header, read actual data
        let actual_data_len = query_length - 4;
        let data_view =
            Uint8Array::new_with_byte_offset_and_length(&buffer, data_ptr + 4, actual_data_len);

        Ok(Some(data_view.slice(0, actual_data_len)))
    }

    pub fn query_location_codes(&self, team: u8, location: u32) -> Vec<u32> {
        let buf = self
            .query_location(0xFFFFFFFF, team, location)
            .ok()
            .flatten()
            .map(|b| b.to_vec())
            .unwrap_or_default();

        let mut codes = Vec::new();
        let mut offset = 0usize;

        while offset + 2 <= buf.len() {
            let field_len = i16::from_le_bytes([buf[offset], buf[offset + 1]]) as usize;
            offset += 2;

            codes.push(if field_len == 0 {
                0
            } else {
                extract_card_code(&buf, &mut offset, field_len)
            });
        }

        codes
    }

    pub fn query_hand(&self, team: u8) -> Vec<String> {
        self.query_location_codes(team, LOCATION_HAND)
            .into_iter()
            .map(|code| code.to_string())
            .collect()
    }

    pub fn query_deck(&self, team: u8) -> Vec<String> {
        self.query_location_codes(team, 0x01u32)
            .into_iter()
            .map(|code| code.to_string())
            .collect()
    }

    pub fn add_card(
        &self,
        team: u8,
        duelist: u8,
        code: u32,
        controller: u8,
        location: u32,
        sequence: u32,
        position: u32,
    ) -> anyhow::Result<()> {
        let info_ptr = self.core.allocate_memory(24)?;

        let memory = self.core.get_wasm_memory()?;
        let memory_buf = memory.buffer();
        let info_offset = info_ptr.get_pointer().into();
        let info_view = Uint8Array::new_with_byte_offset_and_length(&memory_buf, info_offset, 24);

        // OCG_NewCardInfo struct layout:
        // uint8_t team;       // offset 0
        // uint8_t duelist;    // offset 1
        // [2 bytes padding]   // offset 2-3
        // uint32_t code;      // offset 4-7
        // uint8_t con;        // offset 8
        // [3 bytes padding]   // offset 9-11
        // uint32_t loc;       // offset 12-15
        // uint32_t seq;       // offset 16-19
        // uint32_t pos;       // offset 20-23

        info_view.set_index(0, team);
        info_view.set_index(1, duelist);
        // Padding at 2-3 is left as-is (zeros from allocation)

        write_le_bytes(&info_view, 4, &code.to_le_bytes());

        info_view.set_index(8, controller);
        // Padding at 9-11 is left as-is

        write_le_bytes(&info_view, 12, &location.to_le_bytes());
        write_le_bytes(&info_view, 16, &sequence.to_le_bytes());
        write_le_bytes(&info_view, 20, &position.to_le_bytes());

        self.core.0.add_card(self.handle.into(), info_offset as u32);

        Ok(())
    }

    pub fn load_script(&self, script: Vec<u8>, name: &str) -> anyhow::Result<i32> {
        let name_bytes = name.as_bytes();

        // allocate memory for script and name (name needs +1 for null terminator)
        let script_alloc = self.core.allocate_memory(script.len() as u32)?;
        let name_alloc = self.core.allocate_memory((name_bytes.len() + 1) as u32)?;

        let script_ptr = script_alloc.get_pointer();
        let name_ptr = name_alloc.get_pointer();

        let memory = self.core.get_wasm_memory()?;
        let buffer: ArrayBuffer = memory.buffer().unchecked_into();

        let script_dest = Uint8Array::new_with_byte_offset_and_length(
            &buffer,
            script_ptr.into(),
            script.len() as u32,
        );
        script_dest.set(&Uint8Array::from(script.as_slice()), 0);

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
            script.len() as u32,
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
