use crate::{pieces::Piece, square::square_to_algebraic};

pub const NORTH: i8 = 8;
pub const EAST: i8 = 1;
pub const SOUTH: i8 = -NORTH;
pub const WEST: i8 = -EAST;

#[derive(PartialEq, Eq, Copy, Clone, Debug)]
pub struct Move {
    pub to: u8,
    pub from: u8,
    pub piece_type: Piece,
    pub move_type: MoveType,
}

impl Move {
    pub fn new(from: u8, to: u8, piece_type: Piece, move_type: MoveType) -> Self {
        Self {
            to,
            from,
            piece_type,
            move_type,
        }
    }

    pub fn print(&self) {
        print!("{}", self.to_algebraic());
    }

    pub fn to_algebraic(self) -> String {
        let mut promotion_piece_char = "";
        if self.move_type == MoveType::Promotion {
            promotion_piece_char = match self.piece_type {
                Piece::Bishop => "b",
                Piece::Knight => "n",
                Piece::Rook => "r",
                Piece::Queen => "q",
                _ => "",
            }
        }
        let algebreaic = format!(
            "{}{}{}",
            square_to_algebraic(self.from),
            square_to_algebraic(self.to),
            promotion_piece_char
        );
        algebreaic
    }
}

#[derive(PartialEq, Eq, Copy, Clone, Debug)]
pub enum MoveType {
    Quiet,   // Non-capturing move
    Capture, // Capturing move
    EnPassant,
    Castle,
    Promotion,
}

impl std::fmt::Display for MoveType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let piece_str = match self {
            MoveType::Quiet => "Quiet",
            MoveType::Capture => "Capture",
            MoveType::EnPassant => "En Passant",
            MoveType::Castle => "Castle",
            MoveType::Promotion => "Promotion",
        };
        write!(f, "{}", piece_str)
    }
}

