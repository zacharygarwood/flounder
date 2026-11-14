//! End-to-end tests of the UCI protocol, driving the built binary over
//! stdin/stdout exactly as a GUI or lichess-bot does.
//!
//! Timing assertions use generous margins so they hold on slow, shared CI
//! machines; they exist to catch hangs and gross clock overruns, not to
//! measure precision.

use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::sync::{Mutex, MutexGuard};
use std::time::{Duration, Instant};

/// Held for each engine's lifetime so tests run one engine at a time: a
/// searching engine would otherwise steal CPU from another test's timing.
static ONE_ENGINE_AT_A_TIME: Mutex<()> = Mutex::new(());

struct Engine {
    _turn: MutexGuard<'static, ()>,
    child: Child,
    stdin: Option<ChildStdin>,
    output: Receiver<String>,
}

impl Engine {
    /// Starts the engine and completes the UCI handshake, so timings measured
    /// afterwards exclude start-up.
    fn start() -> Self {
        // A test that panicked while holding the lock leaves it poisoned; the
        // lock guards no data, so later tests can still take their turn.
        let turn = ONE_ENGINE_AT_A_TIME.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        let mut child = Command::new(env!("CARGO_BIN_EXE_flounder"))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .expect("engine binary should start");
        let stdout = child.stdout.take().unwrap();
        let (sender, output) = mpsc::channel();
        std::thread::spawn(move || {
            for line in BufReader::new(stdout).lines().map_while(Result::ok) {
                if sender.send(line).is_err() {
                    break;
                }
            }
        });
        let mut engine = Self { _turn: turn, stdin: child.stdin.take(), child, output };
        engine.send("uci");
        engine.wait_for("uciok", Duration::from_secs(10));
        engine.send("isready");
        engine.wait_for("readyok", Duration::from_secs(10));
        engine
    }

    fn send(&mut self, command: &str) {
        let stdin = self.stdin.as_mut().expect("stdin is open");
        writeln!(stdin, "{command}").unwrap();
        stdin.flush().unwrap();
    }

    /// The next line starting with `prefix`, skipping others, and how long it
    /// took to arrive. Panics if none arrives within `timeout`.
    fn wait_for(&mut self, prefix: &str, timeout: Duration) -> (String, Duration) {
        let start = Instant::now();
        loop {
            let left = timeout.saturating_sub(start.elapsed());
            match self.output.recv_timeout(left) {
                Ok(line) if line.starts_with(prefix) => return (line, start.elapsed()),
                Ok(_) => {}
                Err(RecvTimeoutError::Timeout) => panic!("no '{prefix}' line within {timeout:?}"),
                Err(RecvTimeoutError::Disconnected) => panic!("engine exited before sending '{prefix}'"),
            }
        }
    }

    /// Asserts that no line starting with `prefix` arrives for `duration`.
    fn assert_silent(&mut self, prefix: &str, duration: Duration) {
        let start = Instant::now();
        while let Some(left) = duration.checked_sub(start.elapsed()) {
            match self.output.recv_timeout(left) {
                Ok(line) => assert!(!line.starts_with(prefix), "unexpected '{line}'"),
                Err(_) => return,
            }
        }
    }

    fn best_move(line: &str) -> &str {
        line.split_whitespace().nth(1).expect("bestmove names a move")
    }
}

impl Drop for Engine {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[test]
fn handshake_advertises_ponder() {
    let mut engine = Engine::start();
    engine.send("uci");
    engine.wait_for("option name Ponder type check", Duration::from_secs(5));
}

#[test]
fn finds_mate_in_one() {
    let mut engine = Engine::start();
    engine.send("position fen 6k1/5ppp/8/8/8/8/5PPP/3R2K1 w - - 0 1");
    engine.send("go depth 4");
    let (line, _) = engine.wait_for("bestmove", Duration::from_secs(10));
    assert_eq!(Engine::best_move(&line), "d1d8");
}

#[test]
fn plays_from_a_position_reached_by_a_move_list() {
    // After 1. f3 e5 2. g4, Black mates with Qh4.
    let mut engine = Engine::start();
    engine.send("position startpos moves f2f3 e7e5 g2g4");
    engine.send("go depth 4");
    let (line, _) = engine.wait_for("bestmove", Duration::from_secs(10));
    assert_eq!(Engine::best_move(&line), "d8h4");
}

#[test]
fn stop_ends_an_infinite_search_promptly() {
    let mut engine = Engine::start();
    engine.send("position startpos");
    engine.send("go infinite");
    engine.assert_silent("bestmove", Duration::from_millis(300));
    engine.send("stop");
    let (line, elapsed) = engine.wait_for("bestmove", Duration::from_secs(2));
    assert!(elapsed < Duration::from_millis(500), "stop took {elapsed:?}");
    assert_ne!(Engine::best_move(&line), "0000");
}

#[test]
fn ponder_search_waits_for_ponderhit_then_uses_its_clock() {
    let mut engine = Engine::start();
    engine.send("position startpos moves e2e4 e7e5");
    engine.send("go ponder wtime 3000 btime 3000");
    // Pondering must not answer on its own, however long it runs.
    engine.assert_silent("bestmove", Duration::from_millis(500));
    engine.send("ponderhit");
    // 3s on the clock allows ~100ms soft / ~300ms hard for this move.
    let (_, elapsed) = engine.wait_for("bestmove", Duration::from_secs(3));
    assert!(elapsed < Duration::from_millis(1000), "took {elapsed:?} after ponderhit");
}

#[test]
fn stop_ends_a_ponder_search() {
    let mut engine = Engine::start();
    engine.send("position startpos moves e2e4");
    engine.send("go ponder wtime 60000 btime 60000");
    engine.assert_silent("bestmove", Duration::from_millis(300));
    engine.send("stop");
    let (_, elapsed) = engine.wait_for("bestmove", Duration::from_secs(2));
    assert!(elapsed < Duration::from_millis(500), "stop took {elapsed:?}");
}

#[test]
fn movetime_is_respected() {
    let mut engine = Engine::start();
    engine.send("position startpos");
    engine.send("go movetime 300");
    let (_, elapsed) = engine.wait_for("bestmove", Duration::from_secs(5));
    assert!(elapsed < Duration::from_millis(800), "took {elapsed:?} for movetime 300");
}

#[test]
fn moves_quickly_when_the_clock_is_nearly_out() {
    let mut engine = Engine::start();
    engine.send("position startpos moves e2e4");
    engine.send("go wtime 60 btime 60");
    let (line, elapsed) = engine.wait_for("bestmove", Duration::from_secs(5));
    assert!(elapsed < Duration::from_millis(250), "took {elapsed:?} with 60ms left");
    assert_ne!(Engine::best_move(&line), "0000");
}

#[test]
fn reports_no_move_when_there_is_none() {
    // Black is checkmated.
    let mut engine = Engine::start();
    engine.send("position fen 3R2k1/5ppp/8/8/8/8/5PPP/6K1 b - - 0 1");
    engine.send("go depth 3");
    let (line, _) = engine.wait_for("bestmove", Duration::from_secs(5));
    assert_eq!(Engine::best_move(&line), "0000");
}

#[test]
fn exits_when_stdin_closes_mid_search() {
    let mut engine = Engine::start();
    engine.send("position startpos");
    engine.send("go infinite");
    engine.stdin.take(); // close stdin, as when the GUI goes away
    let start = Instant::now();
    while start.elapsed() < Duration::from_secs(3) {
        if engine.child.try_wait().unwrap().is_some() {
            return;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    panic!("engine still running 3s after stdin closed");
}
