#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct OCGPlayer {
    starting_lp: u32,
    starting_draw_count: u32,
    draw_count_per_turn: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct OCGCardData {
    code: u32,
    alias: u32,
    setcodes: u32,
    type_: u32,
    level: u32,
    attribute: u32,
    race: u64,
    attack: i32,
    defense: i32,
    lscale: u32,
    rscale: u32,
    link_marker: u32,
}

impl Default for OCGCardData {
    fn default() -> Self {
        Self {
            code: 0,
            alias: 0,
            setcodes: 0,
            type_: 0,
            level: 0,
            attribute: 0,
            race: 0,
            attack: 0,
            defense: 0,
            lscale: 0,
            rscale: 0,
            link_marker: 0,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct OCGDuelOptions {
    seed: [u64; 4],
    flags: u64,
    team1: OCGPlayer,
    team2: OCGPlayer,
    pub card_reader: u32,
    payload1: u32,
    pub script_reader: u32,
    payload2: u32,
    pub log_handler: u32,
    payload3: u32,
    pub card_reader_done: u32,
    payload4: u32,
    enable_unsafe_libraries: u8,
    _padding: [u8; 7],
}

impl Default for OCGDuelOptions {
    fn default() -> Self {
        Self {
            seed: [1, 2, 3, 4],
            flags: 0,
            team1: OCGPlayer {
                starting_lp: 8000,
                starting_draw_count: 5,
                draw_count_per_turn: 1,
            },
            team2: OCGPlayer {
                starting_lp: 8000,
                starting_draw_count: 5,
                draw_count_per_turn: 1,
            },
            card_reader: 0,
            payload1: 0,
            script_reader: 0,
            payload2: 0,
            log_handler: 0,
            payload3: 0,
            card_reader_done: 0,
            payload4: 0,
            enable_unsafe_libraries: 0,
            _padding: [0; 7],
        }
    }
}