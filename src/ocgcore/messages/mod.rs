mod announce_number;
mod become_target;
mod card_event;
pub mod card_hint;
mod card_locations;
mod card_selected;
mod chaining;
pub mod confirm_cards;
pub(crate) mod draw;
mod hint;
mod idle;
pub mod move_card;
mod new_phase;
mod position_change;
mod select_card;
mod select_chain;
mod select_effect_yes_no;
mod select_option;
mod select_place;
mod select_position;
mod select_tribute;
mod select_unselect_card;
mod select_yes_no;
mod set_card;
mod shuffle_deck;
mod shuffle_hand;
mod sort_card;
mod summoning;
mod win;

pub use announce_number::AnnounceNumberMessageData;
pub use become_target::BecomeTargetMessageData;
pub use card_hint::CardHintMessageData;
pub use card_selected::CardSelectedMessageData;
pub use chaining::{ChainLinkMessageData, ChainingMessageData};
pub use confirm_cards::ConfirmCardsMessageData;
pub use draw::DrawMessageData;
pub use hint::{Hint, HintMessageData};
pub use idle::IdleMessageData;
pub use move_card::MoveMessageData;
pub use new_phase::NewPhaseMessageData;
pub use position_change::PositionChangeMessageData;
pub use select_card::SelectCardMessageData;
pub use select_chain::SelectChainMessageData;
pub use select_effect_yes_no::SelectEffectYNMessageData;
pub use select_option::SelectOptionMessageData;
pub use select_place::SelectPlaceMessageData;
pub use select_position::SelectPositionMessageData;
pub use select_tribute::SelectTributeMessageData;
pub use select_unselect_card::SelectUnselectMessageData;
pub use select_yes_no::SelectYesNoMessageData;
pub use set_card::SetCardMessageData;
pub use shuffle_deck::ShuffleDeckMessageData;
pub use shuffle_hand::ShuffleHandMessageData;
pub use sort_card::SortCardMessageData;
pub use summoning::SummoningMessageData;
pub use win::WinMessageData;

use anyhow::Context;
use ocgcore_ffi::constants::{
    MSG_ANNOUNCE_NUMBER, MSG_BECOME_TARGET, MSG_CARD_HINT, MSG_CARD_SELECTED, MSG_CHAIN_DISABLED,
    MSG_CHAIN_END, MSG_CHAIN_SOLVED, MSG_CHAIN_SOLVING, MSG_CHAINED, MSG_CHAINING,
    MSG_CONFIRM_CARDS, MSG_DRAW, MSG_FLIPSUMMONED, MSG_FLIPSUMMONING, MSG_HINT, MSG_MOVE,
    MSG_NEW_PHASE, MSG_NEW_TURN, MSG_POS_CHANGE, MSG_RETRY, MSG_SELECT_CARD, MSG_SELECT_CHAIN,
    MSG_SELECT_EFFECTYN, MSG_SELECT_IDLECMD, MSG_SELECT_OPTION, MSG_SELECT_PLACE,
    MSG_SELECT_POSITION, MSG_SELECT_TRIBUTE, MSG_SELECT_UNSELECT_CARD, MSG_SELECT_YESNO, MSG_SET,
    MSG_SHUFFLE_DECK, MSG_SHUFFLE_HAND, MSG_SORT_CARD, MSG_SPSUMMONED, MSG_SPSUMMONING,
    MSG_SUMMONED, MSG_SUMMONING, MSG_WIN,
};

#[derive(Hash, Debug, Clone, PartialEq, Eq)]
pub enum CoreMessage {
    Unsupported(u8),
    Retry,
    NewTurn,
    Hint(HintMessageData),
    BecomeTarget(BecomeTargetMessageData),
    CardHint(CardHintMessageData),
    CardSelected(CardSelectedMessageData),
    ConfirmCards(ConfirmCardsMessageData),
    ShuffleDeck(ShuffleDeckMessageData),
    ShuffleHand(ShuffleHandMessageData),
    Move(MoveMessageData),
    PositionChange(PositionChangeMessageData),
    SetCard(SetCardMessageData),
    Summoning(SummoningMessageData),
    SpecialSummoning(SummoningMessageData),
    FlipSummoning(SummoningMessageData),
    Summoned,
    SpecialSummoned,
    FlipSummoned,
    Chaining(ChainingMessageData),
    Chained(ChainLinkMessageData),
    ChainSolving(ChainLinkMessageData),
    ChainSolved(ChainLinkMessageData),
    ChainDisabled(ChainLinkMessageData),
    ChainEnd,
    NewPhase(NewPhaseMessageData),
    Win(WinMessageData),
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
        let message_type = *bytes.get(4).context("Missing message type at offset 4")?;
        match u32::from(message_type) {
            MSG_RETRY => Ok(CoreMessage::Retry),
            MSG_NEW_TURN => Ok(CoreMessage::NewTurn),
            MSG_HINT => Ok(CoreMessage::Hint(HintMessageData::try_from(bytes)?)),
            MSG_BECOME_TARGET => Ok(CoreMessage::BecomeTarget(
                BecomeTargetMessageData::try_from(bytes)?,
            )),
            MSG_CARD_HINT => Ok(CoreMessage::CardHint(CardHintMessageData::try_from(bytes)?)),
            MSG_CARD_SELECTED => Ok(CoreMessage::CardSelected(
                CardSelectedMessageData::try_from(bytes)?,
            )),
            MSG_CONFIRM_CARDS => Ok(CoreMessage::ConfirmCards(
                ConfirmCardsMessageData::try_from(bytes)?,
            )),
            MSG_SHUFFLE_DECK => Ok(CoreMessage::ShuffleDeck(ShuffleDeckMessageData::try_from(
                bytes,
            )?)),
            MSG_SHUFFLE_HAND => Ok(CoreMessage::ShuffleHand(ShuffleHandMessageData::try_from(
                bytes,
            )?)),
            MSG_MOVE => Ok(CoreMessage::Move(MoveMessageData::try_from(bytes)?)),
            MSG_POS_CHANGE => Ok(CoreMessage::PositionChange(
                PositionChangeMessageData::try_from(bytes)?,
            )),
            MSG_SET => Ok(CoreMessage::SetCard(SetCardMessageData::try_from(bytes)?)),
            MSG_SUMMONING => Ok(CoreMessage::Summoning(SummoningMessageData::try_from(
                bytes,
            )?)),
            MSG_SPSUMMONING => Ok(CoreMessage::SpecialSummoning(
                SummoningMessageData::try_from(bytes)?,
            )),
            MSG_FLIPSUMMONING => Ok(CoreMessage::FlipSummoning(SummoningMessageData::try_from(
                bytes,
            )?)),
            MSG_SUMMONED | MSG_SPSUMMONED | MSG_FLIPSUMMONED => {
                let length = u32::from_le_bytes(bytes[..4].try_into()?);
                anyhow::ensure!(length == 1, "Invalid summoned message length: {length}");
                Ok(match u32::from(message_type) {
                    MSG_SUMMONED => CoreMessage::Summoned,
                    MSG_SPSUMMONED => CoreMessage::SpecialSummoned,
                    _ => CoreMessage::FlipSummoned,
                })
            }
            MSG_CHAINING => Ok(CoreMessage::Chaining(ChainingMessageData::try_from(bytes)?)),
            MSG_CHAINED => Ok(CoreMessage::Chained(ChainLinkMessageData::try_from(bytes)?)),
            MSG_CHAIN_SOLVING => Ok(CoreMessage::ChainSolving(ChainLinkMessageData::try_from(
                bytes,
            )?)),
            MSG_CHAIN_SOLVED => Ok(CoreMessage::ChainSolved(ChainLinkMessageData::try_from(
                bytes,
            )?)),
            MSG_CHAIN_DISABLED => Ok(CoreMessage::ChainDisabled(ChainLinkMessageData::try_from(
                bytes,
            )?)),
            MSG_CHAIN_END => {
                let length = u32::from_le_bytes(bytes[..4].try_into()?);
                anyhow::ensure!(length == 1, "Invalid chain end message length: {length}");
                Ok(CoreMessage::ChainEnd)
            }
            MSG_NEW_PHASE => Ok(CoreMessage::NewPhase(NewPhaseMessageData::try_from(bytes)?)),
            MSG_WIN => Ok(CoreMessage::Win(WinMessageData::try_from(bytes)?)),
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
                tracing::error!("Received unhandled message {message_type} with bytes: {bytes:?}");
                Ok(CoreMessage::Unsupported(message_type))
            }
        }
    }
}

pub fn parse_messages(bytes: &[u8]) -> anyhow::Result<Vec<CoreMessage>> {
    let mut remaining = bytes;
    let mut messages = Vec::new();
    while !remaining.is_empty() {
        let length_bytes = remaining.get(..4).context("Missing message length")?;
        let length = u32::from_le_bytes(length_bytes.try_into()?) as usize;
        anyhow::ensure!(length > 0, "Invalid empty message");
        let end = 4usize
            .checked_add(length)
            .context("Message length overflow")?;
        let message = remaining.get(..end).context("Truncated message")?;
        messages.push(CoreMessage::try_from(message).with_context(|| {
            format!(
                "Failed to parse message at offset {}",
                bytes.len() - remaining.len()
            )
        })?);
        remaining = &remaining[end..];
    }
    Ok(messages)
}

#[cfg(test)]
#[path = "../../../tests/unit/ocgcore/messages/mod.rs"]
mod tests;
