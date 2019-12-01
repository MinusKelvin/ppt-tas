

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum Piece {
    S = 0,
    Z = 1,
    J = 2,
    L = 3,
    T = 4,
    O = 5,
    I = 6
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum RotationState {
    North = 0,
    East = 1,
    South = 2,
    West = 3
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct PieceState {
    pub kind: Piece,
    pub state: RotationState,
    pub x: i32,
    pub y: i32,
}

impl PieceState {
    pub fn cells(&self) -> [(i32, i32); 4] {
        let mut cells = PIECE_SHAPES[self.index()];
        for i in 0..4 {
            cells[i].0 += self.x;
            cells[i].1 += self.y;
        }
        cells
    }

    pub fn cw(&mut self, field: &[[Option<Piece>; 10]]) {
        self.rotate(field, match self.state {
            RotationState::North => RotationState::East,
            RotationState::East => RotationState::South,
            RotationState::South => RotationState::West,
            RotationState::West => RotationState::North
        })
    }

    pub fn ccw(&mut self, field: &[[Option<Piece>; 10]]) {
        self.rotate(field, match self.state {
            RotationState::North => RotationState::West,
            RotationState::West => RotationState::South,
            RotationState::South => RotationState::East,
            RotationState::East => RotationState::North
        })
    }

    fn rotate(&mut self, field: &[[Option<Piece>; 10]], to: RotationState) {
        let mut result = *self;
        result.state = to;
        let kicks = PIECE_OFFSETS[self.index()].iter()
            .zip(PIECE_OFFSETS[result.index()].iter())
            .map(|(&(x1, y1), &(x2, y2))| (x1 - x2, y1 - y2));
        for (x, y) in kicks {
            result.x = self.x + x;
            result.y = self.y + y;
            if !result.overlaps(field) {
                *self = result;
                return
            }
        }
    }

    fn index(&self) -> usize {
        self.kind as usize * 4 + self.state as usize
    }

    pub fn overlaps(&self, field: &[[Option<Piece>; 10]]) -> bool {
        for &(x, y) in &self.cells() {
            if y < 0 || y >= 40 || x < 0 || x >= 10 || field[y as usize][x as usize].is_some() {
                return true
            }
        }
        false
    }
}

const PIECE_SHAPES: [[(i32, i32); 4]; 28] = [
    // S
    [(-1, 0), (0, 0), (0, 1), (1, 1)],
    [(0, 1), (0, 0), (1, 0), (1, -1)],
    [(1, 0), (0, 0), (0, -1), (-1, -1)],
    [(0, -1), (0, 0), (-1, 0), (-1, 1)],
    // Z
    [(1, 0), (0, 0), (0, 1), (-1, 1)],
    [(0, -1), (0, 0), (1, 0), (1, 1)],
    [(-1, 0), (0, 0), (0, -1), (1, -1)],
    [(0, 1), (0, 0), (-1, 0), (-1, -1)],
    // J
    [(-1, 1), (-1, 0), (0, 0), (1, 0)],
    [(1, 1), (0, 1), (0, 0), (0, -1)],
    [(1, -1), (1, 0), (0, 0), (-1, 0)],
    [(-1, -1), (0, -1), (0, 0), (0, 1)],
    // L
    [(-1, 0), (0, 0), (1, 0), (1, 1)],
    [(0, 1), (0, 0), (0, -1), (1, -1)],
    [(1, 0), (0, 0), (-1, 0), (-1, -1)],
    [(0, -1), (0, 0), (0, 1), (-1, 1)],
    // T
    [(0, 0), (-1, 0), (0, 1), (1, 0)],
    [(0, 0), (0, 1), (1, 0), (0, -1)],
    [(0, 0), (1, 0), (0, -1), (-1, 0)],
    [(0, 0), (0, -1), (-1, 0), (0, 1)],
    // O
    [(0, 0), (1, 0), (0, 1), (1, 1)],
    [(0, 0), (0, -1), (1, 0), (1, -1)],
    [(0, 0), (-1, 0), (0, -1), (-1, -1)],
    [(0, 0), (0, 1), (-1, 0), (-1, 1)],
    // I
    [(-1, 0), (0, 0), (1, 0), (2, 0)],
    [(0, 1), (0, 0), (0, -1), (0, -2)],
    [(1, 0), (0, 0), (-1, 0), (-2, 0)],
    [(0, -1), (0, 0), (0, 1), (0, 2)]
];

const PIECE_OFFSETS: [[(i32, i32); 5]; 28] = [
    // S
    [(0, 0); 5],
    [(0, 0), (1, 0), (1, -1), (0, 2), (1, 2)],
    [(0, 0); 5],
    [(0, 0), (-1, 0), (-1, -1), (0, 2), (-1, 2)],
    // Z
    [(0, 0); 5],
    [(0, 0), (1, 0), (1, -1), (0, 2), (1, 2)],
    [(0, 0); 5],
    [(0, 0), (-1, 0), (-1, -1), (0, 2), (-1, 2)],
    // J
    [(0, 0); 5],
    [(0, 0), (1, 0), (1, -1), (0, 2), (1, 2)],
    [(0, 0); 5],
    [(0, 0), (-1, 0), (-1, -1), (0, 2), (-1, 2)],
    // L
    [(0, 0); 5],
    [(0, 0), (1, 0), (1, -1), (0, 2), (1, 2)],
    [(0, 0); 5],
    [(0, 0), (-1, 0), (-1, -1), (0, 2), (-1, 2)],
    // T
    [(0, 0); 5],
    [(0, 0), (1, 0), (1, -1), (0, 2), (1, 2)],
    [(0, 0); 5],
    [(0, 0), (-1, 0), (-1, -1), (0, 2), (-1, 2)],
    // O
    [(0, 0); 5],
    [(0, -1); 5],
    [(-1, -1); 5],
    [(-1, 0); 5],
    // I
    [(0, 0), (-1, 0), (2, 0), (-1, 0), (2, 0)],
    [(-1, 0), (0, 0), (0, 0), (0, 1), (0, -2)],
    [(-1, 1), (1, 1), (-2, 1), (1, 0), (-2, 0)],
    [(0, 1), (0, 1), (0, 1), (0, -1), (0, 2)]
];