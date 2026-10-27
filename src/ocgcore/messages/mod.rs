mod announce_number;
mod draw;
mod idle;
mod select_card;
mod select_chain;
mod select_effect_yes_no;
mod select_option;
mod select_place;
mod select_position;
mod select_tribute;
mod select_unselect_card;
mod select_yes_no;
mod sort_card;

pub use announce_number::AnnounceNumberMessageData;
pub use draw::DrawMessageData;
pub use idle::IdleMessageData;
pub use select_card::SelectCardMessageData;
pub use select_chain::SelectChainMessageData;
pub use select_effect_yes_no::SelectEffectYNMessageData;
pub use select_option::SelectOptionMessageData;
pub use select_place::SelectPlaceMessageData;
pub use select_position::SelectPositionMessageData;
pub use select_tribute::SelectTributeMessageData;
pub use select_unselect_card::SelectUnselectMessageData;
pub use select_yes_no::SelectYesNoMessageData;
pub use sort_card::SortCardMessageData;

use ocgcore_ffi::constants::{
    MSG_ANNOUNCE_NUMBER, MSG_DRAW, MSG_NEW_TURN, MSG_RETRY, MSG_SELECT_CARD, MSG_SELECT_CHAIN,
    MSG_SELECT_EFFECTYN, MSG_SELECT_IDLECMD, MSG_SELECT_OPTION, MSG_SELECT_PLACE,
    MSG_SELECT_POSITION, MSG_SELECT_TRIBUTE, MSG_SELECT_UNSELECT_CARD, MSG_SELECT_YESNO,
    MSG_SORT_CARD,
};

#[derive(Hash, Debug, Clone, PartialEq, Eq)]
pub enum CoreMessage {
    Unsupported(u8),
    Retry,
    NewTurn,
    Idle(IdleMessageData),
    Draw(DrawMessageData),
    SelectEffectYN(SelectEffectYNMessageData),
    SelectYesNo(SelectYesNoMessageData),
    SelectCard(SelectCardMessageData),
    SelectChain(SelectChainMessageData),
    SelectPlace(SelectPlaceMessageData),
    SelectPosition(SelectPositionMessageData),
    SelectTribute(SelectTributeMessageData),
    SelectUnselectCard(SelectUnselectMessageData),
    SelectOption(SelectOptionMessageData),
    AnnounceNumber(AnnounceNumberMessageData),
    SortCard(SortCardMessageData),
}

impl TryFrom<&[u8]> for CoreMessage {
    type Error = anyhow::Error;

    fn try_from(bytes: &[u8]) -> anyhow::Result<Self, Self::Error> {
        let message_type = bytes[4];
        match u32::from(message_type) {
            MSG_RETRY => Ok(CoreMessage::Retry),
            MSG_NEW_TURN => Ok(CoreMessage::NewTurn),
            MSG_SELECT_IDLECMD => Ok(CoreMessage::Idle(IdleMessageData::try_from(bytes)?)),
            MSG_SELECT_EFFECTYN => Ok(CoreMessage::SelectEffectYN(
                SelectEffectYNMessageData::try_from(bytes)?,
            )),
            MSG_SELECT_YESNO => Ok(CoreMessage::SelectYesNo(SelectYesNoMessageData::try_from(
                bytes,
            )?)),
            MSG_SELECT_OPTION => Ok(CoreMessage::SelectOption(
                SelectOptionMessageData::try_from(bytes)?,
            )),
            MSG_SELECT_CARD => Ok(CoreMessage::SelectCard(SelectCardMessageData::try_from(
                bytes,
            )?)),
            MSG_SELECT_CHAIN => Ok(CoreMessage::SelectChain(SelectChainMessageData::try_from(
                bytes,
            )?)),
            MSG_SELECT_PLACE => Ok(CoreMessage::SelectPlace(SelectPlaceMessageData::try_from(
                bytes,
            )?)),
            MSG_SELECT_POSITION => Ok(CoreMessage::SelectPosition(
                SelectPositionMessageData::try_from(bytes)?,
            )),
            MSG_SELECT_TRIBUTE => Ok(CoreMessage::SelectTribute(
                SelectTributeMessageData::try_from(bytes)?,
            )),
            MSG_SORT_CARD => Ok(CoreMessage::SortCard(SortCardMessageData::try_from(bytes)?)),
            MSG_SELECT_UNSELECT_CARD => Ok(CoreMessage::SelectUnselectCard(
                SelectUnselectMessageData::try_from(bytes)?,
            )),
            MSG_DRAW => Ok(CoreMessage::Draw(DrawMessageData::try_from(bytes)?)),
            MSG_ANNOUNCE_NUMBER => Ok(CoreMessage::AnnounceNumber(
                AnnounceNumberMessageData::try_from(bytes)?,
            )),
            _ => {
                tracing::debug!("Received unhandled message {message_type} with bytes: {bytes:?}");
                Ok(CoreMessage::Unsupported(message_type))
            },
        }
    }
}
