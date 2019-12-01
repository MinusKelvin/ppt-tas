mod piece;

pub use piece::*;

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct State {
    field: Vec<[Option<Piece>; 10]>,
    hold: Option<Piece>,
    hold_is_high: bool,
    state: GameState,
    time: u32,
    score: u32,
    lines: u32,
    left_das: u32,
    right_das: u32,
    line_clear_points: Option<(u32, u32)>,
    pc_points: Option<(u32, u32)>,
    piece_generator: PieceGenerator
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
enum GameState {
    SpawnDelay(u32),
    LineClearDelay(u32),
    FallingPiece(FallingPiece)
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
struct FallingPiece {
    piece: PieceState,
    gravity: u32,
    lock_delay: u32,
    last_frame_was_movement: bool,
    lowest_y: i32,
    rotations: u32,
    rotations_movements: u32,
}

impl State {
    pub fn new(seed: u32) -> State {
        State {
            field: vec![[None; 10]; 40],
            hold: None,
            hold_is_high: false,
            state: GameState::SpawnDelay(5),
            time: 0,
            score: 0,
            lines: 0,
            left_das: 0,
            right_das: 0,
            line_clear_points: None,
            pc_points: None,
            piece_generator: PieceGenerator::new(seed)
        }
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct PieceGenerator {
    rng: u32,
    index: u8,
    bag: [Piece; 7]
}

impl PieceGenerator {
    pub fn new(seed: u32) -> Self {
        let mut seed = seed;
        for _ in 0..1987 {
            seed = roll(seed);
        }
        PieceGenerator {
            rng: seed,
            index: 7,
            bag: [Piece::S; 7]
        }
    }

    pub fn next(&mut self) -> Piece {
        if self.index == 7 {
            use Piece::*;
            self.index = 0;
            self.bag = [S, Z, J, L, T, O, I];
        }

        self.rng = roll(self.rng);
        let index = ((self.rng >> 16) * (7 - self.index as u32)) >> 16 + self.index as u32;
        self.bag.swap(self.index as usize, index as usize);
        let piece = self.bag[self.index as usize];
        self.index += 1;
        piece
    }
}

fn roll(rng: u32) -> u32 {
    rng.overflowing_mul(0x5D588B65).0.overflowing_add(0x269EC3).0
}