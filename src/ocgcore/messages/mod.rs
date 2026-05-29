mod announce_number;
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
pub use idle::IdleMessageData;
pub use select_card::SelectCardMessageData;
pub use select_chain::SelectChainMessageData;
pub use select_effect_yes_no::SelectEffectYNMessageData;
pub use select_option::SelectOptionMessageData;
pub use select_place::SelectPlaceMessageData;
pub use select_place::Zone;
pub use select_position::SelectPositionMessageData;
pub use select_tribute::SelectTributeMessageData;
pub use select_unselect_card::SelectUnselectMessageData;
pub use select_yes_no::SelectYesNoMessageData;
pub use sort_card::SortCardMessageData;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CoreMessage {
    Retry,
    Idle(IdleMessageData),
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

        tracing::debug!("Received message {message_type:?} with bytes: {bytes:?}");

        match message_type {
            1 => Ok(CoreMessage::Retry),
            11 => Ok(CoreMessage::Idle(IdleMessageData::try_from(bytes)?)),
            12 => Ok(CoreMessage::SelectEffectYN(
                SelectEffectYNMessageData::try_from(bytes)?,
            )),
            13 => Ok(CoreMessage::SelectYesNo(SelectYesNoMessageData::try_from(
                bytes,
            )?)),
            14 => Ok(CoreMessage::SelectOption(
                SelectOptionMessageData::try_from(bytes)?,
            )),
            15 => Ok(CoreMessage::SelectCard(SelectCardMessageData::try_from(
                bytes,
            )?)),
            16 => Ok(CoreMessage::SelectChain(SelectChainMessageData::try_from(
                bytes,
            )?)),
            18 => Ok(CoreMessage::SelectPlace(SelectPlaceMessageData::try_from(
                bytes,
            )?)),
            19 => Ok(CoreMessage::SelectPosition(
                SelectPositionMessageData::try_from(bytes)?,
            )),
            20 => Ok(CoreMessage::SelectTribute(
                SelectTributeMessageData::try_from(bytes)?,
            )),
            25 => Ok(CoreMessage::SortCard(SortCardMessageData::try_from(bytes)?)),
            26 => Ok(CoreMessage::SelectUnselectCard(
                SelectUnselectMessageData::try_from(bytes)?,
            )),
            143 => Ok(CoreMessage::AnnounceNumber(
                AnnounceNumberMessageData::try_from(bytes)?,
            )),
            _ => anyhow::bail!("Received wrong message: {message_type}"),
        }
    }
}
