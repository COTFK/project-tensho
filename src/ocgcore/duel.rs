use std::ffi::c_void;
use std::rc::Rc;

use ocgcore_ffi::constants::{
    QUERY_ATTACK, QUERY_CODE, QUERY_DEFENSE, QUERY_LEVEL, QUERY_LINK, QUERY_POSITION, QUERY_RANK,
    QUERY_TYPE,
};
use ocgcore_ffi::types::OCG_Duel;
use ocgcore_ffi::types::OCG_NewCardInfo;
use ocgcore_ffi::types::OCG_QueryInfo;

use super::OCGCore;
use super::constants::CardLocation;
use crate::ocgcore::DuelCardData;
use crate::ocgcore::Response;
use crate::ocgcore::constants::Player;
use crate::ocgcore::messages::CoreMessage;
use crate::ocgcore::messages::parse_messages;
use crate::ocgcore::query::decode_card_queries;
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

    pub(crate) fn query_cards(
        &self,
        player: Player,
        location: CardLocation,
    ) -> anyhow::Result<Vec<Option<DuelCardData>>> {
        let mut flags = QUERY_CODE | QUERY_POSITION | QUERY_TYPE;
        if matches!(
            location,
            CardLocation::MonsterZone | CardLocation::SpellTrapZone
        ) {
            flags |= QUERY_LEVEL | QUERY_RANK | QUERY_ATTACK | QUERY_DEFENSE | QUERY_LINK;
        }
        let query = OCG_QueryInfo {
            flags,
            con: player as u8,
            loc: location as u32,
            seq: 0,
            overlay_seq: 0,
        };

        let mut length = 0;
        let ptr = unsafe {
            ocgcore_ffi::OCG_DuelQueryLocation(self.inner.handle, &mut length, &query) as *const u8
        };
        if length == 0 {
            anyhow::bail!("Core returned an empty query buffer");
        }
        assert!(!ptr.is_null(), "Core returned a null query buffer");
        // Decode before another core call can invalidate the query buffer.
        let data = unsafe { std::slice::from_raw_parts(ptr, length as usize) };
        decode_card_queries(data, player, location)
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
