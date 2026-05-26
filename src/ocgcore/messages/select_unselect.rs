use crate::ocgcore::utility::read_u8;
use crate::ocgcore::utility::read_u32;
use crate::ocgcore::ActiveCard;
use crate::ocgcore::constants::CardController;
use crate::ocgcore::constants::CardLocation;
use crate::ocgcore::constants::BattlePosition;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectUnselectMessage {
    pub playerid: u8,
    pub finishable: bool,
    pub min_count: u32,
    pub max_count: u32,
    pub preselected: usize,
    pub cards: Vec<ActiveCard>,
}

impl TryFrom<&[u8]> for SelectUnselectMessage {
    type Error = anyhow::Error;

    fn try_from(raw_bytes: &[u8]) -> anyhow::Result<Self, Self::Error> {
        // 1. Handle length headers if the socket layer prepends them
        let message_offset = if raw_bytes[0] == 26 {
            0
        } else if raw_bytes.len() >= 5 && raw_bytes[4] == 26 {
            let declared_len = u32::from_le_bytes(raw_bytes[0..4].try_into().unwrap()) as usize;
            if declared_len != raw_bytes.len().saturating_sub(4) {
                anyhow::bail!("Length prefix mismatch parsing message 26");
            }
            4
        } else {
            anyhow::bail!(
                "Invalid message ID: Expected MSG_SELECT_UNSELECT_CARD (26), got {}",
                raw_bytes.get(0).copied().unwrap_or_default()
            );
        };

        let mut cursor = message_offset + 1;

        let playerid = read_u8(raw_bytes, &mut cursor, "playerid")?;
        let finishable = read_u8(raw_bytes, &mut cursor, "finishable")? != 0;
        let preselected = read_u8(raw_bytes, &mut cursor, "pre_selected_count")? as usize;
        let min_count = read_u32(raw_bytes, &mut cursor, "min_count")?;
        let max_count = read_u32(raw_bytes, &mut cursor, "max_count")?;
        let selectable_count = read_u32(raw_bytes, &mut cursor, "selectable_count")? as usize;

        // Sanity guard against massive/corrupted allocations
        if selectable_count > 100 {
            anyhow::bail!("Impossible selection count detected: {}", selectable_count);
        }

        let mut cards = Vec::with_capacity(selectable_count);

        // 3. Stride loop processing card entries
        for _ in 0..selectable_count {
            let card_code = read_u32(raw_bytes, &mut cursor, "card_code")?;

            // Unpack 4 bytes of location tracking configurations
            let controller = read_u8(raw_bytes, &mut cursor, "controller")?;
            let location = read_u8(raw_bytes, &mut cursor, "location")?;
            let index = read_u8(raw_bytes, &mut cursor, "index")?;
            let position = read_u8(raw_bytes, &mut cursor, "position")?;

            // Check selection status flag
            let is_selected = read_u32(raw_bytes, &mut cursor, "is_selected")? != 0;

            cards.push(ActiveCard {
                card_code,
                controller: CardController::try_from(controller)?,
                location: CardLocation::try_from(location)?,
                position: BattlePosition::try_from(position).ok(),
                index,
                chain_option: None,
                description: None,
                is_selected,
            });
        }

        Ok(Self {
            playerid,
            finishable,
            min_count,
            max_count,
            preselected,
            cards,
        })
    }
}
