#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct OCGPlayer {
    starting_lp: u32,
    starting_draw_count: u32,
    draw_count_per_turn: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct OCGCardData {
    pub code: u32,
    pub alias: u32,
    pub setcodes: u32,  // This will be a pointer written separately
    pub type_: u32,
    pub level: u32,
    pub attribute: u32,
    pub race: u64,
    pub attack: i32,
    pub defense: i32,
    pub lscale: u32,
    pub rscale: u32,
    pub link_marker: u32,
}

impl OCGCardData {
    pub fn with_code(code: u32) -> Self {
        Self {
            code,
            ..Self::default()
        }
    }

    pub fn write_bytes(&self, out: &mut [u8]) {
        assert!(out.len() >= std::mem::size_of::<Self>());

        out[0..4].copy_from_slice(&self.code.to_le_bytes());
        out[4..8].copy_from_slice(&self.alias.to_le_bytes());
        out[8..12].copy_from_slice(&self.setcodes.to_le_bytes());
        out[12..16].copy_from_slice(&self.type_.to_le_bytes());
        out[16..20].copy_from_slice(&self.level.to_le_bytes());
        out[20..24].copy_from_slice(&self.attribute.to_le_bytes());
        out[24..32].copy_from_slice(&self.race.to_le_bytes());
        out[32..36].copy_from_slice(&self.attack.to_le_bytes());
        out[36..40].copy_from_slice(&self.defense.to_le_bytes());
        out[40..44].copy_from_slice(&self.lscale.to_le_bytes());
        out[44..48].copy_from_slice(&self.rscale.to_le_bytes());
        out[48..52].copy_from_slice(&self.link_marker.to_le_bytes());
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
            seed: [
                rand::random(),
                rand::random(),
                rand::random(),
                rand::random(),
            ],
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
