use crate::ocgcore::constants::CardLocation;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Zone {
    pub location: CardLocation,
    pub sequence: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct SelectPlaceMessageData {
    pub player: u8,
    pub count: u8,
    pub zones: Vec<Zone>,
}

impl TryFrom<&[u8]> for SelectPlaceMessageData {
    type Error = anyhow::Error;

    fn try_from(bytes: &[u8]) -> anyhow::Result<Self, Self::Error> {
        let mut zones = Vec::new();

        let player = bytes[5];
        let count = bytes[6];
        let zone_mask = u32::from_le_bytes([bytes[7], bytes[8], bytes[9], bytes[10]]);

        // Main + Extra Monster Zones
        for sequence in 0..7 {
            if (zone_mask & (1 << sequence)) == 0 {
                zones.push(Zone {
                    location: CardLocation::MonsterZone,
                    sequence,
                });
            }
        }

        // Spell/Trap Zones
        for sequence in 0..5 {
            if (zone_mask & (1 << (8 + sequence))) == 0 {
                zones.push(Zone {
                    location: CardLocation::SpellTrapZone,
                    sequence,
                });
            }
        }

        // Field Zone
        if (zone_mask & (1 << 13)) == 0 {
            zones.push(Zone {
                location: CardLocation::SpellTrapZone,
                sequence: 5,
            });
        }

        Ok(SelectPlaceMessageData {
            player,
            count,
            zones,
        })
    }
}
