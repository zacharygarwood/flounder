use crate::bitboard::{FILES, RANKS};
use crate::board::Board;
use crate::pieces::{Color, Piece};

#[allow(dead_code)]
pub fn print_bitboard(num: u64) {
    for rank in (0..RANKS).rev() {
        print!(" {} ", rank + 1);
        for file in 0..FILES {
            let square = rank * 8 + file;
            let bit = (num >> square) & 1;
            print!(" {} ", bit);
        }
        println!();
    }
    println!("    a  b  c  d  e  f  g  h");
    println!();
}

#[allow(dead_code)]
pub fn print_board(board: &Board) {
    for rank in (0..RANKS).rev() {
        print!(" {} ", rank + 1);
        for file in 0..FILES {
            let square = rank * 8 + file;
            let piece = board.get_piece_at(square);
            let color = board.get_color_at(square);
            match piece {
                Some(Piece::Pawn) => match color {
                    Some(Color::White) => print!(" P "),
                    Some(Color::Black) => print!(" p "),
                    None => {}
                },
                Some(Piece::Knight) => match color {
                    Some(Color::White) => print!(" N "),
                    Some(Color::Black) => print!(" n "),
                    None => {}
                },
                Some(Piece::Bishop) => match color {
                    Some(Color::White) => print!(" B "),
                    Some(Color::Black) => print!(" b "),
                    None => {}
                },
                Some(Piece::Rook) => match color {
                    Some(Color::White) => print!(" R "),
                    Some(Color::Black) => print!(" r "),
                    None => {}
                },
                Some(Piece::Queen) => match color {
                    Some(Color::White) => print!(" Q "),
                    Some(Color::Black) => print!(" q "),
                    None => {}
                },
                Some(Piece::King) => match color {
                    Some(Color::White) => print!(" K "),
                    Some(Color::Black) => print!(" k "),
                    None => {}
                },
                None => print!(" . "),
            }
        }
        println!();
    }
    println!("    a  b  c  d  e  f  g  h");
    println!();
}
