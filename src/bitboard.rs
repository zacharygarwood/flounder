use crate::moves::{EAST, NORTH, SOUTH, WEST};
use crate::square::{rank_file_to_square, Square};

pub type Bitboard = u64;

#[allow(dead_code)]
pub const FILES: u8 = 8;
#[allow(dead_code)]
pub const RANKS: u8 = 8;
pub const SQUARES: Square = 64;

pub const RANK_1: Bitboard = 0x00000000000000FF;
pub const RANK_2: Bitboard = RANK_1 << 8;
pub const RANK_3: Bitboard = RANK_2 << 8;
pub const RANK_4: Bitboard = RANK_3 << 8;
pub const RANK_5: Bitboard = RANK_4 << 8;
pub const RANK_6: Bitboard = RANK_5 << 8;
pub const RANK_7: Bitboard = RANK_6 << 8;
pub const RANK_8: Bitboard = RANK_7 << 8;

pub const FILE_A: Bitboard = 0x0101010101010101;
pub const FILE_B: Bitboard = FILE_A << 1;
pub const FILE_C: Bitboard = FILE_B << 1;
pub const FILE_D: Bitboard = FILE_C << 1;
pub const FILE_E: Bitboard = FILE_D << 1;
pub const FILE_F: Bitboard = FILE_E << 1;
pub const FILE_G: Bitboard = FILE_F << 1;
pub const FILE_H: Bitboard = FILE_G << 1;

pub const WHITE_KING_SIDE: Bitboard = 0x0000000000000060;
pub const WHITE_QUEEN_SIDE: Bitboard = 0x000000000000000E;
pub const BLACK_KING_SIDE: Bitboard = 0x6000000000000000;
pub const BLACK_QUEEN_SIDE: Bitboard = 0x0E00000000000000;

pub trait BitboardOperations {
    fn shift(&self, n: i8) -> Bitboard;
    fn set_bit(&mut self, square: Square);
    fn remove_bit(&mut self, square: Square);
    fn empty() -> Bitboard;
    fn rank_file_to_bitboard(rank: u8, file: u8) -> Bitboard;
    fn square_to_bitboard(square: Square) -> Bitboard;
    fn rank_file_to_edge_mask(rank: u8, file: u8) -> Bitboard;
}

impl BitboardOperations for Bitboard {
    // Performs shifting used by non-sliding pieces
    fn shift(&self, dir: i8) -> Bitboard {
        if dir == NORTH {
            shift_left(*self, 8)
        } else if dir == SOUTH {
            shift_right(*self, 8)
        } else if dir == EAST {
            shift_left(*self & !FILE_H, 1)
        } else if dir == WEST {
            shift_right(*self & !FILE_A, 1)
        } else if dir == NORTH + EAST {
            shift_left(*self & !FILE_H, 9)
        } else if dir == NORTH + WEST {
            shift_left(*self & !FILE_A, 7)
        } else if dir == SOUTH + EAST {
            shift_right(*self & !FILE_H, 7)
        } else if dir == SOUTH + WEST {
            shift_right(*self & !FILE_A, 9)
        } else if dir == NORTH + NORTH + EAST {
            shift_left(*self & !FILE_H, 17)
        } else if dir == NORTH + NORTH + WEST {
            shift_left(*self & !FILE_A, 15)
        } else if dir == SOUTH + SOUTH + EAST {
            shift_right(*self & !FILE_H, 15)
        } else if dir == SOUTH + SOUTH + WEST {
            shift_right(*self & !FILE_A, 17)
        } else if dir == NORTH + EAST + EAST {
            shift_left(*self & !(FILE_G | FILE_H), 10)
        } else if dir == NORTH + WEST + WEST {
            shift_left(*self & !(FILE_A | FILE_B), 6)
        } else if dir == SOUTH + EAST + EAST {
            shift_right(*self & !(FILE_G | FILE_H), 6)
        } else if dir == SOUTH + WEST + WEST {
            shift_right(*self & !(FILE_A | FILE_B), 10)
        } else if dir > 0 {
            shift_left(*self, dir as u8)
        } else {
            shift_right(*self, -dir as u8)
        }
    }

    // Set a bit on an existing Bitboard
    fn set_bit(&mut self, square: Square) {
        *self |= 1 << square;
    }

    // Set a bit on an existing Bitboard
    fn remove_bit(&mut self, square: Square) {
        *self &= !(1 << square);
    }

    // Create an empty Bitboard
    fn empty() -> Bitboard {
        0
    }

    // Creates a bitboard with one bit set at rank, file
    fn rank_file_to_bitboard(rank: u8, file: u8) -> Bitboard {
        1 << rank_file_to_square(rank, file)
    }

    // Creates a bitboard with one bit set at rank, file
    fn square_to_bitboard(square: u8) -> Bitboard {
        1 << square
    }

    // Creates a bitboard with the edges opposite of rank, file set to 1
    // Used in generating attack masks as the edges do not need to be set
    fn rank_file_to_edge_mask(rank: u8, file: u8) -> Bitboard {
        let mut mask = Bitboard::empty();

        mask |= match rank {
            0 => RANK_8,
            7 => RANK_1,
            _ => RANK_1 | RANK_8,
        };

        mask |= match file {
            0 => FILE_H,
            7 => FILE_A,
            _ => FILE_A | FILE_H,
        };

        mask
    }
}

fn shift_left(bb: Bitboard, i: u8) -> u64 {
    bb.checked_shl(u32::from(i)).unwrap_or(0)
}

fn shift_right(bb: Bitboard, i: u8) -> Bitboard {
    bb.checked_shr(u32::from(i)).unwrap_or(0)
}

pub struct BitboardIterator {
    bitboard: Bitboard,
}

impl BitboardIterator {
    pub fn new(bitboard: Bitboard) -> Self {
        BitboardIterator { bitboard }
    }
}

// Iterates through each 1 bit in the bitboard
impl Iterator for BitboardIterator {
    type Item = Square;

    fn next(&mut self) -> Option<Self::Item> {
        if self.bitboard == 0 {
            return None;
        }

        let least_significant_bit = self.bitboard & (!self.bitboard + 1);
        let square = least_significant_bit.trailing_zeros() as u8;

        self.bitboard ^= least_significant_bit;

        Some(square)
    }
}

