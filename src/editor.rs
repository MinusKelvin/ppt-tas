use crate::gameplay::*;

#[derive(Debug)]
pub struct TasEditor {
    seed: u32,
    frames: Vec<Frame>,
    pieces: Vec<Piece>,
    generator: PieceGenerator
}

impl TasEditor {
    pub fn new(seed: u32) -> Self {
        TasEditor {
            seed,
            frames: vec![],
            pieces: vec![],
            generator: PieceGenerator::new(seed)
        }
    }
}

bitflags::bitflags! {
    struct Input : u8 {
        const LEFT = 0x01;
        const RIGHT = 0x02;
        const HARD_DROP = 0x04;
        const SOFT_DROP = 0x08;
        const ROTATE_LEFT = 0x10;
        const ROTATE_RIGHT = 0x20;
        const HOLD = 0x40;
    }
}

#[derive(Clone, Debug)]
struct Frame {
    input: Input,
    state: Option<State>
}
