use js_sys::Uint8Array;
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::*;

use crate::ocgcore::data::OCGCardData;
use crate::ocgcore::ffi::OCGCoreInstance;

pub struct CardReader(pub JsValue);

impl CardReader {
    pub fn new<CardReaderFn>(instance: OCGCoreInstance, mut card_reader_fn: CardReaderFn) -> Self
    where
        CardReaderFn: FnMut(u32) -> OCGCardData + 'static,
    {
        let card_reader = Closure::wrap(Box::new(move |_: u32, code: u32, data_ptr: u32| {
            if data_ptr == 0 {
                return;
            }

            let mut data = card_reader_fn(code);

            let buffer = instance.get_wasm_memory().buffer();

            // Allocate memory for setcodes array (u16 values + terminator)
            // Format: [setcode1: u16][setcode2: u16]...[0x0000: u16 terminator]
            // For now we handle single setcode values - convert to array
            let setcodes_array = if data.setcodes > 0 {
                // Allocate 4 bytes: 2 bytes for the setcode value + 2 bytes for terminator (0x0000)
                let setcodes_ptr = instance.malloc(4);

                // Write setcode as u16 at offset 0
                let setcode_u16 = (data.setcodes & 0xFFFF) as u16;
                let setcode_bytes = setcode_u16.to_le_bytes();
                let sc_view = Uint8Array::new_with_byte_offset_and_length(&buffer, setcodes_ptr, 2);
                sc_view.set(&Uint8Array::from(setcode_bytes.as_slice()), 0);

                // Write terminator (0x0000) at offset 2
                let term_view =
                    Uint8Array::new_with_byte_offset_and_length(&buffer, setcodes_ptr + 2, 2);
                term_view.set(&Uint8Array::from(&[0u8, 0u8][..]), 0);

                setcodes_ptr
            } else {
                0
            };

            // Update the setcodes field to point to the allocated array
            data.setcodes = setcodes_array;

            // Write OCGCardData to WASM memory
            let mut data_bytes = [0u8; std::mem::size_of::<OCGCardData>()];
            data.write_bytes(&mut data_bytes);

            Uint8Array::new_with_byte_offset_and_length(
                &buffer,
                data_ptr,
                std::mem::size_of::<OCGCardData>() as u32,
            )
            .set(&Uint8Array::from(data_bytes.as_slice()), 0);
        }) as Box<dyn FnMut(u32, u32, u32)>);

        Self(card_reader.into_js_value())
    }
}
