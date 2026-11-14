//! Texel tuning: fits the evaluation weights so that a sigmoid of the
//! evaluation best predicts game results on a set of labeled quiet positions.
//!
//! Self-play data from this engine proved too biased to tune on (it lost Elo
//! twice), so this reads an external dataset in the zurichess
//! `quiet-labeled.epd` format: `<fen fields> c9 "1-0";`.

use crate::bitboard::{BitboardIterator, SQUARES};
use crate::board::Board;
use crate::eval::{self, feature, ENDGAME_TABLES, OPENING_TABLES, WEIGHTS};
use crate::lookup::LookupTable;
use crate::pieces::{Color, Piece, PIECE_COUNT};
use std::fs::File;
use std::io::{BufRead, BufReader, Write};

const PST_PARAMS: usize = PIECE_COUNT * SQUARES as usize;
const PARAM_COUNT: usize = PST_PARAMS + feature::COUNT;

/// Which weights the tuner may change.
#[derive(Clone, Copy)]
pub enum Scope {
    /// Every weight, including the material/piece-square tables.
    All,
    /// Only the positional features; the piece-square tables stay fixed.
    Positional,
}

impl Scope {
    fn first_tuned_param(self) -> usize {
        match self {
            Scope::All => 0,
            Scope::Positional => PST_PARAMS,
        }
    }
}

/// A position's evaluation as a linear function of the weights: how many
/// times (White minus Black) each weight applies, plus the parts that are not
/// linear in any weight.
struct Trace {
    /// (weight index, net White-minus-Black coefficient)
    features: Vec<(u16, i16)>,
    phase: i32,
    /// White-relative opening score from terms that are not tuned.
    fixed_opening: i32,
}

fn trace(board: &Board, lookup: &LookupTable) -> Trace {
    let mut coefs = vec![0i32; PARAM_COUNT];
    for (color, sign) in [(Color::White, 1), (Color::Black, -1)] {
        for piece in [Piece::Pawn, Piece::Knight, Piece::Bishop, Piece::Rook, Piece::Queen, Piece::King] {
            for square in BitboardIterator::new(board.bb(color, piece)) {
                let table_index = if color == Color::White { square ^ 56 } else { square };
                coefs[piece.index() * SQUARES as usize + table_index as usize] += sign;
            }
        }
    }
    let features = eval::features(board, lookup);
    coefs[PST_PARAMS..].copy_from_slice(&features.counts);

    Trace {
        features: coefs
            .iter()
            .enumerate()
            .filter(|(_, &coef)| coef != 0)
            .map(|(index, &coef)| (index as u16, coef as i16))
            .collect(),
        phase: board.phase.min(24),
        fixed_opening: features.king_danger,
    }
}

fn current_params() -> Vec<[f64; 2]> {
    let mut params = Vec::with_capacity(PARAM_COUNT);
    for piece in 0..PIECE_COUNT {
        for square in 0..SQUARES as usize {
            params.push([OPENING_TABLES[piece][square] as f64, ENDGAME_TABLES[piece][square] as f64]);
        }
    }
    params.extend(WEIGHTS.iter().map(|&(op, eg)| [op as f64, eg as f64]));
    params
}

struct Sample {
    trace: Trace,
    /// Game result for White: 1, 0.5 or 0.
    result: f64,
}

/// Parses one dataset line into a FEN and White's result.
fn parse_line(line: &str) -> Option<(String, f64)> {
    let (position, label) = line.split_once(" c9 ")?;
    let result = match label.trim().trim_end_matches(';').trim_matches('"') {
        "1-0" => 1.0,
        "0-1" => 0.0,
        "1/2-1/2" => 0.5,
        _ => return None,
    };
    Some((format!("{} 0 1", position.trim()), result))
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

/// Fits the weights in `scope` to the labeled positions in `data_path` with
/// Adam and writes all weights as Rust constants to `out_path`.
pub fn tune(data_path: &str, epochs: usize, scope: Scope, out_path: &str) -> std::io::Result<()> {
    let lookup = LookupTable::init();
    let mut samples = Vec::new();
    for line in BufReader::new(File::open(data_path)?).lines() {
        let Some((fen, result)) = parse_line(&line?) else { continue };
        samples.push(Sample { trace: trace(&Board::new(&fen), &lookup), result });
    }
    eprintln!("tune: loaded {} positions", samples.len());

    let mut params = current_params();
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
        for i in scope.first_tuned_param()..PARAM_COUNT {
            for phase in 0..2 {
                let g = grad[i][phase] / samples.len() as f64;
                momentum[i][phase] = BETA1 * momentum[i][phase] + (1.0 - BETA1) * g;
                velocity[i][phase] = BETA2 * velocity[i][phase] + (1.0 - BETA2) * g * g;
                let m = momentum[i][phase] / (1.0 - BETA1.powi(epoch as i32));
                let v = velocity[i][phase] / (1.0 - BETA2.powi(epoch as i32));
                params[i][phase] -= LEARNING_RATE * m / (v.sqrt() + EPSILON);
            }
        }
        if epoch % 100 == 0 || epoch == epochs {
            eprintln!("tune: epoch {} error = {:.6}", epoch, error(&samples, &params, k));
        }
    }

    let rounded: Vec<(i32, i32)> = params.iter().map(|p| (p[0].round() as i32, p[1].round() as i32)).collect();
    File::create(out_path)?.write_all(format_params(&rounded).as_bytes())
}

/// Formats weights as the Rust constants in `eval.rs`, ready to paste in.
fn format_params(params: &[(i32, i32)]) -> String {
    let names = ["Pawn", "Knight", "Bishop", "Rook", "Queen", "King"];
    let mut out = String::new();
    for (label, phase) in [("OPENING_TABLES", 0), ("ENDGAME_TABLES", 1)] {
        out += &format!("pub const {}: [PST; PIECE_COUNT] = [\n", label);
        for (piece, name) in names.iter().enumerate() {
            out += &format!("    // {}\n    [\n", name);
            for rank in 0..8 {
                let row: Vec<String> = (0..8)
                    .map(|file| {
                        let (op, eg) = params[piece * 64 + rank * 8 + file];
                        format!("{:5}", if phase == 0 { op } else { eg })
                    })
                    .collect();
                out += &format!("       {},\n", row.join(","));
            }
            out += "    ],\n";
        }
        out += "];\n\n";
    }
    out += "pub const WEIGHTS: [(i32, i32); feature::COUNT] = [\n";
    for (op, eg) in &params[PST_PARAMS..] {
        out += &format!("    ({}, {}),\n", op, eg);
    }
    out += "];\n";
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trace_reproduces_the_evaluation() {
        let lookup = LookupTable::init();
        let params = current_params();
        for fen in [
            "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
            "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1",
            "8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1",
            "2rq1rk1/pp1bppbp/2np1np1/8/3NP3/2N1BP2/PPPQ2PP/2KR1B1R w - - 0 1",
            "5rk1/1ppb3p/p1pb4/6q1/3P1p1r/2P1R2P/PP1BQ1P1/5RKN b - - 0 1",
        ] {
            let board = Board::new(fen);
            let trace = trace(&board, &lookup);
            let (mut opening, mut endgame) = (trace.fixed_opening as f64, 0.0);
            for &(index, coef) in &trace.features {
                opening += coef as f64 * params[index as usize][0];
                endgame += coef as f64 * params[index as usize][1];
            }
            let (extra_op, extra_eg) = eval::eval_extras(&board, &lookup);
            assert_eq!(opening as i32, board.pst_mg + extra_op, "opening mismatch for {fen}");
            assert_eq!(endgame as i32, board.pst_eg + extra_eg, "endgame mismatch for {fen}");
        }
    }

    #[test]
    fn parses_dataset_lines() {
        let (fen, result) = parse_line(r#"4Q3/8/8/8/6k1/4K2p/3N4/5q2 b - - c9 "0-1";"#).unwrap();
        assert_eq!(fen, "4Q3/8/8/8/6k1/4K2p/3N4/5q2 b - - 0 1");
        assert_eq!(result, 0.0);
        assert_eq!(parse_line(r#"8/8/8/8/8/8/8/8 w - - c9 "1/2-1/2";"#).unwrap().1, 0.5);
    }
}
