use crate::board::{Board, Position, Castle};
use crate::pieces::{Piece, Color};
use crate::square::{Square, algebraic_to_square, rank_file_to_square};
use crate::zobrist::zobrist;
use core::result::Result;

pub fn fen_to_board(fen: &str) -> Result<Board, String> {
    let fen_parts: Vec<&str> = fen.split(' ').collect();

    let position = parse_piece_placement(fen_parts[0])?;
    let active_color = parse_active_color(fen_parts[1])?;
    let castling_ability = parse_castling_ability(fen_parts[2])?;
    let en_passant_target = parse_en_passant_target(fen_parts[3])?;
    let halfmove_clock = parse_halfmove_clock(fen_parts[4]);
    let fullmove_counter = parse_fullmove_counter(fen_parts[5]);

    let mut board = Board {
        position,
        active_color,
        castling_ability,
        en_passant_target,
        halfmove_clock,
        fullmove_counter,
        hash: 0,
        pst_mg: 0,
        pst_eg: 0,
        phase: 0,
    };
    board.hash = zobrist().hash(&board);
    board.recompute_eval_terms();
    Ok(board)
}

/*
<Piece Placement> ::= <rank8>'/'<rank7>'/'<rank6>'/'<rank5>'/'<rank4>'/'<rank3>'/'<rank2>'/'<rank1>
<ranki>       ::= [<digit17>]<piece> {[<digit17>]<piece>} [<digit17>] | '8'
<piece>       ::= <white Piece> | <black Piece>
<digit17>     ::= '1' | '2' | '3' | '4' | '5' | '6' | '7'
<white Piece> ::= 'P' | 'N' | 'B' | 'R' | 'Q' | 'K'
<black Piece> ::= 'p' | 'n' | 'b' | 'r' | 'q' | 'k'
 */
fn parse_piece_placement(piece_placement: &str) -> Result<Position, String> {
    let mut position = Position::new();
    let pieces_placement_rank: Vec<&str> = piece_placement.split('/').collect();

    if pieces_placement_rank.len() != 8 {
        return Err("Invalid number of ranks in FEN piece placement".to_string())
    }

    for (idx, values) in pieces_placement_rank.iter().enumerate() {
        let rank = 7 - idx as u8;
        let mut file = 0;
        for c in values.chars() {
            let square = rank_file_to_square(rank, file);
            match c {
                'p' | 'n' | 'b' | 'r' | 'q' | 'k' | 
                'P' | 'N' | 'B' | 'R' | 'Q' | 'K' => {
                    position.add_piece(char_to_color(c), char_to_piece(c), square);
                    file += 1;
                } 
                '1'..='8' => file += c.to_digit(10).unwrap() as u8,
                _ => return Err("Invalid character in FEN piece placement".to_string())
            };
        }
    }
    Ok(position)
}

/*
<Side to move> ::= {'w' | 'b'}
 */
fn parse_active_color(active_color: &str) -> Result<Color, String> {
    let c = active_color.chars().next().unwrap();
    match c {
        'w' => Ok(Color::White),
        'b' => Ok(Color::Black),
        _ => Err("Invalid character in FEN active color".to_string())
    }
}

/*
<Castling ability> ::= '-' | ['K'] ['Q'] ['k'] ['q'] (1..4)
 */
fn parse_castling_ability(castling_ability: &str) -> Result<Castle, String> {
    if castling_ability.chars().count() > 4 {
        return Err("Invalid number of castling characters in FEN".to_string())
    }

    // Rights will be off in the event of '-' and set on accordingly
    let mut castle_rights = Castle::new(false, false, false, false);
    for c in castling_ability.chars() {
        castle_rights.set(c, true);
    }
    Ok(castle_rights)
}

/*
<En passant target square> ::= '-' | <epsquare>
<epsquare>   ::= <fileLetter> <eprank>
<fileLetter> ::= 'a' | 'b' | 'c' | 'd' | 'e' | 'f' | 'g' | 'h'
<eprank>     ::= '3' | '6'
 */
fn parse_en_passant_target(en_passant_target: &str) -> Result<Option<Square>, String> {
    if en_passant_target.chars().count() > 2 {
        return Err("Invalid number of en passant target characters in FEN".to_string())
    }

    match en_passant_target.chars().next().unwrap() {
        '-' => Ok(None),
        _ => Ok(Some(algebraic_to_square(&en_passant_target[0..2])))
    }
}

/*
<Halfmove Clock> ::= <digit> {<digit>}
<digit> ::= '0' | '1' | '2' | '3' | '4' | '5' | '6' | '7' | '8' | '9'
 */
fn parse_halfmove_clock(halfmove_clock: &str) -> u8 {
    halfmove_clock.parse().expect("Failed to parse halfmove clock from FEN")
}

/*
<Fullmove counter> ::= <digit19> {<digit>}
<digit19> ::= '1' | '2' | '3' | '4' | '5' | '6' | '7' | '8' | '9'
<digit>   ::= '0' | <digit19>
 */
fn parse_fullmove_counter(fullmove_counter: &str) -> u8 {
    fullmove_counter.parse().expect("Failed to parse fullmove counter from FEN")
}

pub fn char_to_piece(c: char) -> Piece {
    match c.to_ascii_lowercase() {
        'p' => Piece::Pawn,
        'n' => Piece::Knight,
        'b' => Piece::Bishop,
        'r' => Piece::Rook,
        'q' => Piece::Queen,
        'k' => Piece::King,
        _ => panic!("Invalid piece character provided by FEN")
    }
}

pub fn char_to_color(c: char) -> Color {
    if c.is_lowercase() {
        Color::Black
    } else {
        Color::White
    }
}