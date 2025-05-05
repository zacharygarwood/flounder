use std::ops::{Index, IndexMut, Not};
use crate::bitboard::Bitboard;


pub const COLOR_COUNT: usize = 2;
pub const PIECE_COUNT: usize = 6;

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Piece {
    Pawn,
    Knight,
    Bishop,
    Rook,
    Queen,
    King,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Color {
    White,
    Black,
}

impl Color {
    pub fn index(&self) -> usize {
        match self {
            Color::White => 0,
            Color::Black => 1,
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub struct ColorIterator {
    index: usize,
}

impl ColorIterator {
    pub fn new() -> Self {
        ColorIterator { index: 0 }
    }
}

impl Iterator for ColorIterator {
    type Item = Color;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index < 2 {
            let color = match self.index {
                0 => Color::White,
                1 => Color::Black,
                _ => unreachable!(),
            };
            self.index += 1;
            Some(color)
        } else {
            None
        }
    }
}

impl std::fmt::Display for Piece {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let piece_str = match self {
            Piece::Pawn => "Pawn",
            Piece::Knight => "Knight",
            Piece::Bishop => "Bishop",
            Piece::Rook => "Rook",
            Piece::Queen => "Queen",
            Piece::King => "King",
        };
        write!(f, "{}", piece_str)
    }
}

impl std::fmt::Display for Color {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let piece_str = match self {
            Color::White => "White",
            Color::Black => "Black",
        };
        write!(f, "{}", piece_str)
    }
}

impl Piece {
    pub fn index(&self) -> usize {
        match self {
            Piece::Pawn => 0,
            Piece::Knight => 1,
            Piece::Bishop => 2,
            Piece::Rook => 3,
            Piece::Queen => 4,
            Piece::King => 5,
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub struct PieceIterator {
    index: usize,
}

impl PieceIterator {
    pub fn new() -> Self {
        PieceIterator { index: 0 }
    }
}

impl Iterator for PieceIterator {
    type Item = Piece;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index < 6 {
            let piece = match self.index {
                0 => Piece::Pawn,
                1 => Piece::Knight,
                2 => Piece::Bishop,
                3 => Piece::Rook,
                4 => Piece::Queen,
                5 => Piece::King,
                _ => unreachable!(),
            };
            self.index += 1;
            Some(piece)
        } else {
            None
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub struct PromotionPieceIterator {
    current_piece: Piece,
}

impl PromotionPieceIterator {
    pub fn new() -> Self {
        PromotionPieceIterator {
            current_piece: Piece::Pawn,
        }
    }
}

impl Iterator for PromotionPieceIterator {
    type Item = Piece;

    fn next(&mut self) -> Option<Self::Item> {
        let next_piece = match self.current_piece {
            Piece::Pawn => Piece::Knight,
            Piece::Knight => Piece::Bishop,
            Piece::Bishop => Piece::Rook,
            Piece::Rook => Piece::Queen,
            Piece::Queen => return None,
            _ => unreachable!(),
        };

        self.current_piece = next_piece;
        Some(next_piece)
    }
}

impl Index<Piece> for [Bitboard; PIECE_COUNT] {
    type Output = Bitboard;

    fn index(&self, piece: Piece) -> &Self::Output {
        match piece {
            Piece::Pawn => &self[0],
            Piece::Knight => &self[1],
            Piece::Bishop => &self[2],
            Piece::Rook => &self[3],
            Piece::Queen => &self[4],
            Piece::King => &self[5],
        }
    }
}

impl IndexMut<Piece> for [Bitboard; PIECE_COUNT] {
    fn index_mut(&mut self, piece: Piece) -> &mut Self::Output {
        match piece {
            Piece::Pawn => &mut self[0],
            Piece::Knight => &mut self[1],
            Piece::Bishop => &mut self[2],
            Piece::Rook => &mut self[3],
            Piece::Queen => &mut self[4],
            Piece::King => &mut self[5],
        }
    }
}


impl Index<Color> for [Bitboard; COLOR_COUNT] {
    type Output = Bitboard;

    fn index(&self, color: Color) -> &Self::Output {
        match color {
            Color::White => &self[0],
            Color::Black => &self[1],
        }
    }
}

impl IndexMut<Color> for [Bitboard; COLOR_COUNT] {
    fn index_mut(&mut self, color: Color) -> &mut Self::Output {
        match color {
            Color::White => &mut self[0],
            Color::Black => &mut self[1],
        }
    }
}

impl Color {
    pub fn index(&self) -> usize {
        match self {
            Color::White => 0,
            Color::Black => 1,
        }
    }
}

impl std::ops::Not for Color {
    type Output = Color;

    fn not(self) -> Self::Output {
        match self {
            Color::White => Color::Black,
            Color::Black => Color::White,
        }
    }
}
