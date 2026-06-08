use crate::bitboard::{BitboardIterator, BitboardOperations, FILE_A, SQUARES};
use crate::board::Board;
use crate::lookup::LookupTable;
use crate::moves::{EAST, NORTH, SOUTH, WEST};
use crate::pieces::{Color, Piece, PIECE_COUNT};

type PST = [i32; SQUARES as usize];

const BISHOP_PAIR_OP: i32 = 25;
const BISHOP_PAIR_EG: i32 = 45;

const ROOK_OPEN_FILE_OP: i32 = 25;
const ROOK_OPEN_FILE_EG: i32 = 15;
const ROOK_SEMI_OPEN_FILE_OP: i32 = 12;
const ROOK_SEMI_OPEN_FILE_EG: i32 = 8;

// Indexed by the pawn's rank relative to its own side (0 = home rank).
const PASSED_PAWN_OP: [i32; 8] = [0, 5, 10, 15, 25, 45, 80, 0];
const PASSED_PAWN_EG: [i32; 8] = [0, 10, 18, 30, 50, 85, 130, 0];

// Penalty per missing pawn in the three-file zone in front of the king.
const KING_SHIELD_PENALTY: i32 = 12;

const KNIGHT_MOBILITY: i32 = 4;
const BISHOP_MOBILITY: i32 = 4;
const ROOK_MOBILITY_OP: i32 = 2;
const ROOK_MOBILITY_EG: i32 = 4;
const QUEEN_MOBILITY: i32 = 1;

// Attack-unit weights for pieces bearing on the squares around the enemy king.
const KING_ATTACK_KNIGHT: i32 = 2;
const KING_ATTACK_BISHOP: i32 = 2;
const KING_ATTACK_ROOK: i32 = 3;
const KING_ATTACK_QUEEN: i32 = 5;
// Upper bound on the (quadratic) king danger penalty, in centipawns.
const KING_DANGER_CAP: i32 = 500;

/// If the cheap material/PST score is this far outside the search window, the
/// positional extras cannot bring it back in, so they are skipped (lazy eval).
const LAZY_EVAL_MARGIN: i32 = 150;

const OPENING_TABLES: [PST; PIECE_COUNT] = [
    // Pawn (82 + positional)
    [
        82, 82, 82, 82, 82, 82, 82, 82, 180, 216, 143, 177, 150, 208, 116, 71, 76, 89, 108, 113,
        147, 138, 107, 62, 68, 95, 88, 103, 105, 94, 99, 59, 55, 80, 77, 94, 99, 88, 92, 57, 56,
        78, 78, 72, 85, 85, 115, 70, 47, 81, 62, 59, 67, 106, 120, 60, 82, 82, 82, 82, 82, 82, 82,
        82,
    ],
    // Knight (337 + positional)
    [
        170, 248, 303, 288, 398, 240, 322, 230, 264, 296, 409, 373, 360, 399, 344, 320, 290, 397,
        374, 402, 421, 466, 410, 381, 328, 354, 356, 390, 374, 406, 355, 359, 324, 341, 353, 350,
        365, 356, 358, 329, 314, 328, 349, 347, 356, 354, 362, 321, 308, 284, 325, 334, 336, 355,
        323, 318, 232, 316, 279, 304, 320, 309, 318, 314,
    ],
    // Bishop (365 + positional)
    [
        336, 369, 283, 328, 340, 323, 372, 357, 339, 381, 347, 352, 395, 424, 383, 318, 349, 402,
        408, 405, 400, 415, 402, 363, 361, 370, 384, 415, 402, 402, 372, 363, 359, 378, 378, 391,
        399, 377, 375, 369, 365, 380, 380, 380, 379, 392, 383, 375, 369, 380, 381, 365, 372, 386,
        398, 366, 332, 362, 351, 344, 352, 353, 326, 344,
    ],
    // Rook (477 + positional)
    [
        509, 519, 509, 528, 540, 486, 508, 520, 504, 509, 535, 539, 557, 544, 503, 521, 472, 496,
        503, 513, 494, 522, 538, 493, 453, 466, 484, 503, 501, 512, 469, 457, 441, 451, 465, 476,
        486, 470, 483, 454, 432, 452, 461, 460, 480, 477, 472, 444, 433, 461, 457, 468, 476, 488,
        471, 406, 458, 464, 478, 494, 493, 484, 440, 451,
    ],
    // Queen (1025 + positional)
    [
        997, 1025, 1054, 1037, 1084, 1069, 1068, 1070, 1001, 986, 1020, 1026, 1009, 1082, 1053,
        1079, 1012, 1008, 1032, 1033, 1054, 1081, 1072, 1082, 998, 998, 1009, 1009, 1024, 1042,
        1023, 1026, 1016, 999, 1016, 1015, 1023, 1021, 1028, 1022, 1011, 1027, 1014, 1023, 1020,
        1027, 1039, 1030, 990, 1017, 1036, 1027, 1033, 1040, 1022, 1026, 1024, 1007, 1016, 1035,
        1010, 1000, 994, 975,
    ],
    // King (0 + positional)
    [
        -65, 23, 16, -15, -56, -34, 2, 13, 29, -1, -20, -7, -8, -4, -38, -29, -9, 24, 2, -16, -20,
        6, 22, -22, -17, -20, -12, -27, -30, -25, -14, -36, -49, -1, -27, -39, -46, -44, -33, -51,
        -14, -14, -22, -46, -44, -30, -15, -27, 1, 7, -8, -64, -43, -16, 9, 8, -15, 36, 12, -54, 8,
        -28, 24, 14,
    ],
];

const ENDGAME_TABLES: [PST; PIECE_COUNT] = [
    // Pawn (94 + positional)
    [
        94, 94, 94, 94, 94, 94, 94, 94, 272, 267, 252, 228, 241, 226, 259, 281, 188, 194, 179, 161,
        150, 147, 176, 178, 126, 118, 107, 99, 92, 98, 111, 111, 107, 103, 91, 87, 87, 86, 97, 93,
        98, 101, 88, 95, 94, 89, 93, 86, 107, 102, 102, 104, 107, 94, 96, 87, 94, 94, 94, 94, 94,
        94, 94, 94,
    ],
    // Knight (281 + positional)
    [
        223, 243, 268, 253, 250, 254, 218, 182, 256, 273, 256, 279, 272, 256, 257, 229, 257, 261,
        291, 290, 280, 272, 262, 240, 264, 284, 303, 303, 303, 292, 289, 263, 263, 275, 297, 306,
        297, 298, 285, 263, 258, 278, 280, 296, 291, 278, 261, 259, 239, 261, 271, 276, 279, 261,
        258, 237, 252, 230, 258, 266, 259, 263, 231, 217,
    ],
    // Bishop (297 + positional)
    [
        283, 276, 286, 289, 290, 288, 280, 273, 289, 293, 304, 285, 294, 284, 293, 283, 299, 289,
        297, 296, 295, 303, 297, 301, 294, 306, 309, 306, 311, 307, 300, 299, 291, 300, 310, 316,
        304, 307, 294, 288, 285, 294, 305, 307, 310, 300, 290, 282, 283, 279, 290, 296, 301, 288,
        282, 270, 274, 288, 274, 292, 288, 281, 292, 280,
    ],
    // Rook (512 + positional)
    [
        525, 522, 530, 527, 524, 524, 520, 517, 523, 525, 525, 523, 509, 515, 520, 515, 519, 519,
        519, 517, 516, 509, 507, 509, 516, 515, 525, 513, 514, 513, 511, 514, 515, 517, 520, 516,
        507, 506, 504, 501, 508, 512, 507, 511, 505, 500, 504, 496, 506, 506, 512, 514, 503, 503,
        501, 509, 503, 514, 515, 511, 507, 499, 516, 492,
    ],
    // Queen (936 + positional)
    [
        927, 958, 958, 963, 963, 955, 946, 956, 919, 956, 968, 977, 994, 961, 966, 936, 916, 942,
        945, 985, 983, 971, 955, 945, 939, 958, 960, 981, 993, 976, 993, 972, 918, 964, 955, 983,
        967, 970, 975, 959, 920, 909, 951, 942, 945, 953, 946, 941, 914, 913, 906, 920, 920, 913,
        900, 904, 903, 908, 914, 893, 931, 904, 916, 895,
    ],
    // King (0 + positional)
    [
        -74, -35, -18, -18, -11, 15, 4, -17, -12, 17, 14, 17, 17, 38, 23, 11, 10, 17, 23, 15, 20,
        45, 44, 13, -8, 22, 24, 27, 26, 33, 26, 3, -18, -4, 21, 24, 27, 23, 9, -11, -19, -3, 11,
        21, 23, 16, 7, -9, -27, -11, 4, 13, 14, 4, -5, -17, -53, -34, -21, -11, -28, -14, -24, -43,
    ],
];

const PHASE_INCREMENTS: [i32; PIECE_COUNT] = [0, 1, 1, 2, 4, 0];

/// White-relative piece-square contribution of a single piece: returns the
/// `(opening, endgame, phase)` deltas the board accumulates incrementally as
/// pieces are added and removed.
pub fn pst_contribution(color: Color, piece: Piece, square: u8) -> (i32, i32, i32) {
    let table_index = if color == Color::White {
        (square ^ 56) as usize
    } else {
        square as usize
    };
    let piece_index = piece.index();
    let opening = OPENING_TABLES[piece_index][table_index];
    let endgame = ENDGAME_TABLES[piece_index][table_index];
    let phase = PHASE_INCREMENTS[piece_index];

    match color {
        Color::White => (opening, endgame, phase),
        Color::Black => (-opening, -endgame, phase),
    }
}

pub struct Evaluator;

impl Evaluator {
    pub fn new() -> Self {
        Self
    }

    /// Evaluates the position from the side to move's perspective.
    ///
    /// The material/piece-square score and game phase are maintained
    /// incrementally on the board; only the positional extras are computed here.
    /// When the cheap material/PST score already lies a clear margin outside the
    /// `[alpha, beta]` window, the expensive extras are skipped (lazy eval).
    pub fn evaluate(&self, board: &Board, lookup: &LookupTable, alpha: i32, beta: i32) -> i32 {
        let sign = if board.active_color() == Color::White {
            1
        } else {
            -1
        };
        let phase = board.phase.min(24);

        let material = taper(sign * board.pst_mg, sign * board.pst_eg, phase);
        if material - LAZY_EVAL_MARGIN >= beta || material + LAZY_EVAL_MARGIN <= alpha {
            return material;
        }

        let (extra_opening, extra_endgame) = eval_extras(board, lookup);
        taper(
            sign * (board.pst_mg + extra_opening),
            sign * (board.pst_eg + extra_endgame),
            phase,
        )
    }
}

/// Blends opening and endgame scores by game phase (24 = full material).
fn taper(opening: i32, endgame: i32, phase: i32) -> i32 {
    (opening * phase + endgame * (24 - phase)) / 24
}

fn north_fill(mut bb: u64) -> u64 {
    bb |= bb << 8;
    bb |= bb << 16;
    bb |= bb << 32;
    bb
}

fn south_fill(mut bb: u64) -> u64 {
    bb |= bb >> 8;
    bb |= bb >> 16;
    bb |= bb >> 32;
    bb
}

/// Sum of the positional terms not covered by the piece-square tables, computed
/// from White's perspective as (opening, endgame) tuples in centipawns.
fn eval_extras(board: &Board, lookup: &LookupTable) -> (i32, i32) {
    let (mut opening, mut endgame) = (0, 0);

    let (op, eg) = bishop_pair(board);
    opening += op;
    endgame += eg;

    let (op, eg) = passed_pawns(board);
    opening += op;
    endgame += eg;

    let (op, eg) = rook_files(board);
    opening += op;
    endgame += eg;

    opening += king_shield(board);

    let (op, eg) = mobility_and_king_safety(board, lookup);
    opening += op;
    endgame += eg;

    (opening, endgame)
}

fn bishop_pair(board: &Board) -> (i32, i32) {
    let white = (board.bb(Color::White, Piece::Bishop).count_ones() >= 2) as i32;
    let black = (board.bb(Color::Black, Piece::Bishop).count_ones() >= 2) as i32;
    let diff = white - black;
    (diff * BISHOP_PAIR_OP, diff * BISHOP_PAIR_EG)
}

fn passed_pawns(board: &Board) -> (i32, i32) {
    let white_pawns = board.bb(Color::White, Piece::Pawn);
    let black_pawns = board.bb(Color::Black, Piece::Pawn);

    // A pawn is passed when no enemy pawn stands on its file or an adjacent file
    // on any rank ahead of it.
    let black_spread = black_pawns | black_pawns.shift(EAST) | black_pawns.shift(WEST);
    let white_blocked = south_fill(black_spread.shift(SOUTH));
    let white_passers = white_pawns & !white_blocked;

    let white_spread = white_pawns | white_pawns.shift(EAST) | white_pawns.shift(WEST);
    let black_blocked = north_fill(white_spread.shift(NORTH));
    let black_passers = black_pawns & !black_blocked;

    let (mut opening, mut endgame) = (0, 0);
    for square in BitboardIterator::new(white_passers) {
        let rank = (square >> 3) as usize;
        opening += PASSED_PAWN_OP[rank];
        endgame += PASSED_PAWN_EG[rank];
    }
    for square in BitboardIterator::new(black_passers) {
        let rank = 7 - (square >> 3) as usize;
        opening -= PASSED_PAWN_OP[rank];
        endgame -= PASSED_PAWN_EG[rank];
    }
    (opening, endgame)
}

fn rook_files(board: &Board) -> (i32, i32) {
    let white_pawns = board.bb(Color::White, Piece::Pawn);
    let black_pawns = board.bb(Color::Black, Piece::Pawn);
    let all_pawns = white_pawns | black_pawns;

    let (mut opening, mut endgame) = (0, 0);
    for square in BitboardIterator::new(board.bb(Color::White, Piece::Rook)) {
        let file = FILE_A << (square & 7);
        if file & all_pawns == 0 {
            opening += ROOK_OPEN_FILE_OP;
            endgame += ROOK_OPEN_FILE_EG;
        } else if file & white_pawns == 0 {
            opening += ROOK_SEMI_OPEN_FILE_OP;
            endgame += ROOK_SEMI_OPEN_FILE_EG;
        }
    }
    for square in BitboardIterator::new(board.bb(Color::Black, Piece::Rook)) {
        let file = FILE_A << (square & 7);
        if file & all_pawns == 0 {
            opening -= ROOK_OPEN_FILE_OP;
            endgame -= ROOK_OPEN_FILE_EG;
        } else if file & black_pawns == 0 {
            opening -= ROOK_SEMI_OPEN_FILE_OP;
            endgame -= ROOK_SEMI_OPEN_FILE_EG;
        }
    }
    (opening, endgame)
}

/// Opening-only term: penalize each side for pawns missing from the three-file
/// zone directly in front of its king. Returned White-relative.
fn king_shield(board: &Board) -> i32 {
    let white_missing = shield_missing(
        board.bb(Color::White, Piece::King),
        board.bb(Color::White, Piece::Pawn),
        NORTH,
    );
    let black_missing = shield_missing(
        board.bb(Color::Black, Piece::King),
        board.bb(Color::Black, Piece::Pawn),
        SOUTH,
    );
    (black_missing - white_missing) * KING_SHIELD_PENALTY
}

fn shield_missing(king: u64, pawns: u64, forward: i8) -> i32 {
    let front = king.shift(forward);
    let near = front | front.shift(EAST) | front.shift(WEST);
    let zone = near | near.shift(forward);
    let present = (zone & pawns).count_ones() as i32;
    3 - present.min(3)
}

/// The squares around a king (its attacks plus its own square), used as the
/// zone an attacking side bears down on.
fn king_zone(lookup: &LookupTable, king: u64) -> u64 {
    let square = king.trailing_zeros() as u8;
    lookup.non_sliding_moves(square, Piece::King) | king
}

/// Maps accumulated attack units against a king to a centipawn penalty. The
/// growth is quadratic so a single attacker is cheap while several pile up fast.
fn king_danger_penalty(units: i32) -> i32 {
    (units * units).min(KING_DANGER_CAP)
}

/// Combined piece mobility and king safety. Each piece's attack set is computed
/// once and reused for both: its mobility (squares it can move to) and whether
/// it bears on the enemy king's zone.
fn mobility_and_king_safety(board: &Board, lookup: &LookupTable) -> (i32, i32) {
    let occupied = board.bb_all();
    let white_king_zone = king_zone(lookup, board.bb(Color::White, Piece::King));
    let black_king_zone = king_zone(lookup, board.bb(Color::Black, Piece::King));

    let (mut opening, mut endgame) = (0, 0);
    let (mut danger_to_white, mut danger_to_black) = (0, 0);

    for color in [Color::White, Color::Black] {
        let sign = if color == Color::White { 1 } else { -1 };
        let own = board.bb_color(color);
        let enemy_zone = if color == Color::White {
            black_king_zone
        } else {
            white_king_zone
        };
        let (mut op, mut eg) = (0, 0);
        let mut units = 0;

        for square in BitboardIterator::new(board.bb(color, Piece::Knight)) {
            let attacks = lookup.non_sliding_moves(square, Piece::Knight);
            let moves = (attacks & !own).count_ones() as i32;
            op += moves * KNIGHT_MOBILITY;
            eg += moves * KNIGHT_MOBILITY;
            if attacks & enemy_zone != 0 {
                units += KING_ATTACK_KNIGHT;
            }
        }
        for square in BitboardIterator::new(board.bb(color, Piece::Bishop)) {
            let attacks = lookup.sliding_moves(square, occupied, Piece::Bishop);
            let moves = (attacks & !own).count_ones() as i32;
            op += moves * BISHOP_MOBILITY;
            eg += moves * BISHOP_MOBILITY;
            if attacks & enemy_zone != 0 {
                units += KING_ATTACK_BISHOP;
            }
        }
        for square in BitboardIterator::new(board.bb(color, Piece::Rook)) {
            let attacks = lookup.sliding_moves(square, occupied, Piece::Rook);
            let moves = (attacks & !own).count_ones() as i32;
            op += moves * ROOK_MOBILITY_OP;
            eg += moves * ROOK_MOBILITY_EG;
            if attacks & enemy_zone != 0 {
                units += KING_ATTACK_ROOK;
            }
        }
        for square in BitboardIterator::new(board.bb(color, Piece::Queen)) {
            let attacks = lookup.sliding_moves(square, occupied, Piece::Queen);
            let moves = (attacks & !own).count_ones() as i32;
            op += moves * QUEEN_MOBILITY;
            eg += moves * QUEEN_MOBILITY;
            if attacks & enemy_zone != 0 {
                units += KING_ATTACK_QUEEN;
            }
        }

        opening += sign * op;
        endgame += sign * eg;
        if color == Color::White {
            danger_to_black += units;
        } else {
            danger_to_white += units;
        }
    }

    // King safety matters most in the opening/middlegame, so it rides the
    // opening score. A king under heavier attack is worse for its owner.
    opening += king_danger_penalty(danger_to_black) - king_danger_penalty(danger_to_white);

    (opening, endgame)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lookup::LookupTable;
    use std::time::Instant;

    #[test]
    fn test_eval() {
        let evaluator = Evaluator::new();
        let lookup = LookupTable::init();
        let board = Board::new("rnbqkb1r/p1pp1ppp/1p3n2/4N3/4P3/8/PPPP1PPP/RNBQKB1R w KQkq - 0 4");

        let start = Instant::now();

        evaluator.evaluate(&board, &lookup, -1_000_000, 1_000_000);

        let duration = start.elapsed();
        println!("Test took: {:?}", duration);
    }

    #[test]
    fn bishop_pair_favors_two_bishops() {
        // White has both bishops; Black has one bishop and a knight.
        let board = Board::new("4k3/8/8/8/8/8/8/2B1KB2 w - - 0 1");
        let (op, _eg) = bishop_pair(&board);
        assert_eq!(op, BISHOP_PAIR_OP);
    }

    #[test]
    fn passed_pawn_detected() {
        // White a-pawn on a5 with no black pawns ahead on a/b files is passed.
        let board = Board::new("4k3/8/8/P7/8/8/8/4K3 w - - 0 1");
        let (_op, eg) = passed_pawns(&board);
        // a5 is relative rank 4 for White.
        assert_eq!(eg, PASSED_PAWN_EG[4]);
    }

    #[test]
    fn blocked_pawn_is_not_passed() {
        // White a5 and black a7 share a file and block each other, so neither
        // pawn is passed and the term is zero.
        let board = Board::new("4k3/p7/8/P7/8/8/8/4K3 w - - 0 1");
        let (_op, eg) = passed_pawns(&board);
        assert_eq!(eg, 0);

        // A black pawn on an adjacent file ahead also denies White a passer.
        let board = Board::new("4k3/1p6/8/P7/8/8/8/4K3 w - - 0 1");
        let (_op, eg) = passed_pawns(&board);
        assert_eq!(eg, 0);
    }

    #[test]
    fn rook_on_open_file_scored() {
        // White rook on the open d-file (no pawns anywhere on d).
        let board = Board::new("4k3/8/8/8/8/8/8/3RK3 w - - 0 1");
        let (op, _eg) = rook_files(&board);
        assert_eq!(op, ROOK_OPEN_FILE_OP);
    }

    #[test]
    fn rook_on_semi_open_file_scored() {
        // White rook on d-file with only a black pawn on it: semi-open.
        let board = Board::new("4k3/3p4/8/8/8/8/8/3RK3 w - - 0 1");
        let (op, _eg) = rook_files(&board);
        assert_eq!(op, ROOK_SEMI_OPEN_FILE_OP);
    }
}
