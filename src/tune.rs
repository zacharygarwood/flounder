//! Texel tuning: self-play data generation and evaluation weight fitting.
//!
//! `datagen` plays engine-vs-engine games from randomized openings and records
//! quiet positions labeled with the game result. `tune` then fits the
//! evaluation weights so that a sigmoid of the evaluation best predicts those
//! results, and prints the tuned weights as Rust constants for `eval.rs`.

use crate::board::Board;
use crate::eval::tuning::{self, Trace, PARAM_COUNT, PST_PARAMS};
use crate::move_gen::MoveGenerator;
use crate::moves::MoveType;
use crate::pieces::{Color, Piece};
use crate::search::Searcher;
use rand::{rngs::StdRng, Rng, SeedableRng};
use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Write};

/// The piece-square tables (which include material) are left as they are.
/// Game phase is derived from material, so material's opening and endgame
/// weights can trade off against each other almost freely; tuning them on
/// self-play data produced nonsense values (an endgame knight worth a pawn)
/// and lost heavily. Only the positional terms after the tables are tuned.
const FIRST_TUNED_PARAM: usize = PST_PARAMS;

/// Random plies played from the start position so games diverge.
const RANDOM_OPENING_PLIES: usize = 8;

/// Games still running after this many plies are scored as draws.
const MAX_GAME_PLIES: usize = 400;

/// A score (White-relative, centipawns) at least this large for
/// `ADJUDICATE_PLIES` consecutive plies ends the game as a win.
const ADJUDICATE_WIN_SCORE: i32 = 1500;
const ADJUDICATE_PLIES: usize = 4;

/// After `DRAW_ADJUDICATE_AFTER` plies, a score within this many centipawns of
/// zero for `DRAW_ADJUDICATE_PLIES` consecutive plies ends the game as a draw.
const DRAW_ADJUDICATE_SCORE: i32 = 10;
const DRAW_ADJUDICATE_AFTER: usize = 80;
const DRAW_ADJUDICATE_PLIES: usize = 10;

/// Plays `games` self-play games searched to `depth` and appends each quiet
/// position, labeled with the game's result for White, to `out_path` as
/// `fen;result` lines.
pub fn datagen(games: usize, seed: u64, depth: u8, out_path: &str) -> std::io::Result<()> {
    let move_gen = MoveGenerator::new();
    let mut searcher = Searcher::new();
    let mut rng = StdRng::seed_from_u64(seed);
    let mut out = BufWriter::new(File::create(out_path)?);
    let mut positions = 0usize;

    for game in 0..games {
        let mut board = Board::default();
        let mut history: Vec<Board> = Vec::new();

        // Randomize the opening, abandoning lines that end the game.
        let mut playable = true;
        for _ in 0..RANDOM_OPENING_PLIES {
            let moves = move_gen.generate_moves(&board);
            if moves.is_empty() {
                playable = false;
                break;
            }
            history.push(board);
            board.make_move(&moves[rng.gen_range(0..moves.len())]);
        }
        if !playable || move_gen.generate_moves(&board).is_empty() {
            continue;
        }

        let mut fens = Vec::new();
        let mut result = 0.5;
        let mut winning_plies = 0;
        let mut drawish_plies = 0;

        for ply in 0..MAX_GAME_PLIES {
            let in_check = move_gen.is_in_check(&board);
            if move_gen.generate_moves(&board).is_empty() {
                result = match (in_check, board.active_color()) {
                    (true, Color::White) => 0.0,
                    (true, Color::Black) => 1.0,
                    (false, _) => 0.5,
                };
                break;
            }
            if board.halfmove_clock >= 100
                || is_threefold(&history, &board)
                || insufficient_material(&board)
            {
                break;
            }

            searcher.reset_repetition();
            for position in &history {
                searcher.record_game_position(position);
            }
            let (score, best_move) = searcher.find_best_move(&board, depth, None);
            let best_move = best_move.expect("a position with legal moves has a best move");
            let white_score = match board.active_color() {
                Color::White => score,
                Color::Black => -score,
            };

            winning_plies = if white_score.abs() >= ADJUDICATE_WIN_SCORE { winning_plies + 1 } else { 0 };
            if winning_plies >= ADJUDICATE_PLIES {
                result = if white_score > 0 { 1.0 } else { 0.0 };
                break;
            }
            drawish_plies = if white_score.abs() <= DRAW_ADJUDICATE_SCORE { drawish_plies + 1 } else { 0 };
            if ply >= DRAW_ADJUDICATE_AFTER && drawish_plies >= DRAW_ADJUDICATE_PLIES {
                break;
            }

            // Keep only quiet positions, where the static evaluation is a fair
            // estimate of the position's value.
            if !in_check && matches!(best_move.move_type, MoveType::Quiet | MoveType::Castle) {
                fens.push(position_fen(&board));
            }

            history.push(board);
            board.make_move(&best_move);
        }

        for fen in &fens {
            writeln!(out, "{};{}", fen, result)?;
        }
        positions += fens.len();
        if (game + 1) % 100 == 0 {
            eprintln!("datagen: {} games, {} positions", game + 1, positions);
        }
    }

    out.flush()
}

fn is_threefold(history: &[Board], board: &Board) -> bool {
    history.iter().filter(|position| position.hash == board.hash).count() >= 2
}

/// Neither side has mating material: bare kings plus at most one minor piece.
fn insufficient_material(board: &Board) -> bool {
    let heavy = board.bb_piece(Piece::Pawn) | board.bb_piece(Piece::Rook) | board.bb_piece(Piece::Queen);
    let minors = board.bb_piece(Piece::Knight) | board.bb_piece(Piece::Bishop);
    heavy == 0 && minors.count_ones() <= 1
}

/// The piece placement and side to move as a FEN; the remaining fields do not
/// affect the evaluation.
fn position_fen(board: &Board) -> String {
    let mut fen = String::new();
    for rank in (0..8).rev() {
        let mut empty = 0;
        for file in 0..8 {
            let square = rank * 8 + file;
            match (board.get_piece_at(square), board.get_color_at(square)) {
                (Some(piece), Some(color)) => {
                    if empty > 0 {
                        fen += &empty.to_string();
                        empty = 0;
                    }
                    let c = match piece {
                        Piece::Pawn => 'p',
                        Piece::Knight => 'n',
                        Piece::Bishop => 'b',
                        Piece::Rook => 'r',
                        Piece::Queen => 'q',
                        Piece::King => 'k',
                    };
                    fen.push(if color == Color::White { c.to_ascii_uppercase() } else { c });
                }
                _ => empty += 1,
            }
        }
        if empty > 0 {
            fen += &empty.to_string();
        }
        if rank > 0 {
            fen.push('/');
        }
    }
    let side = if board.active_color() == Color::White { "w" } else { "b" };
    format!("{} {} - - 0 1", fen, side)
}

struct Sample {
    trace: Trace,
    result: f64,
}

/// White-relative evaluation of a sample under `params`.
fn sample_eval(sample: &Sample, params: &[[f64; 2]]) -> f64 {
    let mut opening = sample.trace.fixed_opening as f64;
    let mut endgame = 0.0;
    for &(index, coef) in &sample.trace.features {
        opening += coef as f64 * params[index as usize][0];
        endgame += coef as f64 * params[index as usize][1];
    }
    let phase = sample.trace.phase as f64 / 24.0;
    opening * phase + endgame * (1.0 - phase)
}

fn sigmoid(eval: f64, k: f64) -> f64 {
    1.0 / (1.0 + 10f64.powf(-k * eval / 400.0))
}

/// Mean squared error between predicted and actual results.
fn error(samples: &[Sample], params: &[[f64; 2]], k: f64) -> f64 {
    let total: f64 = samples
        .iter()
        .map(|sample| (sample.result - sigmoid(sample_eval(sample, params), k)).powi(2))
        .sum();
    total / samples.len() as f64
}

/// Gradient of the error with respect to each (opening, endgame) weight,
/// computed in parallel over chunks of the samples.
fn gradient(samples: &[Sample], params: &[[f64; 2]], k: f64) -> Vec<[f64; 2]> {
    let threads = std::thread::available_parallelism().map_or(1, |n| n.get());
    let chunk_size = samples.len().div_ceil(threads).max(1);
    let partials: Vec<Vec<[f64; 2]>> = std::thread::scope(|scope| {
        let handles: Vec<_> = samples
            .chunks(chunk_size)
            .map(|chunk| {
                scope.spawn(move || {
                    let mut grad = vec![[0.0; 2]; PARAM_COUNT];
                    for sample in chunk {
                        let predicted = sigmoid(sample_eval(sample, params), k);
                        // d(error)/d(eval), dropping constant factors that the
                        // optimizer's step size absorbs.
                        let slope = (predicted - sample.result) * predicted * (1.0 - predicted);
                        let phase = sample.trace.phase as f64 / 24.0;
                        for &(index, coef) in &sample.trace.features {
                            grad[index as usize][0] += slope * coef as f64 * phase;
                            grad[index as usize][1] += slope * coef as f64 * (1.0 - phase);
                        }
                    }
                    grad
                })
            })
            .collect();
        handles.into_iter().map(|handle| handle.join().unwrap()).collect()
    });

    let mut grad = vec![[0.0; 2]; PARAM_COUNT];
    for partial in partials {
        for (total, part) in grad.iter_mut().zip(partial) {
            total[0] += part[0];
            total[1] += part[1];
        }
    }
    grad
}

/// The sigmoid scale that best fits the current evaluation to the results.
fn best_k(samples: &[Sample], params: &[[f64; 2]]) -> f64 {
    let (mut low, mut high) = (0.1, 3.0);
    for _ in 0..40 {
        let a = low + (high - low) / 3.0;
        let b = high - (high - low) / 3.0;
        if error(samples, params, a) < error(samples, params, b) {
            high = b;
        } else {
            low = a;
        }
    }
    (low + high) / 2.0
}

/// Fits the evaluation weights to the labeled positions in `data_path` with
/// Adam and writes the tuned weights as Rust constants to `out_path`.
pub fn tune(data_path: &str, epochs: usize, out_path: &str) -> std::io::Result<()> {
    let move_gen = MoveGenerator::new();
    let mut samples = Vec::new();
    for line in BufReader::new(File::open(data_path)?).lines() {
        let line = line?;
        let Some((fen, result)) = line.split_once(';') else { continue };
        let Ok(result) = result.trim().parse::<f64>() else { continue };
        let board = Board::new(fen);
        samples.push(Sample { trace: tuning::trace(&board, &move_gen.lookup), result });
    }
    eprintln!("tune: loaded {} positions", samples.len());

    let mut params: Vec<[f64; 2]> =
        tuning::current_params().iter().map(|&(op, eg)| [op as f64, eg as f64]).collect();
    let k = best_k(&samples, &params);
    eprintln!("tune: K = {:.4}, initial error = {:.6}", k, error(&samples, &params, k));

    const LEARNING_RATE: f64 = 1.0;
    const BETA1: f64 = 0.9;
    const BETA2: f64 = 0.999;
    const EPSILON: f64 = 1e-8;
    let mut momentum = vec![[0.0; 2]; PARAM_COUNT];
    let mut velocity = vec![[0.0; 2]; PARAM_COUNT];

    for epoch in 1..=epochs {
        let grad = gradient(&samples, &params, k);
        for i in FIRST_TUNED_PARAM..PARAM_COUNT {
            for phase in 0..2 {
                let g = grad[i][phase] / samples.len() as f64;
                momentum[i][phase] = BETA1 * momentum[i][phase] + (1.0 - BETA1) * g;
                velocity[i][phase] = BETA2 * velocity[i][phase] + (1.0 - BETA2) * g * g;
                let m = momentum[i][phase] / (1.0 - BETA1.powi(epoch as i32));
                let v = velocity[i][phase] / (1.0 - BETA2.powi(epoch as i32));
                params[i][phase] -= LEARNING_RATE * m / (v.sqrt() + EPSILON);
            }
        }
        if epoch % 50 == 0 || epoch == epochs {
            eprintln!("tune: epoch {} error = {:.6}", epoch, error(&samples, &params, k));
        }
    }

    let rounded: Vec<(i32, i32)> =
        params.iter().map(|p| (p[0].round() as i32, p[1].round() as i32)).collect();
    let mut out = File::create(out_path)?;
    out.write_all(tuning::format_params(&rounded).as_bytes())
}
