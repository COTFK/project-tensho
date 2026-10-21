use ocgcore_ffi::types::OCG_CardData;
use std::cell::RefCell;
use std::collections::HashMap;

static EMPTY_SETCODES: [u16; 1] = [0];

std::thread_local! {
    static CARD_DATABASE: RefCell<CardDatabase> = RefCell::new(CardDatabase::new());
}

#[derive(Debug, Clone)]
pub struct CardData {
    pub data: OCG_CardData,
    _setcodes: Box<[u16]>, // owns the buffer that data.setcodes should point to
}

#[derive(Debug, Clone)]
pub struct CardDatabase {
    cards: HashMap<u32, Box<CardData>>,
}

impl CardDatabase {
    pub fn new() -> Self {
        Self {
            cards: HashMap::new(),
        }
    }

    pub fn insert(&mut self, code: u32, mut data: OCG_CardData, setcodes: &[u16]) {
        let mut buf: Box<[u16]> = setcodes.iter().copied().chain(std::iter::once(0)).collect();
        data.setcodes = buf.as_mut_ptr();
        self.cards.insert(
            code,
            Box::new(CardData {
                data,
                _setcodes: buf,
            }),
        );
    }

    pub fn extend(&mut self, other: CardDatabase) {
        self.cards.extend(other.cards);
    }

    pub fn get(&self, code: u32) -> Option<OCG_CardData> {
        self.cards.get(&code).map(|card| card.data)
    }
}

pub fn store_card_data(db: CardDatabase) {
    CARD_DATABASE.with(|cache| cache.borrow_mut().extend(db));
}

pub fn get_card_data(code: u32) -> OCG_CardData {
    CARD_DATABASE
        .with(|cache| cache.borrow().get(code))
        .unwrap_or_else(|| {
            tracing::warn!(code, "engine requested card data that is not cached");
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
