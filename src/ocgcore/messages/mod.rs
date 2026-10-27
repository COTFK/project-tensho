mod announce_number;
mod draw;
mod hint;
mod idle;
pub mod move_card;
mod new_phase;
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
mod win;

pub use announce_number::AnnounceNumberMessageData;
pub use draw::DrawMessageData;
pub use hint::{Hint, HintMessageData};
pub use idle::IdleMessageData;
pub use move_card::MoveMessageData;
pub use new_phase::NewPhaseMessageData;
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
pub use win::WinMessageData;

use anyhow::Context;
use ocgcore_ffi::constants::{
    MSG_ANNOUNCE_NUMBER, MSG_DRAW, MSG_HINT, MSG_MOVE, MSG_NEW_PHASE, MSG_NEW_TURN, MSG_RETRY,
    MSG_SELECT_CARD, MSG_SELECT_CHAIN, MSG_SELECT_EFFECTYN, MSG_SELECT_IDLECMD, MSG_SELECT_OPTION,
    MSG_SELECT_PLACE, MSG_SELECT_POSITION, MSG_SELECT_TRIBUTE, MSG_SELECT_UNSELECT_CARD,
    MSG_SELECT_YESNO, MSG_SORT_CARD, MSG_WIN,
};

#[derive(Hash, Debug, Clone, PartialEq, Eq)]
pub enum CoreMessage {
    Unsupported(u8),
    Retry,
    NewTurn,
    Hint(HintMessageData),
    Move(MoveMessageData),
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
            MSG_MOVE => Ok(CoreMessage::Move(MoveMessageData::try_from(bytes)?)),
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
                tracing::debug!("Received unhandled message {message_type} with bytes: {bytes:?}");
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
mod tests {
    use super::*;
    use crate::ocgcore::constants::Phase;

    #[test]
    fn parses_framed_messages_in_order() {
        let bytes = [1, 0, 0, 0, 255, 3, 0, 0, 0, 41, 1, 0, 3, 0, 0, 0, 41, 0, 2];
        assert_eq!(
            parse_messages(&bytes).unwrap(),
            vec![
                CoreMessage::Unsupported(255),
                CoreMessage::NewPhase(NewPhaseMessageData { phase: Phase::Draw }),
                CoreMessage::NewPhase(NewPhaseMessageData { phase: Phase::End }),
            ]
        );
        assert!(parse_messages(&[]).unwrap().is_empty());
    }

    #[test]
    fn rejects_invalid_frames_and_truncated_messages() {
        assert!(parse_messages(&[0, 0, 0, 0]).is_err());
        assert!(parse_messages(&[2, 0, 0, 0, 41]).is_err());
        assert!(parse_messages(&[1, 0, 0]).is_err());
        assert!(parse_messages(&[2, 0, 0, 0, 41, 1, 1, 0, 0, 0, 2]).is_err());
    }
}
