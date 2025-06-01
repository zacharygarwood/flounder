use crate::board::Board;
use crate::bitboard::{SQUARES, BitboardIterator};
use crate::pieces::{Piece, Color, PIECE_COUNT, PieceIterator};

type PST = [i32; SQUARES as usize];

const OPENING_PAWN_TABLE: PST = [
      0,   0,   0,   0,   0,   0,  0,   0,
     98, 134,  61,  95,  68, 126, 34, -11,
     -6,   7,  26,  31,  65,  56, 25, -20,
    -14,  13,   6,  21,  23,  12, 17, -23,
    -27,  -2,  -5,  12,  17,   6, 10, -25,
    -26,  -4,  -4, -10,   3,   3, 33, -12,
    -35,  -1, -20, -23, -15,  24, 38, -22,
      0,   0,   0,   0,   0,   0,  0,   0,
];

const ENDGAME_PAWN_TABLE: PST = [
      0,   0,   0,   0,   0,   0,   0,   0,
    178, 173, 158, 134, 147, 132, 165, 187,
     94, 100,  85,  67,  56,  53,  82,  84,
     32,  24,  13,   5,  -2,   4,  17,  17,
     13,   9,  -3,  -7,  -7,  -8,   3,  -1,
      4,   7,  -6,   1,   0,  -5,  -1,  -8,
     13,   8,   8,  10,  13,   0,   2,  -7,
      0,   0,   0,   0,   0,   0,   0,   0,
];

const OPENING_KNIGHT_TABLE: PST = [
    -167, -89, -34, -49,  61, -97, -15, -107,
     -73, -41,  72,  36,  23,  62,   7,  -17,
     -47,  60,  37,  65,  84, 129,  73,   44,
      -9,  17,  19,  53,  37,  69,  18,   22,
     -13,   4,  16,  13,  28,  19,  21,   -8,
     -23,  -9,  12,  10,  19,  17,  25,  -16,
     -29, -53, -12,  -3,  -1,  18, -14,  -19,
    -105, -21, -58, -33, -17, -28, -19,  -23,
];

const ENDGAME_KNIGHT_TABLE: PST = [
    -58, -38, -13, -28, -31, -27, -63, -99,
    -25,  -8, -25,  -2,  -9, -25, -24, -52,
    -24, -20,  10,   9,  -1,  -9, -19, -41,
    -17,   3,  22,  22,  22,  11,   8, -18,
    -18,  -6,  16,  25,  16,  17,   4, -18,
    -23,  -3,  -1,  15,  10,  -3, -20, -22,
    -42, -20, -10,  -5,  -2, -20, -23, -44,
    -29, -51, -23, -15, -22, -18, -50, -64,
];

const OPENING_BISHOP_TABLE: PST = [
    -29,   4, -82, -37, -25, -42,   7,  -8,
    -26,  16, -18, -13,  30,  59,  18, -47,
    -16,  37,  43,  40,  35,  50,  37,  -2,
     -4,   5,  19,  50,  37,  37,   7,  -2,
     -6,  13,  13,  26,  34,  12,  10,   4,
      0,  15,  15,  15,  14,  27,  18,  10,
      4,  15,  16,   0,   7,  21,  33,   1,
    -33,  -3, -14, -21, -13, -12, -39, -21,
];

const ENDGAME_BISHOP_TABLE: PST = [
    -14, -21, -11,  -8, -7,  -9, -17, -24,
     -8,  -4,   7, -12, -3, -13,  -4, -14,
      2,  -8,   0,  -1, -2,   6,   0,   4,
     -3,   9,  12,   9, 14,  10,   3,   2,
     -6,   3,  13,  19,  7,  10,  -3,  -9,
    -12,  -3,   8,  10, 13,   3,  -7, -15,
    -14, -18,  -7,  -1,  4,  -9, -15, -27,
    -23,  -9, -23,  -5, -9, -16,  -5, -17,
];

const OPENING_ROOK_TABLE: PST = [
     32,  42,  32,  51, 63,  9,  31,  43,
     27,  32,  58,  62, 80, 67,  26,  44,
     -5,  19,  26,  36, 17, 45,  61,  16,
    -24, -11,   7,  26, 24, 35,  -8, -20,
    -36, -26, -12,  -1,  9, -7,   6, -23,
    -45, -25, -16, -17,  3,  0,  -5, -33,
    -44, -16, -20,  -9, -1, 11,  -6, -71,
    -19, -13,   1,  17, 16,  7, -37, -26,
];

const ENDGAME_ROOK_TABLE: PST = [
    13, 10, 18, 15, 12,  12,   8,   5,
    11, 13, 13, 11, -3,   3,   8,   3,
     7,  7,  7,  5,  4,  -3,  -5,  -3,
     4,  3, 13,  1,  2,   1,  -1,   2,
     3,  5,  8,  4, -5,  -6,  -8, -11,
    -4,  0, -5, -1, -7, -12,  -8, -16,
    -6, -6,  0,  2, -9,  -9, -11,  -3,
    -9,  2,  3, -1, -5, -13,   4, -20,
];

const OPENING_QUEEN_TABLE: PST = [
    -28,   0,  29,  12,  59,  44,  43,  45,
    -24, -39,  -5,   1, -16,  57,  28,  54,
    -13, -17,   7,   8,  29,  56,  47,  57,
    -27, -27, -16, -16,  -1,  17,  -2,   1,
     -9, -26,  -9, -10,  -2,  -4,   3,  -3,
    -14,   2, -11,  -2,  -5,   2,  14,   5,
    -35,  -8,  11,   2,   8,  15,  -3,   1,
     -1, -18,  -9,  10, -15, -25, -31, -50,
];

const ENDGAME_QUEEN_TABLE: PST = [
     -9,  22,  22,  27,  27,  19,  10,  20,
    -17,  20,  32,  41,  58,  25,  30,   0,
    -20,   6,   9,  49,  47,  35,  19,   9,
      3,  22,  24,  45,  57,  40,  57,  36,
    -18,  28,  19,  47,  31,  34,  39,  23,
    -16, -27,  15,   6,   9,  17,  10,   5,
    -22, -23, -30, -16, -16, -23, -36, -32,
    -33, -28, -22, -43,  -5, -32, -20, -41,
];

const OPENING_KING_TABLE: PST = [
    -65,  23,  16, -15, -56, -34,   2,  13,
     29,  -1, -20,  -7,  -8,  -4, -38, -29,
     -9,  24,   2, -16, -20,   6,  22, -22,
    -17, -20, -12, -27, -30, -25, -14, -36,
    -49,  -1, -27, -39, -46, -44, -33, -51,
    -14, -14, -22, -46, -44, -30, -15, -27,
      1,   7,  -8, -64, -43, -16,   9,   8,
    -15,  36,  12, -54,   8, -28,  24,  14,
];

const ENDGAME_KING_TABLE: PST = [
    -74, -35, -18, -18, -11,  15,   4, -17,
    -12,  17,  14,  17,  17,  38,  23,  11,
     10,  17,  23,  15,  20,  45,  44,  13,
     -8,  22,  24,  27,  26,  33,  26,   3,
    -18,  -4,  21,  24,  27,  23,   9, -11,
    -19,  -3,  11,  21,  23,  16,   7,  -9,
    -27, -11,   4,  13,  14,   4,  -5, -17,
    -53, -34, -21, -11, -28, -14, -24, -43
];

enum Phase {
    Opening,
    Endgame
}

pub struct Evaluator {
    gamephase: i32,
    opening_score: i32,
    endgame_score: i32,
    opening_tables: [PST; PIECE_COUNT],
    endgame_tables: [PST; PIECE_COUNT],
}

impl Evaluator {
    pub fn new() -> Self {
        Self {
            gamephase: 0,
            opening_score: 0,
            endgame_score: 0,
            opening_tables: Self::initialize_tables(&Phase::Opening),
            endgame_tables: Self::initialize_tables(&Phase::Endgame),
        }
    }

    /// ---------------------------------------------
    /// Functions to perform evaluation of the board
    /// ---------------------------------------------
    pub fn evaluate(&mut self, board: &Board) -> i32 {
        self.reset();

        let active_color = board.active_color();
        let piece_iter = PieceIterator::new();
        for piece in piece_iter {
            self.evaluate_piece(active_color, piece, board);
        }

        let mut opening_phase = self.gamephase;
        if opening_phase > 24 {
            // In case of early promotion
            opening_phase = 24;
        }

        let endgame_phase = 24 - opening_phase;
        eprintln!("Opening score: {}, Opening phase: {}, Endgame score: {}, Endgame phase: {}", self.opening_score, opening_phase, self.endgame_score, endgame_phase);
        (self.opening_score * opening_phase + self.endgame_score * endgame_phase) / 24
    }

    fn evaluate_piece(
        &mut self,
        color: Color,
        piece: Piece,
        board: &Board,
    ) {
        // Calculate active player and oponent scores and increments for the passed peice
        let (p_opening_score, p_endgame_score, p_gamephase) = self.evaluate_piece_position(color, piece, board);
        let (o_opening_score, o_endgame_score, o_gamephase) = self.evaluate_piece_position(!color, piece, board);

        self.opening_score += p_opening_score - o_opening_score;
        self.endgame_score += p_endgame_score - o_endgame_score;
        self.gamephase += p_gamephase + o_gamephase;
    }

    fn evaluate_piece_position(
        &self,
        color: Color,
        piece: Piece,
        board: &Board
    ) -> (i32, i32, i32) {
        let mut opening_score = 0;
        let mut endgame_score = 0;
        let mut gamephase = 0;

        let pieces = board.bb(color, piece);
        let bitboard_iter = BitboardIterator::new(pieces);
        for bit in bitboard_iter {
            let square = match color {
                Color::White => (7 - bit / 8) * 8 + bit % 8,
                Color::Black => bit
            };
            opening_score += self.opening_tables[piece.index()][square as usize];
            endgame_score += self.endgame_tables[piece.index()][square as usize];
            gamephase += Self::phase_increment(piece);
        }
        (opening_score, endgame_score, gamephase)
    }

    fn phase_increment(piece: Piece) -> i32 {
        match piece {
            Piece::Pawn => 0,
            Piece::Knight => 1,
            Piece::Bishop => 1,
            Piece::Rook => 2,
            Piece::Queen => 4,
            Piece::King => 0,
        }
    }

    fn reset(&mut self) {
        self.opening_score = 0;
        self.endgame_score = 0;
        self.gamephase = 0;
    }

    /// --------------------------------------------
    /// Functions to construct the Evaluator fields
    /// --------------------------------------------
    fn initialize_tables(phase: &Phase) -> [PST; PIECE_COUNT] {
        let pst = [0; 64];
        let mut tables = [pst; PIECE_COUNT];

        let piece_iter = PieceIterator::new();
        for piece in piece_iter {
            tables[piece.index()] = Self::piece_square_table(piece, phase);
        }

        tables
    }

    fn piece_square_table(piece: Piece, phase: &Phase) -> PST {
        match phase {
            Phase::Opening => Self::opening_table(piece),
            Phase::Endgame => Self::endgame_table(piece)
        }
    }

    fn populate_table(table: PST, piece_value: i32) -> PST {
        table.map(|x| x + piece_value)
    }

    fn opening_table(piece: Piece) -> PST {
        let piece_value = Self::opening_value(piece);
        let table = match piece {
            Piece::Pawn => OPENING_PAWN_TABLE,
            Piece::Knight => OPENING_KNIGHT_TABLE,
            Piece::Bishop => OPENING_BISHOP_TABLE,
            Piece::Rook => OPENING_ROOK_TABLE,
            Piece::Queen => OPENING_QUEEN_TABLE,
            Piece::King => OPENING_KING_TABLE,
        };

        Self::populate_table(table, piece_value)
    }

    fn endgame_table(piece: Piece) -> PST {
        let piece_value = Self::endgame_value(piece);
        let table = match piece {
            Piece::Pawn => ENDGAME_PAWN_TABLE,
            Piece::Knight => ENDGAME_KNIGHT_TABLE,
            Piece::Bishop => ENDGAME_BISHOP_TABLE,
            Piece::Rook => ENDGAME_ROOK_TABLE,
            Piece::Queen => ENDGAME_QUEEN_TABLE,
            Piece::King => ENDGAME_KING_TABLE,
        };

        Self::populate_table(table, piece_value)
    }

    fn opening_value(piece: Piece) -> i32 {
        match piece {
            Piece::Pawn => 82,
            Piece::Knight => 337,
            Piece::Bishop => 365,
            Piece::Rook => 477,
            Piece::Queen => 1025,
            Piece::King => 0,
        }
    }

    fn endgame_value(piece: Piece) -> i32 {
        match piece {
            Piece::Pawn => 94,
            Piece::Knight => 281,
            Piece::Bishop => 297,
            Piece::Rook => 512,
            Piece::Queen => 936,
            Piece::King => 0,
        }
    }
}

