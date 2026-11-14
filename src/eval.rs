use crate::bitboard::{BitboardIterator, BitboardOperations, FILE_A, RANK_3, RANK_4, RANK_5, RANK_6, SQUARES};
use crate::board::Board;
use crate::lookup::LookupTable;
use crate::moves::{EAST, NORTH, SOUTH, WEST};
use crate::pieces::{Color, Piece, PIECE_COUNT};

pub type PST = [i32; SQUARES as usize];

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

pub const OPENING_TABLES: [PST; PIECE_COUNT] = [
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

pub const ENDGAME_TABLES: [PST; PIECE_COUNT] = [
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

/// Indices of the positional features the eval scores, each weighted by an
/// (opening, endgame) pair in `WEIGHTS`.
pub mod feature {
    pub const BISHOP_PAIR: usize = 0;
    /// Eight entries, by the pawn's rank relative to its own side.
    pub const PASSED_PAWN: usize = BISHOP_PAIR + 1;
    pub const ROOK_OPEN_FILE: usize = PASSED_PAWN + 8;
    pub const ROOK_SEMI_OPEN_FILE: usize = ROOK_OPEN_FILE + 1;
    /// Pawns missing from the three-file zone in front of the king.
    pub const KING_SHIELD_MISSING: usize = ROOK_SEMI_OPEN_FILE + 1;
    /// Per-square mobility of knight, bishop, rook and queen, counting only
    /// squares not attacked by enemy pawns.
    pub const MOBILITY: usize = KING_SHIELD_MISSING + 1;
    /// A pawn with another pawn of its side further up the same file.
    pub const DOUBLED_PAWN: usize = MOBILITY + 4;
    /// A pawn with no pawns of its side on the adjacent files.
    pub const ISOLATED_PAWN: usize = DOUBLED_PAWN + 1;
    /// A knight / bishop on ranks 4-6, defended by a pawn, that no enemy pawn
    /// can ever drive away.
    pub const KNIGHT_OUTPOST: usize = ISOLATED_PAWN + 1;
    pub const BISHOP_OUTPOST: usize = KNIGHT_OUTPOST + 1;
    /// An enemy piece (not a pawn) attacked by one of our pawns.
    pub const PAWN_THREAT: usize = BISHOP_OUTPOST + 1;
    /// An enemy rook or queen attacked by one of our knights or bishops.
    pub const MINOR_THREAT: usize = PAWN_THREAT + 1;
    /// The side to move, who can act on the position first.
    pub const TEMPO: usize = MINOR_THREAT + 1;
    pub const COUNT: usize = TEMPO + 1;
}

/// (opening, endgame) weight of each feature, in centipawns.
#[rustfmt::skip]
pub const WEIGHTS: [(i32, i32); feature::COUNT] = [
    (25, 46),
    (0, 0),
    (13, -5),
    (4, 4),
    (-1, 23),
    (8, 41),
    (-8, 49),
    (-64, 40),
    (0, 0),
    (34, -7),
    (6, 12),
    (-19, 7),
    (6, 0),
    (4, 0),
    (2, 2),
    (1, 5),
    (-7, -21),
    (-15, -4),
    (26, 15),
    (28, -7),
    (60, 3),
    (38, 20),
    (36, 26),
];

/// How a position scores on each feature, White minus Black, plus the king
/// danger term, which is not linear in any weight.
pub struct Features {
    pub counts: [i32; feature::COUNT],
    /// White-relative opening-phase king-danger score in centipawns.
    pub king_danger: i32,
}

/// Sum of the positional terms not covered by the piece-square tables, computed
/// from White's perspective as (opening, endgame) tuples in centipawns.
pub fn eval_extras(board: &Board, lookup: &LookupTable) -> (i32, i32) {
    let features = features(board, lookup);
    let (mut opening, mut endgame) = (features.king_danger, 0);
    for (count, (op, eg)) in features.counts.iter().zip(WEIGHTS) {
        opening += count * op;
        endgame += count * eg;
    }
    (opening, endgame)
}

/// Measures every positional feature of `board`.
pub fn features(board: &Board, lookup: &LookupTable) -> Features {
    let mut counts = [0; feature::COUNT];
    let white_pawns = board.bb(Color::White, Piece::Pawn);
    let black_pawns = board.bb(Color::Black, Piece::Pawn);

    counts[feature::BISHOP_PAIR] = bishop_pair(board, Color::White) - bishop_pair(board, Color::Black);
    passed_pawns(white_pawns, black_pawns, &mut counts);
    rook_files(board, &mut counts);
    counts[feature::KING_SHIELD_MISSING] =
        shield_missing(board.bb(Color::White, Piece::King), white_pawns, NORTH)
            - shield_missing(board.bb(Color::Black, Piece::King), black_pawns, SOUTH);
    counts[feature::DOUBLED_PAWN] = doubled_pawns(white_pawns) - doubled_pawns(black_pawns);
    counts[feature::ISOLATED_PAWN] = isolated_pawns(white_pawns) - isolated_pawns(black_pawns);
    outposts(board, &mut counts);
    counts[feature::PAWN_THREAT] = pawn_threats(board, Color::White) - pawn_threats(board, Color::Black);
    counts[feature::TEMPO] = if board.active_color() == Color::White { 1 } else { -1 };
    let king_danger = mobility_and_king_safety(board, lookup, &mut counts);

    Features { counts, king_danger }
}

fn bishop_pair(board: &Board, color: Color) -> i32 {
    (board.bb(color, Piece::Bishop).count_ones() >= 2) as i32
}

fn pawn_attacks(pawns: u64, color: Color) -> u64 {
    match color {
        Color::White => pawns.shift(NORTH + EAST) | pawns.shift(NORTH + WEST),
        Color::Black => pawns.shift(SOUTH + EAST) | pawns.shift(SOUTH + WEST),
    }
}

fn adjacent_files(bb: u64) -> u64 {
    bb.shift(EAST) | bb.shift(WEST)
}

fn passed_pawns(white_pawns: u64, black_pawns: u64, counts: &mut [i32; feature::COUNT]) {
    // A pawn is passed when no enemy pawn stands on its file or an adjacent file
    // on any rank ahead of it.
    let white_blocked = south_fill((black_pawns | adjacent_files(black_pawns)).shift(SOUTH));
    let black_blocked = north_fill((white_pawns | adjacent_files(white_pawns)).shift(NORTH));
    for square in BitboardIterator::new(white_pawns & !white_blocked) {
        counts[feature::PASSED_PAWN + (square >> 3) as usize] += 1;
    }
    for square in BitboardIterator::new(black_pawns & !black_blocked) {
        counts[feature::PASSED_PAWN + 7 - (square >> 3) as usize] -= 1;
    }
}

fn rook_files(board: &Board, counts: &mut [i32; feature::COUNT]) {
    let all_pawns = board.bb_piece(Piece::Pawn);
    for (color, sign) in [(Color::White, 1), (Color::Black, -1)] {
        let own_pawns = board.bb(color, Piece::Pawn);
        for square in BitboardIterator::new(board.bb(color, Piece::Rook)) {
            let file = FILE_A << (square & 7);
            if file & all_pawns == 0 {
                counts[feature::ROOK_OPEN_FILE] += sign;
            } else if file & own_pawns == 0 {
                counts[feature::ROOK_SEMI_OPEN_FILE] += sign;
            }
        }
    }
}

fn shield_missing(king: u64, pawns: u64, forward: i8) -> i32 {
    let front = king.shift(forward);
    let near = front | front.shift(EAST) | front.shift(WEST);
    let zone = near | near.shift(forward);
    let present = (zone & pawns).count_ones() as i32;
    3 - present.min(3)
}

/// Pawns with another pawn of the same side somewhere above them on the file:
/// a file with n pawns counts n - 1.
fn doubled_pawns(pawns: u64) -> i32 {
    (pawns & south_fill(pawns >> 8)).count_ones() as i32
}

fn isolated_pawns(pawns: u64) -> i32 {
    let files_with_pawns = north_fill(south_fill(pawns));
    (pawns & !adjacent_files(files_with_pawns)).count_ones() as i32
}

fn outposts(board: &Board, counts: &mut [i32; feature::COUNT]) {
    let white_pawns = board.bb(Color::White, Piece::Pawn);
    let black_pawns = board.bb(Color::Black, Piece::Pawn);
    // Squares an enemy pawn attacks now or could after advancing.
    let black_reach = south_fill(pawn_attacks(black_pawns, Color::Black));
    let white_reach = north_fill(pawn_attacks(white_pawns, Color::White));
    let white_outposts = (RANK_4 | RANK_5 | RANK_6) & pawn_attacks(white_pawns, Color::White) & !black_reach;
    let black_outposts = (RANK_3 | RANK_4 | RANK_5) & pawn_attacks(black_pawns, Color::Black) & !white_reach;

    for (piece, index) in [(Piece::Knight, feature::KNIGHT_OUTPOST), (Piece::Bishop, feature::BISHOP_OUTPOST)] {
        counts[index] = (board.bb(Color::White, piece) & white_outposts).count_ones() as i32
            - (board.bb(Color::Black, piece) & black_outposts).count_ones() as i32;
    }
}

fn pawn_threats(board: &Board, color: Color) -> i32 {
    let enemy_pieces = board.bb_color(!color) & !board.bb_piece(Piece::Pawn) & !board.bb_piece(Piece::King);
    (pawn_attacks(board.bb(color, Piece::Pawn), color) & enemy_pieces).count_ones() as i32
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

/// Mobility, minor-piece threats and king safety. Each piece's attack set is
/// computed once and reused for all three. Returns the White-relative king
/// danger score.
fn mobility_and_king_safety(board: &Board, lookup: &LookupTable, counts: &mut [i32; feature::COUNT]) -> i32 {
    let occupied = board.bb_all();
    let mut king_danger = 0;

    for (color, sign) in [(Color::White, 1), (Color::Black, -1)] {
        let own = board.bb_color(color);
        let enemy_majors = board.bb_color(!color) & (board.bb_piece(Piece::Rook) | board.bb_piece(Piece::Queen));
        // Squares attacked by an enemy pawn are not real mobility: a piece
        // moving there is lost for a pawn.
        let mobility_area = !own & !pawn_attacks(board.bb(!color, Piece::Pawn), !color);
        let enemy_zone = king_zone(lookup, board.bb(!color, Piece::King));
        let mut units = 0;
        let mut minor_attacks = 0;

        for (offset, piece, attack_units) in [
            (0, Piece::Knight, KING_ATTACK_KNIGHT),
            (1, Piece::Bishop, KING_ATTACK_BISHOP),
            (2, Piece::Rook, KING_ATTACK_ROOK),
            (3, Piece::Queen, KING_ATTACK_QUEEN),
        ] {
            for square in BitboardIterator::new(board.bb(color, piece)) {
                let attacks = match piece {
                    Piece::Knight => lookup.non_sliding_moves(square, piece),
                    _ => lookup.sliding_moves(square, occupied, piece),
                };
                counts[feature::MOBILITY + offset] += sign * (attacks & mobility_area).count_ones() as i32;
                if attacks & enemy_zone != 0 {
                    units += attack_units;
                }
                if offset < 2 {
                    minor_attacks |= attacks;
                }
            }
        }

        counts[feature::MINOR_THREAT] += sign * (minor_attacks & enemy_majors).count_ones() as i32;
        // A king under heavier attack is worse for its owner.
        king_danger += sign * king_danger_penalty(units);
    }

    king_danger
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

    /// The same position with colors swapped and the board flipped top to
    /// bottom, so each side has the other's pieces and the other's move.
    /// Expects a FEN without castling rights or en passant square.
    fn color_flipped(fen: &str) -> String {
        let (placement, side) = fen.split_once(' ').unwrap();
        let swap_case = |c: char| if c.is_ascii_uppercase() { c.to_ascii_lowercase() } else { c.to_ascii_uppercase() };
        let flipped: Vec<String> = placement.split('/').rev().map(|rank| rank.chars().map(swap_case).collect()).collect();
        let side = if side.starts_with('w') { "b" } else { "w" };
        format!("{} {} - - 0 1", flipped.join("/"), side)
    }

    /// Piece placement and side to move as a FEN. Castling and en passant do
    /// not affect the evaluation, so they are left out.
    fn placement_fen(board: &Board) -> String {
        let mut ranks = Vec::new();
        for rank in (0..8).rev() {
            let mut row = String::new();
            let mut empty = 0;
            for file in 0..8 {
                let square = rank * 8 + file;
                match (board.get_piece_at(square), board.get_color_at(square)) {
                    (Some(piece), Some(color)) => {
                        if empty > 0 {
                            row += &empty.to_string();
                            empty = 0;
                        }
                        let c = b"pnbrqk"[piece.index()] as char;
                        row.push(if color == Color::White { c.to_ascii_uppercase() } else { c });
                    }
                    _ => empty += 1,
                }
            }
            if empty > 0 {
                row += &empty.to_string();
            }
            ranks.push(row);
        }
        let side = if board.active_color() == Color::White { "w" } else { "b" };
        format!("{} {} - - 0 1", ranks.join("/"), side)
    }

    #[test]
    fn evaluation_is_the_same_for_either_color_in_random_games() {
        use crate::move_gen::MoveGenerator;
        use rand::{rngs::StdRng, Rng, SeedableRng};

        let lookup = LookupTable::init();
        let evaluator = Evaluator::new();
        let move_gen = MoveGenerator::new();
        let mut rng = StdRng::seed_from_u64(0x5EED);
        for _ in 0..200 {
            let mut board = Board::default();
            for _ in 0..80 {
                let moves = move_gen.generate_moves(&board);
                if moves.is_empty() {
                    break;
                }
                board.make_move(&moves[rng.gen_range(0..moves.len())]);
                let fen = placement_fen(&board);
                let flipped = color_flipped(&fen);
                assert_eq!(
                    evaluator.evaluate(&Board::new(&fen), &lookup, -1_000_000, 1_000_000),
                    evaluator.evaluate(&Board::new(&flipped), &lookup, -1_000_000, 1_000_000),
                    "{fen} vs {flipped}"
                );
            }
        }
    }

    #[test]
    fn color_flip_swaps_sides_and_mirrors_ranks() {
        assert_eq!(color_flipped("4k3/8/8/3N4/2P5/8/8/4K3 w - - 0 1"), "4k3/8/8/2p5/3n4/8/8/4K3 b - - 0 1");
    }

    fn features_of(fen: &str) -> [i32; feature::COUNT] {
        features(&Board::new(fen), &LookupTable::init()).counts
    }

    #[test]
    fn bishop_pair_favors_two_bishops() {
        // White has both bishops; Black has none.
        assert_eq!(features_of("4k3/8/8/8/8/8/8/2B1KB2 w - - 0 1")[feature::BISHOP_PAIR], 1);
    }

    #[test]
    fn passed_pawn_counted_at_its_relative_rank() {
        // a5 is relative rank 4 for White.
        let counts = features_of("4k3/8/8/P7/8/8/8/4K3 w - - 0 1");
        assert_eq!(counts[feature::PASSED_PAWN + 4], 1);
    }

    #[test]
    fn blocked_pawn_is_not_passed() {
        let same_file = features_of("4k3/p7/8/P7/8/8/8/4K3 w - - 0 1");
        let adjacent_file = features_of("4k3/1p6/8/P7/8/8/8/4K3 w - - 0 1");
        for counts in [same_file, adjacent_file] {
            assert!(counts[feature::PASSED_PAWN..feature::PASSED_PAWN + 8].iter().all(|&c| c == 0));
        }
    }

    #[test]
    fn rook_on_open_file_scored() {
        assert_eq!(features_of("4k3/8/8/8/8/8/8/3RK3 w - - 0 1")[feature::ROOK_OPEN_FILE], 1);
    }

    #[test]
    fn rook_on_semi_open_file_scored() {
        // Only a black pawn on the d-file.
        assert_eq!(features_of("4k3/3p4/8/8/8/8/8/3RK3 w - - 0 1")[feature::ROOK_SEMI_OPEN_FILE], 1);
    }

    #[test]
    fn tripled_pawns_count_as_two_doubled() {
        assert_eq!(features_of("4k3/8/8/2P5/2P5/2P5/8/4K3 w - - 0 1")[feature::DOUBLED_PAWN], 2);
    }

    #[test]
    fn pawn_without_neighbours_is_isolated() {
        // a2 has no b-pawn; c2 and d2 support each other; Black's h7 is isolated.
        assert_eq!(features_of("4k3/7p/8/8/8/8/P1PP4/4K3 w - - 0 1")[feature::ISOLATED_PAWN], 0);
        assert_eq!(features_of("4k3/8/8/8/8/8/P1PP4/4K3 w - - 0 1")[feature::ISOLATED_PAWN], 1);
    }

    #[test]
    fn defended_knight_beyond_enemy_pawns_reach_is_an_outpost() {
        assert_eq!(features_of("4k3/8/8/3N4/2P5/8/8/4K3 w - - 0 1")[feature::KNIGHT_OUTPOST], 1);
        // A black c7 pawn can advance to c6 and attack d5, so d5 is no outpost.
        assert_eq!(features_of("4k3/2p5/8/3N4/2P5/8/8/4K3 w - - 0 1")[feature::KNIGHT_OUTPOST], 0);
    }

    #[test]
    fn pawn_attacking_a_piece_is_a_threat() {
        // White e4 pawn attacks the black knight on d5.
        assert_eq!(features_of("4k3/8/8/3n4/4P3/8/8/4K3 w - - 0 1")[feature::PAWN_THREAT], 1);
    }

    #[test]
    fn minor_attacking_a_major_is_a_threat() {
        // White knight on c3 attacks the black rook on d5.
        assert_eq!(features_of("4k3/8/8/3r4/8/2N5/8/4K3 w - - 0 1")[feature::MINOR_THREAT], 1);
    }

    #[test]
    fn tempo_belongs_to_the_side_to_move() {
        assert_eq!(features_of("4k3/8/8/8/8/8/8/4K3 w - - 0 1")[feature::TEMPO], 1);
        assert_eq!(features_of("4k3/8/8/8/8/8/8/4K3 b - - 0 1")[feature::TEMPO], -1);
    }
}
