use anyhow::{Context, ensure};

use crate::ocgcore::constants::{CardLocation, Player};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct CardId(u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CardAddress {
    pub controller: Player,
    pub location: CardLocation,
    pub sequence: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Card {
    pub id: CardId,
    pub code: u32,
    pub address: CardAddress,
}

#[derive(Debug, Default)]
pub(crate) struct CardLayer {
    cards: Vec<Card>,
    next_id: u64,
}

impl CardLayer {
    pub fn from_decks(main_deck: &[u32], extra_deck: &[u32]) -> anyhow::Result<Self> {
        let mut layer = Self::default();
        for (location, codes) in [
            (CardLocation::Deck, main_deck),
            (CardLocation::ExtraDeck, extra_deck),
        ] {
            for (sequence, &code) in codes.iter().enumerate() {
                layer.insert(
                    CardAddress {
                        controller: Player::Player,
                        location,
                        sequence: u32::try_from(sequence)?,
                    },
                    code,
                )?;
            }
        }
        Ok(layer)
    }

    pub fn insert(&mut self, address: CardAddress, code: u32) -> anyhow::Result<CardId> {
        ensure!(
            self.card_at(address).is_none(),
            "Card address is already occupied"
        );
        let id = CardId(self.next_id);
        self.next_id = self.next_id.checked_add(1).context("Card IDs exhausted")?;
        self.cards.push(Card { id, code, address });
        Ok(id)
    }

    pub fn cards(&self) -> &[Card] {
        &self.cards
    }

    pub fn card_at(&self, address: CardAddress) -> Option<&Card> {
        self.cards.iter().find(|card| card.address == address)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn address(controller: Player, location: CardLocation, sequence: u32) -> CardAddress {
        CardAddress {
            controller,
            location,
            sequence,
        }
    }

    #[test]
    fn initializes_ordered_decks_and_distinguishes_copies() {
        let layer = CardLayer::from_decks(&[7, 8, 7], &[9, 9]).unwrap();
        let cards = layer.cards();
        assert_eq!(
            cards
                .iter()
                .map(|card| (card.address.location, card.address.sequence, card.code))
                .collect::<Vec<_>>(),
            [
                (CardLocation::Deck, 0, 7),
                (CardLocation::Deck, 1, 8),
                (CardLocation::Deck, 2, 7),
                (CardLocation::ExtraDeck, 0, 9),
                (CardLocation::ExtraDeck, 1, 9),
            ]
        );
        assert_ne!(cards[0].id, cards[2].id);
        assert_ne!(cards[3].id, cards[4].id);
        assert!(
            layer
                .card_at(address(Player::Opponent, CardLocation::Deck, 0))
                .is_none()
        );
    }

    #[test]
    fn initializes_empty_decks() {
        assert!(CardLayer::from_decks(&[], &[]).unwrap().cards().is_empty());
    }

    #[test]
    fn initializes_sequences_beyond_u8() {
        let deck: Vec<u32> = (1..=300).collect();
        let layer = CardLayer::from_decks(&deck, &[]).unwrap();
        let card = layer
            .card_at(address(Player::Player, CardLocation::Deck, 299))
            .unwrap();
        assert_eq!(card.code, 300);
        assert_eq!(card.id, CardId(299));
        assert_eq!(layer.cards().len(), 300);
    }

    #[test]
    fn looks_up_cards_by_full_address() {
        let mut layer = CardLayer::default();
        for (player, location, sequence, code) in [
            (Player::Player, CardLocation::Deck, u32::MAX, 1),
            (Player::Player, CardLocation::Deck, 0, 2),
            (Player::Player, CardLocation::Deck, 256, 3),
            (Player::Player, CardLocation::ExtraDeck, 0, 4),
            (Player::Opponent, CardLocation::Deck, 0, 5),
        ] {
            layer
                .insert(address(player, location, sequence), code)
                .unwrap();
        }
        for (player, location, sequence, code) in [
            (Player::Player, CardLocation::Deck, u32::MAX, 1),
            (Player::Player, CardLocation::Deck, 0, 2),
            (Player::Player, CardLocation::Deck, 256, 3),
            (Player::Player, CardLocation::ExtraDeck, 0, 4),
            (Player::Opponent, CardLocation::Deck, 0, 5),
        ] {
            assert_eq!(
                layer
                    .card_at(address(player, location, sequence))
                    .unwrap()
                    .code,
                code
            );
        }
        assert!(
            layer
                .card_at(address(Player::Opponent, CardLocation::Hand, 0))
                .is_none()
        );
    }

    #[test]
    fn duplicate_addresses_and_exhaustion_do_not_change_the_list() {
        let mut layer = CardLayer::default();
        let first = address(Player::Player, CardLocation::Deck, 0);
        let second = address(Player::Player, CardLocation::Deck, 1);
        assert_eq!(layer.insert(first, 42).unwrap(), CardId(0));
        assert!(layer.insert(first, 99).is_err());
        assert_eq!(layer.insert(second, 42).unwrap(), CardId(1));
        layer.next_id = u64::MAX;
        let third = address(Player::Player, CardLocation::Deck, 2);
        assert!(layer.insert(third, 42).is_err());
        assert!(layer.card_at(third).is_none());
        assert_eq!(layer.cards().len(), 2);
    }

    #[test]
    fn new_layer_resets_cards_and_allocation() {
        let first = address(Player::Player, CardLocation::Deck, 0);
        let mut old = CardLayer::default();
        assert_eq!(old.insert(first, 42).unwrap(), CardId(0));
        let mut new = CardLayer::default();
        assert!(new.card_at(first).is_none());
        assert_eq!(new.insert(first, 42).unwrap(), CardId(0));
    }
}
