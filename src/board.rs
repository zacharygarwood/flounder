use crate::bitboard::{Bitboard, BitboardOperations};
use crate::fen::fen_to_board;
use crate::moves::{Move, MoveType};
use crate::pieces::{Color, ColorIterator, Piece, PieceIterator, COLOR_COUNT, PIECE_COUNT};
use crate::square::{Square, A1, A8, D1, D8, F1, F8, G1, G8, H1, H8};

// Represents the chess board using bitboards
#[derive(Copy, Clone)]
pub struct Board {
    pub position: Position,
    pub active_color: Color,
    pub castling_ability: Castle,
    pub en_passant_target: Option<Square>,

    #[allow(dead_code)]
    pub halfmove_clock: u8,
    #[allow(dead_code)]
    pub fullmove_counter: u8,
}

impl Board {
    pub fn new(fen: &str) -> Self {
        match fen_to_board(fen) {
            Ok(board) => board,
            Err(err) => {
                println!("Error constructing FEN: {}", err);
                println!("Setting board to default values");
                Self::default()
            }
        }
    }

    // Creates the default board state
    pub fn default() -> Self {
        Self {
            position: Position::default(),
            active_color: Color::White,
            castling_ability: Castle::new(true, true, true, true),
            en_passant_target: None,
            halfmove_clock: 0,
            fullmove_counter: 1,
        }
    }

    // Returns select pieces of a certain color e.g. white pawns
    pub fn bb(&self, color: Color, piece: Piece) -> Bitboard {
        self.position.bb(color, piece)
    }

    // Returns all pieces of a certain color e.g. white pieces
    pub fn bb_color(&self, color: Color) -> Bitboard {
        self.position.bb_color(color)
    }

    // Returns all pieces of a select type e.g. pawns
    pub fn bb_piece(&self, piece: Piece) -> Bitboard {
        self.position.bb_piece(piece)
    }

    // Returns the color of the player to play
    pub fn active_color(&self) -> Color {
        self.active_color
    }

    pub fn castling_ability(&self, color: Color) -> (bool, bool) {
        match color {
            Color::White => (
                self.castling_ability.white_king,
                self.castling_ability.white_queen,
            ),
            Color::Black => (
                self.castling_ability.black_king,
                self.castling_ability.black_queen,
            ),
        }
    }

    // Bitboard of all empty spaces
    pub fn bb_empty(&self) -> Bitboard {
        !(self.bb_color(Color::White) | self.bb_color(Color::Black))
    }

    // Bitboard of all pieces
    pub fn bb_all(&self) -> Bitboard {
        self.bb_color(Color::White) | self.bb_color(Color::Black)
    }

    pub fn add_piece(&mut self, color: Color, piece: Piece, square: Square) {
        self.position.add_piece(color, piece, square);
    }

    pub fn remove_piece(&mut self, color: Color, piece: Piece, square: Square) {
        self.position.remove_piece(color, piece, square);
    }

    pub fn clone_with_move(&self, mv: &Move) -> Board {
        let mut board = *self;
        board.make_move(mv);
        board
    }

    pub fn change_color(&mut self) {
        self.active_color = !self.active_color;
    }

    pub fn make_move(&mut self, mv: &Move) {
        self.reset_en_passant_target();
        self.change_castling_rights(mv);

        match mv.move_type {
            MoveType::Quiet => self.make_quiet(mv),
            MoveType::Capture => self.make_capture(mv),
            MoveType::EnPassant => self.make_en_passant(mv),
            MoveType::Castle => self.make_castle(mv),
            MoveType::Promotion => self.make_promotion(mv),
        }

        self.change_color();
    }

    fn reset_en_passant_target(&mut self) {
        self.en_passant_target = None;
    }

    fn change_castling_rights(&mut self, mv: &Move) {
        let color = self.active_color;

        // Any king move (includes castling) removes all rights for that player
        if mv.piece_type == Piece::King {
            self.castling_ability.remove_all_rights(color);
        }

        // Moving the rook when the rights for its side are set removes them
        if mv.piece_type == Piece::Rook {
            let (king_side_rights, queen_side_rights) = self.castling_ability(color);
            let (rook_on_king_side, rook_on_queen_side) = match color {
                Color::White => (mv.from == H1, mv.from == A1),
                Color::Black => (mv.from == H8, mv.from == A8),
            };

            if rook_on_king_side && king_side_rights {
                self.castling_ability.remove_side_rights(color, Piece::King);
            }

            if rook_on_queen_side && queen_side_rights {
                self.castling_ability
                    .remove_side_rights(color, Piece::Queen);
            }
        }

        // Capturing a rook when its rights are set removes them
        if mv.move_type == MoveType::Capture {
            // A capture must be going to another piece, so this will never be None
            let captured_piece = self.get_piece_at(mv.to).unwrap();

            if captured_piece == Piece::Rook {
                let (king_side_rights, queen_side_rights) = self.castling_ability(!color);
                let (rook_on_king_side, rook_on_queen_side) = match color {
                    Color::White => (mv.to == H8, mv.to == A8),
                    Color::Black => (mv.to == H1, mv.to == A1),
                };

                if rook_on_king_side && king_side_rights {
                    self.castling_ability
                        .remove_side_rights(!color, Piece::King);
                }

                if rook_on_queen_side && queen_side_rights {
                    self.castling_ability
                        .remove_side_rights(!color, Piece::Queen);
                }
            }
        }

        // A promotion could also capture a rook so the rights need to be changed
        if mv.move_type == MoveType::Promotion {
            // A promotion could be a push of a pawn and not a capture, so this can be None
            if let Some(Piece::Rook) = self.get_piece_at(mv.to) {
                let (king_side_rights, queen_side_rights) = self.castling_ability(!color);
                let (rook_on_king_side, rook_on_queen_side) = match color {
                    Color::White => (mv.to == H8, mv.to == A8),
                    Color::Black => (mv.to == H1, mv.to == A1),
                };

                if rook_on_king_side && king_side_rights {
                    self.castling_ability
                        .remove_side_rights(!color, Piece::King);
                }

                if rook_on_queen_side && queen_side_rights {
                    self.castling_ability
                        .remove_side_rights(!color, Piece::Queen);
                }
            }
        }
    }

    fn make_quiet(&mut self, mv: &Move) {
        let color = self.active_color;

        let offset = match color {
            Color::White => 8,
            Color::Black => -8,
        };

        // Pawn could be pushed twice adding en passant target
        if self.is_double_pawn_push(mv) {
            self.en_passant_target = Some((mv.from as i8 + offset) as u8);
        }

        self.remove_piece(color, mv.piece_type, mv.from);
        self.add_piece(color, mv.piece_type, mv.to);
    }

    fn make_capture(&mut self, mv: &Move) {
        let color = self.active_color;
        let captured_piece = self.get_piece_at(mv.to).unwrap(); // Should never be None as the move legality is performed in move_gen.rs

        self.remove_piece(!color, captured_piece, mv.to);
        self.remove_piece(color, mv.piece_type, mv.from);
        self.add_piece(color, mv.piece_type, mv.to);
    }

    fn make_en_passant(&mut self, mv: &Move) {
        let color = self.active_color;
        let offset = match color {
            Color::White => 8,
            Color::Black => -8,
        };

        let captured_square = (mv.to as i8 - offset) as u8;

        self.remove_piece(!color, Piece::Pawn, captured_square);
        self.remove_piece(color, Piece::Pawn, mv.from);
        self.add_piece(color, Piece::Pawn, mv.to)
    }

    fn make_castle(&mut self, mv: &Move) {
        let color = self.active_color;

        let is_king_side = match color {
            Color::White => G1 == mv.to,
            Color::Black => G8 == mv.to,
        };

        let (rook_from, rook_to) = match is_king_side {
            true => match color {
                Color::White => (H1, F1),
                Color::Black => (H8, F8),
            },
            false => match color {
                Color::White => (A1, D1),
                Color::Black => (A8, D8),
            },
        };

        self.remove_piece(color, Piece::King, mv.from);
        self.add_piece(color, Piece::King, mv.to);

        self.remove_piece(color, Piece::Rook, rook_from);
        self.add_piece(color, Piece::Rook, rook_to);
    }

    fn make_promotion(&mut self, mv: &Move) {
        let color = self.active_color;

        if let Some(piece) = self.get_piece_at(mv.to) {
            self.remove_piece(!color, piece, mv.to);
        }

        self.remove_piece(color, Piece::Pawn, mv.from);
        self.add_piece(color, mv.piece_type, mv.to);
    }

    pub fn get_piece_at(&self, square: Square) -> Option<Piece> {
        let square_bb = Bitboard::square_to_bitboard(square);

        PieceIterator::new().find(|&piece| self.bb_piece(piece) & square_bb != 0)
    }

    #[allow(dead_code)]
    pub fn get_color_at(&self, square: Square) -> Option<Color> {
        let square_bb = Bitboard::square_to_bitboard(square);

        ColorIterator::new().find(|&color| self.bb_color(color) & square_bb != 0)
    }

    fn is_double_pawn_push(&self, mv: &Move) -> bool {
        let color = self.active_color;

        let offset: i8 = match color {
            Color::White => 16,
            Color::Black => -16,
        };

        let is_double_push = mv.from as i8 + offset == mv.to as i8;
        let is_pawn = mv.piece_type == Piece::Pawn;

        is_double_push && is_pawn
    }
}

#[derive(Copy, Clone)]
pub struct Position {
    pieces: [Bitboard; PIECE_COUNT], // Six bitboards for the pieces
    colors: [Bitboard; COLOR_COUNT], // Two bitboards for the colors
}

impl Position {
    pub fn new() -> Self {
        let pieces = [0; PIECE_COUNT];
        let colors = [0; COLOR_COUNT];

        Self { pieces, colors }
    }

    // Creates the default chess starting position
    pub fn default() -> Self {
        let mut pieces = [0; PIECE_COUNT];
        let mut colors = [0; COLOR_COUNT];

        pieces[Piece::Pawn] = 0x00ff00000000ff00;
        pieces[Piece::Knight] = 0x4200000000000042;
        pieces[Piece::Bishop] = 0x2400000000000024;
        pieces[Piece::Rook] = 0x8100000000000081;
        pieces[Piece::Queen] = 0x0800000000000008;
        pieces[Piece::King] = 0x1000000000000010;

        colors[Color::White] = 0x000000000000ffff;
        colors[Color::Black] = 0xffff000000000000;

        Self { pieces, colors }
    }

    // Returns select pieces of a certain color e.g. white pawns
    pub fn bb(&self, color: Color, piece: Piece) -> u64 {
        self.pieces[piece] & self.colors[color]
    }

    // Returns all pieces of a certain color e.g. white pieces
    pub fn bb_color(&self, color: Color) -> u64 {
        self.colors[color]
    }

    // Returns all pieces of a select type e.g. pawns
    pub fn bb_piece(&self, piece: Piece) -> u64 {
        self.pieces[piece]
    }

    pub fn add_piece(&mut self, color: Color, piece: Piece, square: Square) {
        self.colors[color].set_bit(square);
        self.pieces[piece].set_bit(square);
    }

    pub fn remove_piece(&mut self, color: Color, piece: Piece, square: Square) {
        self.colors[color].remove_bit(square);
        self.pieces[piece].remove_bit(square);
    }
}

#[derive(Copy, Clone)]
pub struct Castle {
    white_king: bool,
    white_queen: bool,
    black_king: bool,
    black_queen: bool,
}

impl Castle {
    pub fn new(white_king: bool, white_queen: bool, black_king: bool, black_queen: bool) -> Self {
        Self {
            white_king,
            white_queen,
            black_king,
            black_queen,
        }
    }

    pub fn set(&mut self, castle: char, ability: bool) {
        match castle {
            'K' => self.white_king = ability,
            'Q' => self.white_queen = ability,
            'k' => self.black_king = ability,
            'q' => self.black_queen = ability,
            _ => {} // Can ignore any other character
        };
    }

    pub fn remove_all_rights(&mut self, color: Color) {
        match color {
            Color::White => {
                self.white_king = false;
                self.white_queen = false;
            }
            Color::Black => {
                self.black_king = false;
                self.black_queen = false;
            }
        };
    }

    pub fn remove_side_rights(&mut self, color: Color, side: Piece) {
        match color {
            Color::White => match side {
                Piece::King => self.white_king = false,
                Piece::Queen => self.white_queen = false,
                _ => {}
            },
            Color::Black => match side {
                Piece::King => self.black_king = false,
                Piece::Queen => self.black_queen = false,
                _ => {}
            },
        }
    }
}
