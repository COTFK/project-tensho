use super::card_text::CardText;
use ocgcore_ffi::types::OCG_CardData;
use std::cell::RefCell;
use std::collections::HashMap;

static EMPTY_SETCODES: [u16; 1] = [0];

std::thread_local! {
    static CARD_CATALOG: RefCell<CardCatalog> = RefCell::new(CardCatalog::new());
}

#[derive(Debug, Clone)]
pub struct CardDefinition {
    pub data: OCG_CardData,
    pub text: CardText,
    _setcodes: Box<[u16]>, // owns the buffer that data.setcodes should point to
}

#[derive(Debug, Clone)]
pub struct CardCatalog {
    cards: HashMap<u32, Box<CardDefinition>>,
}

impl CardCatalog {
    pub fn new() -> Self {
        Self {
            cards: HashMap::new(),
        }
    }

    pub fn insert(&mut self, code: u32, mut data: OCG_CardData, setcodes: &[u16], text: CardText) {
        let mut buf: Box<[u16]> = setcodes.iter().copied().chain(std::iter::once(0)).collect();
        data.setcodes = buf.as_mut_ptr();
        self.cards.insert(
            code,
            Box::new(CardDefinition {
                data,
                text,
                _setcodes: buf,
            }),
        );
    }

    pub fn extend(&mut self, other: CardCatalog) {
        self.cards.extend(other.cards);
    }

    pub fn get(&self, code: u32) -> Option<&CardDefinition> {
        self.cards.get(&code).map(|card| card.as_ref())
    }
}

pub fn store_card_definitions(db: CardCatalog) {
    CARD_CATALOG.with(|cache| cache.borrow_mut().extend(db));
}

pub fn get_card_text(code: u32) -> Option<CardText> {
    CARD_CATALOG.with(|cache| cache.borrow().get(code).map(|card| card.text.clone()))
}

pub fn read_card_definition(code: u32) -> OCG_CardData {
    CARD_CATALOG
        .with(|cache| cache.borrow().get(code).map(|card| card.data))
        .unwrap_or_else(|| {
            tracing::error!(code, "engine requested card data that is not cached");
            OCG_CardData {
                code,
                alias: 0,
                setcodes: EMPTY_SETCODES.as_ptr() as *mut u16,
                r#type: 0,
                level: 0,
                attribute: 0,
                race: 0,
                attack: 0,
                defense: 0,
                lscale: 0,
                rscale: 0,
                link_marker: 0,
            }
        })
}
