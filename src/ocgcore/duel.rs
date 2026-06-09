use std::ffi::c_void;

use ocgcore_ffi::types::OCG_Duel;
use ocgcore_ffi::types::OCG_NewCardInfo;
use ocgcore_ffi::types::OCG_QueryInfo;

use super::OCGCore;
use super::constants::CardLocation;
use super::constants::DuelStatus;
use crate::ocgcore::CardData;
use crate::ocgcore::Response;
use crate::ocgcore::constants::BattlePosition;
use crate::ocgcore::constants::{CardController, CardOwner};
use crate::ocgcore::data::CardType;
use crate::ocgcore::data::HandCard;
use crate::ocgcore::messages::CoreMessage;

#[derive(Debug, Clone, PartialEq)]
pub struct Duel {
    pub(super) handle: OCG_Duel,
    core: OCGCore,
}

impl Duel {
    pub fn new(handle: OCG_Duel, core: &OCGCore) -> Self {
        Self {
            handle,
            core: core.clone(),
        }
    }

    pub fn start(&self) {
        unsafe { ocgcore_ffi::OCG_StartDuel(self.handle) };
    }

    pub fn set_response(&self, response: Response) {
        let bytes = response.get_response_bytes();
        let buf_len = bytes.len() as u32;

        tracing::debug!("Sending response {response:?} with bytes {bytes:?}");

        unsafe {
            ocgcore_ffi::OCG_DuelSetResponse(self.handle, bytes.as_ptr() as *const c_void, buf_len);
        }
    }

    pub fn parse_messages(&self) -> CoreMessage {
        let mut length = 0;
        let messages_ptr =
            unsafe { ocgcore_ffi::OCG_DuelGetMessage(self.handle, &mut length) } as *mut u8;
        let messages = unsafe { std::slice::from_raw_parts(messages_ptr, length as usize) };

        CoreMessage::try_from(messages).unwrap()
    }

    pub fn process(&self) -> DuelStatus {
        let status = unsafe { ocgcore_ffi::OCG_DuelProcess(self.handle) };
        DuelStatus::try_from(status).unwrap()
    }

    pub fn destroy(&self) {
        unsafe {
            ocgcore_ffi::OCG_DestroyDuel(self.handle);
        }
    }

    pub fn count_location(&self, team: CardOwner, location: CardLocation) -> u32 {
        unsafe { ocgcore_ffi::OCG_DuelQueryCount(self.handle, team as u8, location as u32) }
    }

    fn query_location(
        &self,
        flags: u32,
        team: CardOwner,
        location: CardLocation,
    ) -> Option<Vec<u8>> {
        let query = OCG_QueryInfo {
            flags,
            con: team as u8,
            loc: location as u32,
            seq: 0,
            overlay_seq: 0,
        };

        let mut length = 0;
        let ptr = unsafe {
            ocgcore_ffi::OCG_DuelQueryLocation(self.handle, &mut length, &query) as *mut u8
        };
        let data = unsafe { std::slice::from_raw_parts(ptr, length as usize) };

        Some(data.to_vec())
    }

    pub fn get_raw_hand(&self) -> Vec<HandCard> {
        let mut hand = Vec::new();
        let raw_cards = self.get_cards(CardLocation::Hand);

        for (index, card) in raw_cards.iter().enumerate() {
            hand.push(HandCard {
                index,
                code: card.unwrap().card_code,
                is_activatable_or_chainable: false,
                activate_index: None,
                chain_index: None,
                normal_summon_index: None,
                monster_set_index: None,
                spell_trap_set_index: None,
            });
        }

        hand
    }

    pub fn get_cards(&self, location: CardLocation) -> Vec<Option<CardData>> {
        let orig_buf = match self.query_location(0xFFFFFFFF, CardOwner::Player, location) {
            Some(js_array) => js_array,
            None => return Vec::new(),
        };

        if orig_buf.len() < 4 {
            return Vec::new();
        }

        let payload_len =
            u32::from_le_bytes([orig_buf[0], orig_buf[1], orig_buf[2], orig_buf[3]]) as usize;
        if payload_len + 4 > orig_buf.len() {
            return Vec::new();
        }
        let payload = &orig_buf[4..4 + payload_len];

        let mut cards = Vec::new();
        let mut current_sequence: u8 = 0;
        let mut cursor = 0usize;

        while cursor < payload.len() {
            if cursor + 2 <= payload.len() {
                let marker = u16::from_le_bytes([payload[cursor], payload[cursor + 1]]);
                if marker == 0 {
                    cards.push(None);
                    current_sequence += 1;
                    cursor += 2;
                    continue;
                }
            }

            if cursor + 6 > payload.len() {
                panic!("Malformed buffer payload: unexpected end of stream");
            }

            let mut card_code = 0u32;
            let mut card_type = CardType::empty();
            let mut position = None;
            let mut attack = None;
            let mut defense = None;
            let mut level = None;

            loop {
                let length = u16::from_le_bytes([payload[cursor], payload[cursor + 1]]) as usize;
                let record_end = cursor + 2 + length;

                if length < 4 || record_end > payload.len() {
                    panic!("Malformed buffer payload: unexpected end of stream");
                }

                let query_flag = u32::from_le_bytes([
                    payload[cursor + 2],
                    payload[cursor + 3],
                    payload[cursor + 4],
                    payload[cursor + 5],
                ]);

                match query_flag {
                    0x0000_0001 if cursor + 10 <= payload.len() => {
                        card_code = u32::from_le_bytes([
                            payload[cursor + 6],
                            payload[cursor + 7],
                            payload[cursor + 8],
                            payload[cursor + 9],
                        ]);
                    }
                    0x0000_0002 if cursor + 10 <= payload.len() => {
                        let raw_position = u32::from_le_bytes([
                            payload[cursor + 6],
                            payload[cursor + 7],
                            payload[cursor + 8],
                            payload[cursor + 9],
                        ]);
                        position = BattlePosition::try_from(raw_position).ok();
                    }
                    0x0000_0008 if cursor + 10 <= payload.len() => {
                        let raw_card_type = u32::from_le_bytes([
                            payload[cursor + 6],
                            payload[cursor + 7],
                            payload[cursor + 8],
                            payload[cursor + 9],
                        ]);

                        card_type = CardType::from_bits_truncate(raw_card_type);
                    }
                    0x0000_0010 if cursor + 10 <= payload.len() => {
                        level = Some(u32::from_le_bytes([
                            payload[cursor + 6],
                            payload[cursor + 7],
                            payload[cursor + 8],
                            payload[cursor + 9],
                        ]));
                    }
                    0x0000_0100 if cursor + 10 <= payload.len() => {
                        attack = Some(u32::from_le_bytes([
                            payload[cursor + 6],
                            payload[cursor + 7],
                            payload[cursor + 8],
                            payload[cursor + 9],
                        ]));
                    }
                    0x0000_0200 if cursor + 10 <= payload.len() => {
                        defense = Some(u32::from_le_bytes([
                            payload[cursor + 6],
                            payload[cursor + 7],
                            payload[cursor + 8],
                            payload[cursor + 9],
                        ]));
                    }
                    0x8000_0000 => {
                        cards.push(Some(CardData {
                            card_code,
                            controller: CardController::Player,
                            location,
                            position,
                            sequence: current_sequence,
                            action_index: None,
                            description: None,
                            is_selected: false,
                            level,
                            attack,
                            defense,
                            card_type,
                        }));
                        current_sequence += 1;
                        cursor = record_end;
                        break;
                    }
                    _ => {}
                }

                cursor = record_end;

                if cursor >= payload.len() {
                    panic!("Malformed buffer payload: unexpected end of stream");
                }

                if cursor + 2 <= payload.len() {
                    let next_marker = u16::from_le_bytes([payload[cursor], payload[cursor + 1]]);
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
        let card = OCG_NewCardInfo {
            team: 0,
            duelist: owner as u8,
            code,
            con: controller as u8,
            loc: location as u32,
            seq: index,
            pos: position,
        };

        unsafe { ocgcore_ffi::OCG_DuelNewCard(self.handle, &card) };
    }
}
