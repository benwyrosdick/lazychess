use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Widget},
};
use shakmaty::{san::San, uci::UciMove, Chess, Position};

use crate::engine::{format_nodes, format_score, AnalysisInfo};

/// Analysis information for display
#[derive(Debug, Clone, Default)]
pub struct AnalysisState {
    /// Target depth for analysis
    pub target_depth: u32,
    /// Current analysis lines (one per MultiPV)
    pub lines: Vec<AnalysisInfo>,
    /// Is analysis currently running?
    pub is_running: bool,
    /// Is analysis paused?
    pub is_paused: bool,
    /// Latest nodes count
    pub nodes: Option<u64>,
    /// Latest NPS
    pub nps: Option<u64>,
    /// Hash usage
    pub hashfull: Option<u32>,
}

impl AnalysisState {
    pub fn new(target_depth: u32) -> Self {
        Self {
            target_depth,
            lines: Vec::new(),
            is_running: false,
            is_paused: false,
            nodes: None,
            nps: None,
            hashfull: None,
        }
    }

    /// Update with new analysis info
    pub fn update(&mut self, info: AnalysisInfo) {
        // Update overall stats
        if info.nodes.is_some() {
            self.nodes = info.nodes;
        }
        if info.nps.is_some() {
            self.nps = info.nps;
        }
        if info.hashfull.is_some() {
            self.hashfull = info.hashfull;
        }

        // Update the appropriate line based on MultiPV
        let line_idx = info.multipv.unwrap_or(1).saturating_sub(1) as usize;

        // Only update if we have a PV (principal variation)
        if !info.pv.is_empty() {
            // Ensure we have enough slots
            while self.lines.len() <= line_idx {
                self.lines.push(AnalysisInfo::default());
            }
            self.lines[line_idx] = info;
        }
    }

    /// Clear analysis state
    pub fn clear(&mut self) {
        self.lines.clear();
        self.nodes = None;
        self.nps = None;
        self.hashfull = None;
    }
}

/// Convert a list of UCI move strings to SAN notation given a starting position
fn uci_to_san(position: &Chess, uci_moves: &[String]) -> Vec<String> {
    let mut pos = position.clone();
    let mut san_moves = Vec::new();

    for uci_str in uci_moves {
        // Parse UCI move string
        let Ok(uci_move) = uci_str.parse::<UciMove>() else {
            break;
        };

        // Convert to a legal move in this position
        let Ok(m) = uci_move.to_move(&pos) else {
            break;
        };

        // Convert to SAN
        let san = San::from_move(&pos, &m);
        san_moves.push(san.to_string());

        // Apply the move to track position for next move
        let Ok(new_pos) = pos.play(&m) else {
            break;
        };
        pos = new_pos;
    }

    san_moves
}

/// Analysis panel widget
pub struct AnalysisWidget<'a> {
    state: &'a AnalysisState,
    position: &'a Chess,
    multipv: u32,
}

impl<'a> AnalysisWidget<'a> {
    pub fn new(state: &'a AnalysisState, position: &'a Chess, multipv: u32) -> Self {
        Self { state, position, multipv }
    }
}

impl Widget for AnalysisWidget<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let status = if self.state.is_paused {
            " Analysis (PAUSED) "
        } else if self.state.is_running {
            " Analysis "
        } else {
            " Analysis (stopped) "
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .title(status);

        let inner = block.inner(area);
        block.render(area, buf);

        if inner.height < 3 {
            return;
        }

        let mut lines: Vec<Line> = Vec::new();

        // Depth and status line
        let current_depth = self
            .state
            .lines
            .first()
            .and_then(|l| l.depth)
            .unwrap_or(0);

        let depth_line = Line::from(vec![
            Span::styled("Depth: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("{}/{}", current_depth, self.state.target_depth),
                Style::default().fg(Color::White),
            ),
        ]);
        lines.push(depth_line);

        // Main evaluation (from first line)
        if let Some(first_line) = self.state.lines.first() {
            let score = format_score(first_line.score_cp, first_line.score_mate);
            let score_color = if first_line.score_mate.is_some() {
                Color::Yellow
            } else if let Some(cp) = first_line.score_cp {
                if cp > 100 {
                    Color::Green
                } else if cp < -100 {
                    Color::Red
                } else {
                    Color::White
                }
            } else {
                Color::White
            };

            let eval_line = Line::from(vec![
                Span::styled("Eval: ", Style::default().fg(Color::DarkGray)),
                Span::styled(score, Style::default().fg(score_color).add_modifier(Modifier::BOLD)),
            ]);
            lines.push(eval_line);
        } else {
            lines.push(Line::from(vec![
                Span::styled("Eval: ", Style::default().fg(Color::DarkGray)),
                Span::styled("---", Style::default().fg(Color::DarkGray)),
            ]));
        }

        lines.push(Line::from(""));

        // Best lines header
        lines.push(Line::from(Span::styled(
            "Best lines:",
            Style::default().fg(Color::Cyan),
        )));

        // Show each PV line
        for (idx, info) in self.state.lines.iter().take(self.multipv as usize).enumerate() {
            let score = format_score(info.score_cp, info.score_mate);

            // Convert UCI moves to SAN notation
            let san_moves = uci_to_san(self.position, &info.pv);
            
            // Format PV moves (show first few moves)
            let pv_str: String = san_moves
                .iter()
                .take(6)
                .cloned()
                .collect::<Vec<_>>()
                .join(" ");

            let pv_display = if pv_str.len() > 24 {
                format!("{}...", &pv_str[..24])
            } else {
                pv_str
            };

            let line = Line::from(vec![
                Span::styled(
                    format!("{}. ", idx + 1),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled(
                    format!("{:>6} ", score),
                    Style::default().fg(Color::Yellow),
                ),
                Span::styled(pv_display, Style::default().fg(Color::White)),
            ]);
            lines.push(line);
        }

        // Fill empty lines if we don't have enough PVs yet
        for idx in self.state.lines.len()..self.multipv as usize {
            lines.push(Line::from(vec![
                Span::styled(
                    format!("{}. ", idx + 1),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled("---", Style::default().fg(Color::DarkGray)),
            ]));
        }

        lines.push(Line::from(""));

        // Stats line
        let nodes_str = self
            .state
            .nodes
            .map(format_nodes)
            .unwrap_or_else(|| "---".to_string());
        let nps_str = self
            .state
            .nps
            .map(format_nodes)
            .unwrap_or_else(|| "---".to_string());

        let stats_line = Line::from(vec![
            Span::styled("Nodes: ", Style::default().fg(Color::DarkGray)),
            Span::styled(format!("{} ", nodes_str), Style::default().fg(Color::White)),
            Span::styled("NPS: ", Style::default().fg(Color::DarkGray)),
            Span::styled(nps_str, Style::default().fg(Color::White)),
        ]);
        lines.push(stats_line);

        // Hash usage
        if let Some(hashfull) = self.state.hashfull {
            let hash_percent = hashfull as f64 / 10.0;
            let hash_line = Line::from(vec![
                Span::styled("Hash: ", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    format!("{:.1}%", hash_percent),
                    Style::default().fg(Color::White),
                ),
            ]);
            lines.push(hash_line);
        }

        let paragraph = Paragraph::new(lines);
        paragraph.render(inner, buf);
    }
}
