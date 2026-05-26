use js_sys::ArrayBuffer;
use js_sys::Uint8Array;
use js_sys::Uint32Array;
use wasm_bindgen::JsCast;

use super::OCGCore;
use super::constants::CardLocation;
use super::duel_status::DuelStatus;
use super::memory::CorePointer;
use crate::ocgcore::ActiveCard;
use crate::ocgcore::CoreMessage;
use crate::ocgcore::UserResponse;
use crate::ocgcore::constants::BattlePosition;
use crate::ocgcore::constants::{CardController, CardOwner};

#[derive(Debug, Clone, PartialEq)]
pub struct Duel {
    handle: CorePointer,
    core: OCGCore,
}

/// Helper to write little-endian bytes to a `Uint8Array` view
fn write_le_bytes(view: &Uint8Array, offset: u32, bytes: &[u8]) {
    for (i, byte) in bytes.iter().enumerate() {
        view.set_index(offset + i as u32, *byte);
    }
}

/// Helper to read a u32 from a byte slice as little-endian
fn read_u32_le(bytes: &[u8]) -> u32 {
    u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
}

impl Duel {
    pub fn new(handle: CorePointer, core: &OCGCore) -> Self {
        Self {
            handle,
            core: core.clone(),
        }
    }

    pub fn start(&self) {
        self.core.instance.start_duel(self.handle.0);
    }

    pub fn set_response(&self, response: UserResponse) {
        let bytes = response.get_response_bytes();
        let buffer = bytes.as_slice();
        let buf_len = buffer.len() as u32;
        let buf_alloc = self.core.allocate_memory(buf_len);
        let buf_ptr = buf_alloc.get_pointer();

        let memory = self.core.get_wasm_memory();
        let dest_view = js_sys::Uint8Array::new_with_byte_offset_and_length(
            &memory.buffer(),
            buf_ptr.into(),
            buf_len,
        );

        dest_view.set(&js_sys::Uint8Array::from(buffer), 0);

        self.core
            .instance
            .set_response(self.handle.0, buf_ptr.into(), buf_len);
    }

    pub fn get_messages(&self) -> Vec<u8> {
        let length_alloc = self.core.allocate_memory(4);
        let length_ptr = length_alloc.get_pointer();

        let msg_ptr = self
            .core
            .instance
            .get_message(self.handle.0, length_ptr.into());
        let memory = self.core.get_wasm_memory();
        let buffer: ArrayBuffer = memory.buffer().unchecked_into();

        let len = Uint32Array::new_with_byte_offset_and_length(&buffer, length_ptr.into(), 1)
            .get_index(0);

        Uint8Array::new_with_byte_offset_and_length(&buffer, msg_ptr, len).to_vec()
    }

    pub fn parse_messages(&self) -> CoreMessage {
        CoreMessage::try_from(self.get_messages()).unwrap()
    }

    pub fn process(&self) -> DuelStatus {
        DuelStatus::try_from(self.core.instance.process(self.handle.0)).unwrap()
    }

    pub fn destroy(&self) {
        self.core.instance.destroy_duel(self.handle.0);
    }

    pub fn count_location(&self, team: CardOwner, location: CardLocation) -> u32 {
        self.core
            .instance
            .query_count(self.handle.0, team as u8, location as u32)
    }

    fn query_location(
        &self,
        flags: u32,
        team: CardOwner,
        location: CardLocation,
    ) -> Option<Uint8Array> {
        let memory = self.core.get_wasm_memory();
        let buffer: ArrayBuffer = memory.buffer().unchecked_into();

        // Allocate OCG_QueryInfo struct (20 bytes)
        let info_alloc = self.core.allocate_memory(20);
        let length_alloc = self.core.allocate_memory(4);

        let info_ptr = info_alloc.get_pointer();
        let length_ptr = length_alloc.get_pointer();

        // Build OCG_QueryInfo struct: flags (u32) + team (u8) + pad (3) + location (u32) + seq (u32) + overlay_seq (u32)
        let info_view = Uint8Array::new_with_byte_offset_and_length(&buffer, info_ptr.into(), 20);
        write_le_bytes(&info_view, 0, &flags.to_le_bytes());
        info_view.set_index(4, team as u8);
        write_le_bytes(&info_view, 8, &(location as isize).to_le_bytes());

        let data_ptr =
            self.core
                .instance
                .query_location(self.handle.0, length_ptr.into(), info_ptr.into());

        // Read returned length
        let length_view =
            Uint8Array::new_with_byte_offset_and_length(&buffer, length_ptr.into(), 4);
        let query_length = read_u32_le(&length_view.to_vec());

        // Skip 4-byte header, read actual data
        let actual_data_len = query_length - 4;
        let data_view =
            Uint8Array::new_with_byte_offset_and_length(&buffer, data_ptr + 4, actual_data_len);

        Some(data_view.slice(0, actual_data_len))
    }

    pub fn get_cards(&self, location: CardLocation) -> Vec<Option<ActiveCard>> {
        let orig_buf = match self.query_location(0xFFFFFFFF, CardOwner::Player, location) {
            Some(js_array) => js_array.to_vec(), // Fast block copy across WASM boundary
            None => return Vec::new(),
        };

        let mut cards = Vec::new();
        let mut current_sequence: u8 = 0;
        let mut cursor = 0usize;

        while cursor < orig_buf.len() {
            if cursor + 2 <= orig_buf.len() {
                let marker = u16::from_le_bytes([orig_buf[cursor], orig_buf[cursor + 1]]);
                if marker == 0 {
                    cards.push(None);
                    current_sequence += 1;
                    cursor += 2;
                    continue;
                }
            }

            if cursor + 6 > orig_buf.len() {
                panic!("Malformed buffer payload: unexpected end of stream");
            }

            let mut card_code = 0u32;
            let mut position = None;

            loop {
                let length = u16::from_le_bytes([orig_buf[cursor], orig_buf[cursor + 1]]) as usize;
                let record_end = cursor + 2 + length;

                if length < 4 || record_end > orig_buf.len() {
                    panic!("Malformed buffer payload: unexpected end of stream");
                }

                let query_flag = u32::from_le_bytes([
                    orig_buf[cursor + 2],
                    orig_buf[cursor + 3],
                    orig_buf[cursor + 4],
                    orig_buf[cursor + 5],
                ]);

                match query_flag {
                    0x0000_0001 => {
                        if cursor + 10 <= orig_buf.len() {
                            card_code = u32::from_le_bytes([
                                orig_buf[cursor + 6],
                                orig_buf[cursor + 7],
                                orig_buf[cursor + 8],
                                orig_buf[cursor + 9],
                            ]);
                        }
                    }
                    0x0000_0002 => {
                        if cursor + 10 <= orig_buf.len() {
                            let raw_position = u32::from_le_bytes([
                                orig_buf[cursor + 6],
                                orig_buf[cursor + 7],
                                orig_buf[cursor + 8],
                                orig_buf[cursor + 9],
                            ]);
                            position = BattlePosition::try_from(raw_position).ok();
                        }
                    }
                    0x8000_0000 => {
                        cards.push(Some(ActiveCard {
                            card_code,
                            controller: CardController::Player,
                            location,
                            position,
                            sequence: current_sequence,
                            chain_option: None,
                            description: None,
                            is_selected: false,
                        }));
                        current_sequence += 1;
                        cursor = record_end;
                        break;
                    }
                    _ => {}
                }

                cursor = record_end;

                if cursor >= orig_buf.len() {
                    panic!("Malformed buffer payload: unexpected end of stream");
                }

                if cursor + 2 <= orig_buf.len() {
                    let next_marker = u16::from_le_bytes([orig_buf[cursor], orig_buf[cursor + 1]]);
                    if next_marker == 0 {
                        panic!("Malformed buffer payload: missing QUERY_END");
                    }
                }
            }
        }

        cards
    }

    pub fn add_card(
        &self,
        owner: CardOwner,
        code: u32,
        controller: CardController,
        location: CardLocation,
        index: u32,
        position: u32,
    ) {
        let info_ptr = self.core.allocate_memory(24);

        let memory = self.core.get_wasm_memory();
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

        info_view.set_index(0, owner as u8);
        info_view.set_index(1, 0); // Hardcode duelist - this won't ever support tag
        // Padding at 2-3 is left as-is (zeros from allocation)

        write_le_bytes(&info_view, 4, &code.to_le_bytes());

        info_view.set_index(8, controller as u8);
        // Padding at 9-11 is left as-is

        write_le_bytes(&info_view, 12, &(location as isize).to_le_bytes());
        write_le_bytes(&info_view, 16, &index.to_le_bytes());
        write_le_bytes(&info_view, 20, &position.to_le_bytes());

        self.core.instance.add_card(self.handle.into(), info_offset);
    }

    pub fn load_script(&self, script: Vec<u8>, name: &str) -> i32 {
        let name_bytes = name.as_bytes();

        // allocate memory for script and name (name needs +1 for null terminator)
        let script_alloc = self.core.allocate_memory(script.len() as u32);
        let name_alloc = self.core.allocate_memory((name_bytes.len() + 1) as u32);

        let script_ptr = script_alloc.get_pointer();
        let name_ptr = name_alloc.get_pointer();

        let memory = self.core.get_wasm_memory();
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

        self.core.instance.load_script(
            self.handle.0,
            script_ptr.into(),
            script.len() as u32,
            name_ptr.into(),
        )
    }
}
