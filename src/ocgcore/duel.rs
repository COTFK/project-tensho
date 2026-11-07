use std::ffi::c_void;
use std::rc::Rc;

use ocgcore_ffi::types::OCG_Duel;
use ocgcore_ffi::types::OCG_NewCardInfo;
use ocgcore_ffi::types::OCG_QueryInfo;

use super::OCGCore;
use super::constants::CardLocation;
use crate::ocgcore::CardData;
use crate::ocgcore::Response;
use crate::ocgcore::constants::Player;
use crate::ocgcore::data::HandCard;
use crate::ocgcore::messages::CoreMessage;
use crate::ocgcore::messages::parse_messages;
use crate::ocgcore::query::decode_location;
use ocgcore_ffi::enums::OCG_DuelStatus;

struct DuelInner {
    handle: OCG_Duel,
    // Keeps the core, and its script reader, alive while the engine can call back into it.
    _core: OCGCore,
}

impl Drop for DuelInner {
    fn drop(&mut self) {
        unsafe {
            ocgcore_ffi::OCG_DestroyDuel(self.handle);
        }
    }
}

#[derive(Clone)]
pub struct Duel {
    inner: Rc<DuelInner>,
}

impl Duel {
    pub fn ptr_eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.inner, &other.inner)
    }

    pub fn new(handle: OCG_Duel, core: &OCGCore) -> Self {
        Self {
            inner: Rc::new(DuelInner {
                handle,
                _core: core.clone(),
            }),
        }
    }

    pub(super) fn handle(&self) -> OCG_Duel {
        self.inner.handle
    }

    pub fn start(&self) {
        unsafe { ocgcore_ffi::OCG_StartDuel(self.inner.handle) };
    }

    pub fn set_response(&self, response: Response) {
        let bytes = response.get_response_bytes();
        let buf_len = bytes.len() as u32;

        unsafe {
            ocgcore_ffi::OCG_DuelSetResponse(
                self.inner.handle,
                bytes.as_ptr() as *const c_void,
                buf_len,
            );
        }
    }

    pub fn parse_messages(&self) -> Vec<CoreMessage> {
        let mut length = 0;
        let messages_ptr =
            unsafe { ocgcore_ffi::OCG_DuelGetMessage(self.inner.handle, &mut length) } as *mut u8;
        if length == 0 {
            return Vec::new();
        }
        let messages = unsafe { std::slice::from_raw_parts(messages_ptr, length as usize) };
        parse_messages(messages).expect("Invalid core message buffer")
    }

    pub fn process(&self) -> OCG_DuelStatus {
        let status = unsafe { ocgcore_ffi::OCG_DuelProcess(self.inner.handle) };
        OCG_DuelStatus::try_from(status).unwrap()
    }

    pub fn count_location(&self, team: Player, location: CardLocation) -> u32 {
        unsafe { ocgcore_ffi::OCG_DuelQueryCount(self.inner.handle, team as u8, location as u32) }
    }

    fn query_location(&self, flags: u32, team: Player, location: CardLocation) -> Vec<u8> {
        let query = OCG_QueryInfo {
            flags,
            con: team as u8,
            loc: location as u32,
            seq: 0,
            overlay_seq: 0,
        };

        let mut length = 0;
        let ptr = unsafe {
            ocgcore_ffi::OCG_DuelQueryLocation(self.inner.handle, &mut length, &query) as *mut u8
        };
        if length == 0 {
            return Vec::new();
        }
        assert!(!ptr.is_null(), "Core returned a null query buffer");
        unsafe { std::slice::from_raw_parts(ptr, length as usize) }.to_vec()
    }

    pub fn get_raw_hand(&self) -> Vec<HandCard> {
        let mut hand = Vec::new();
        let raw_cards = self.get_cards(CardLocation::Hand);

        for (index, card) in raw_cards.iter().enumerate() {
            hand.push(HandCard {
                index,
                code: card.unwrap().card_code,
            });
        }

        hand
    }

    pub fn get_cards(&self, location: CardLocation) -> Vec<Option<CardData>> {
        self.query_cards(Player::Player, location)
            .unwrap_or_else(|error| panic!("Invalid {location} query: {error:#}"))
    }

    pub(crate) fn query_cards(
        &self,
        player: Player,
        location: CardLocation,
    ) -> anyhow::Result<Vec<Option<CardData>>> {
        let data = self.query_location(u32::MAX, player, location);
        decode_location(&data, player, location)
    }

    pub fn add_card(
        &self,
        owner: Player,
        code: u32,
        controller: Player,
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

        unsafe { ocgcore_ffi::OCG_DuelNewCard(self.inner.handle, &card) };
    }
}
