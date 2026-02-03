#![allow(dead_code)]

use anyhow::{bail, Context, Result};
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::Duration;
use vampirc_uci::{parse_one, UciInfoAttribute, UciMessage, UciOptionConfig};

/// Analysis information from the engine
#[derive(Debug, Clone, Default)]
pub struct AnalysisInfo {
    /// Current search depth
    pub depth: Option<u32>,
    /// Selective depth
    pub seldepth: Option<u32>,
    /// Score in centipawns (positive = white advantage)
    pub score_cp: Option<i32>,
    /// Mate in N moves (positive = white mates, negative = black mates)
    pub score_mate: Option<i32>,
    /// Nodes searched
    pub nodes: Option<u64>,
    /// Nodes per second
    pub nps: Option<u64>,
    /// Time spent in milliseconds
    pub time_ms: Option<u64>,
    /// MultiPV line number (1-indexed)
    pub multipv: Option<u32>,
    /// Principal variation (best line) in UCI notation
    pub pv: Vec<String>,
    /// Hash table usage (per mille)
    pub hashfull: Option<u32>,
}

/// Best move result from engine
#[derive(Debug, Clone)]
pub struct BestMove {
    pub best_move: String,
    pub ponder: Option<String>,
}

/// Messages from the engine to the UI
#[derive(Debug, Clone)]
pub enum EngineEvent {
    /// Engine is ready
    Ready,
    /// Analysis info update
    Info(AnalysisInfo),
    /// Best move found
    BestMove(BestMove),
    /// Engine error
    Error(String),
    /// Engine identification
    Id {
        name: Option<String>,
        author: Option<String>,
    },
    /// Engine options available
    Option(String),
}

/// UCI Engine wrapper
pub struct Engine {
    process: Child,
    stdin: ChildStdin,
    event_rx: Receiver<EngineEvent>,
    /// Is engine currently analyzing?
    is_analyzing: bool,
    /// Engine name
    pub name: Option<String>,
    /// Engine author
    pub author: Option<String>,
}

impl Engine {
    /// Start a new engine process
    pub fn new(path: &str) -> Result<Self> {
        let mut process = Command::new(path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .with_context(|| format!("Failed to start engine at: {}", path))?;

        let stdin = process.stdin.take().context("Failed to get engine stdin")?;
        let stdout = process
            .stdout
            .take()
            .context("Failed to get engine stdout")?;

        // Create channel for engine events
        let (event_tx, event_rx) = mpsc::channel();

        // Spawn reader thread
        thread::spawn(move || {
            Self::read_output(stdout, event_tx);
        });

        let mut engine = Self {
            process,
            stdin,
            event_rx,
            is_analyzing: false,
            name: None,
            author: None,
        };

        // Initialize UCI
        engine.send_command("uci")?;

        // Wait for uciok
        engine.wait_for_ready(Duration::from_secs(5))?;

        Ok(engine)
    }

    /// Read engine output in a separate thread
    fn read_output(stdout: ChildStdout, tx: Sender<EngineEvent>) {
        let reader = BufReader::new(stdout);

        for line in reader.lines() {
            let line = match line {
                Ok(l) => l,
                Err(_) => break,
            };

            if line.is_empty() {
                continue;
            }

            let msg = parse_one(&line);

            match msg {
                UciMessage::Id { name, author } => {
                    let _ = tx.send(EngineEvent::Id { name, author });
                }
                UciMessage::UciOk => {
                    let _ = tx.send(EngineEvent::Ready);
                }
                UciMessage::ReadyOk => {
                    let _ = tx.send(EngineEvent::Ready);
                }
                UciMessage::Info(attrs) => {
                    let info = Self::parse_info(attrs);
                    let _ = tx.send(EngineEvent::Info(info));
                }
                UciMessage::BestMove { best_move, ponder } => {
                    let _ = tx.send(EngineEvent::BestMove(BestMove {
                        best_move: best_move.to_string(),
                        ponder: ponder.map(|p| p.to_string()),
                    }));
                }
                UciMessage::Option(opt) => {
                    let opt_name = match opt {
                        UciOptionConfig::Check { name, .. } => name,
                        UciOptionConfig::Spin { name, .. } => name,
                        UciOptionConfig::Combo { name, .. } => name,
                        UciOptionConfig::Button { name } => name,
                        UciOptionConfig::String { name, .. } => name,
                    };
                    let _ = tx.send(EngineEvent::Option(opt_name));
                }
                _ => {}
            }
        }
    }

    /// Parse info attributes into AnalysisInfo
    fn parse_info(attrs: Vec<UciInfoAttribute>) -> AnalysisInfo {
        let mut info = AnalysisInfo::default();

        for attr in attrs {
            match attr {
                UciInfoAttribute::Depth(d) => info.depth = Some(d as u32),
                UciInfoAttribute::SelDepth(d) => info.seldepth = Some(d as u32),
                UciInfoAttribute::Score { cp, mate, .. } => {
                    info.score_cp = cp.map(|c| c as i32);
                    info.score_mate = mate.map(|m| m as i32);
                }
                UciInfoAttribute::Nodes(n) => info.nodes = Some(n),
                UciInfoAttribute::Nps(n) => info.nps = Some(n),
                UciInfoAttribute::Time(t) => info.time_ms = Some(t.num_milliseconds() as u64),
                UciInfoAttribute::MultiPv(n) => info.multipv = Some(n as u32),
                UciInfoAttribute::Pv(moves) => {
                    info.pv = moves.iter().map(|m| m.to_string()).collect();
                }
                UciInfoAttribute::HashFull(h) => info.hashfull = Some(h as u32),
                _ => {}
            }
        }

        info
    }

    /// Send a raw command to the engine
    pub fn send_command(&mut self, cmd: &str) -> Result<()> {
        writeln!(self.stdin, "{}", cmd).context("Failed to write to engine")?;
        self.stdin.flush().context("Failed to flush engine stdin")?;
        Ok(())
    }

    /// Wait for the engine to be ready
    pub fn wait_for_ready(&mut self, timeout: Duration) -> Result<()> {
        let deadline = std::time::Instant::now() + timeout;

        while std::time::Instant::now() < deadline {
            match self.event_rx.recv_timeout(Duration::from_millis(100)) {
                Ok(EngineEvent::Ready) => return Ok(()),
                Ok(EngineEvent::Id { name, author }) => {
                    self.name = name;
                    self.author = author;
                }
                Ok(_) => continue,
                Err(mpsc::RecvTimeoutError::Timeout) => continue,
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    bail!("Engine process terminated unexpectedly");
                }
            }
        }

        bail!("Timeout waiting for engine to be ready");
    }

    /// Set an engine option
    pub fn set_option(&mut self, name: &str, value: &str) -> Result<()> {
        self.send_command(&format!("setoption name {} value {}", name, value))
    }

    /// Set up the position from FEN or startpos
    pub fn set_position(&mut self, fen: Option<&str>, moves: &[String]) -> Result<()> {
        let pos_str = match fen {
            Some(f) => format!("position fen {}", f),
            None => "position startpos".to_string(),
        };

        let cmd = if moves.is_empty() {
            pos_str
        } else {
            format!("{} moves {}", pos_str, moves.join(" "))
        };

        self.send_command(&cmd)
    }

    /// Start analysis with infinite time
    pub fn go_infinite(&mut self) -> Result<()> {
        self.is_analyzing = true;
        self.send_command("go infinite")
    }

    /// Start analysis with depth limit
    pub fn go_depth(&mut self, depth: u32) -> Result<()> {
        self.is_analyzing = true;
        self.send_command(&format!("go depth {}", depth))
    }

    /// Stop analysis
    pub fn stop(&mut self) -> Result<()> {
        if self.is_analyzing {
            self.send_command("stop")?;
            self.is_analyzing = false;
        }
        Ok(())
    }

    /// Check if currently analyzing
    pub fn is_analyzing(&self) -> bool {
        self.is_analyzing
    }

    /// Try to receive an event (non-blocking)
    pub fn try_recv(&mut self) -> Option<EngineEvent> {
        match self.event_rx.try_recv() {
            Ok(event) => {
                // Update analyzing state on bestmove
                if matches!(event, EngineEvent::BestMove(_)) {
                    self.is_analyzing = false;
                }
                Some(event)
            }
            Err(_) => None,
        }
    }

    /// Send new game notification
    pub fn new_game(&mut self) -> Result<()> {
        self.send_command("ucinewgame")
    }

    /// Quit the engine
    pub fn quit(&mut self) -> Result<()> {
        self.send_command("quit")?;
        let _ = self.process.wait();
        Ok(())
    }
}

impl Drop for Engine {
    fn drop(&mut self) {
        let _ = self.quit();
    }
}

/// Format score for display
pub fn format_score(cp: Option<i32>, mate: Option<i32>) -> String {
    if let Some(m) = mate {
        if m > 0 {
            format!("M{}", m)
        } else {
            format!("-M{}", -m)
        }
    } else if let Some(c) = cp {
        let score = c as f64 / 100.0;
        if score >= 0.0 {
            format!("+{:.2}", score)
        } else {
            format!("{:.2}", score)
        }
    } else {
        "---".to_string()
    }
}

/// Format nodes count for display
pub fn format_nodes(nodes: u64) -> String {
    if nodes >= 1_000_000_000 {
        format!("{:.1}B", nodes as f64 / 1_000_000_000.0)
    } else if nodes >= 1_000_000 {
        format!("{:.1}M", nodes as f64 / 1_000_000.0)
    } else if nodes >= 1_000 {
        format!("{:.1}K", nodes as f64 / 1_000.0)
    } else {
        format!("{}", nodes)
    }
}
