use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Widget},
};

/// Help popup widget
pub struct HelpPopup;

impl HelpPopup {
    pub fn new() -> Self {
        Self
    }

    /// Calculate centered popup area
    pub fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
        let popup_width = r.width * percent_x / 100;
        let popup_height = r.height * percent_y / 100;

        let x = r.x + (r.width - popup_width) / 2;
        let y = r.y + (r.height - popup_height) / 2;

        Rect::new(x, y, popup_width, popup_height)
    }
}

impl Widget for HelpPopup {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Clear the background
        Clear.render(area, buf);

        let block = Block::default()
            .borders(Borders::ALL)
            .title(" Help ")
            .title_alignment(Alignment::Center)
            .style(Style::default().bg(Color::DarkGray));

        let inner = block.inner(area);
        block.render(area, buf);

        let help_sections = vec![
            ("Navigation", vec![
                ("←, h", "Previous move"),
                ("→, l", "Next move"),
                ("Home", "Go to start"),
                ("End", "Go to latest position"),
            ]),
            ("Analysis", vec![
                ("p", "Pause/resume analysis"),
                ("d", "Change search depth"),
                ("m", "Change MultiPV (number of lines)"),
                ("1-9", "Play move from analysis line N"),
            ]),
            ("Import/Export", vec![
                ("i", "Import FEN or PGN"),
                (":fen <FEN>", "Load position from FEN"),
                (":pgn", "Enter PGN input mode"),
                ("y", "Copy current FEN to clipboard"),
            ]),
            ("Display", vec![
                ("f", "Flip board"),
                ("?", "Toggle this help"),
            ]),
            ("General", vec![
                ("Enter, :", "Enter command/move mode"),
                ("Esc", "Cancel input / close popup"),
                ("q, Ctrl+C", "Quit"),
            ]),
        ];

        let mut lines: Vec<Line> = Vec::new();

        for (section, shortcuts) in help_sections {
            // Section header
            lines.push(Line::from(Span::styled(
                section,
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )));

            for (key, desc) in shortcuts {
                lines.push(Line::from(vec![
                    Span::styled(
                        format!("  {:12}", key),
                        Style::default().fg(Color::Yellow),
                    ),
                    Span::styled(desc, Style::default().fg(Color::White)),
                ]));
            }

            lines.push(Line::from(""));
        }

        // Remove last empty line
        lines.pop();

        let paragraph = Paragraph::new(lines);
        paragraph.render(inner, buf);
    }
}

/// Depth input popup
pub struct DepthPopup {
    current_depth: u32,
    input: String,
}

impl DepthPopup {
    pub fn new(current_depth: u32, input: &str) -> Self {
        Self {
            current_depth,
            input: input.to_string(),
        }
    }
}

impl Widget for DepthPopup {
    fn render(self, area: Rect, buf: &mut Buffer) {
        Clear.render(area, buf);

        let block = Block::default()
            .borders(Borders::ALL)
            .title(" Set Depth ")
            .title_alignment(Alignment::Center)
            .style(Style::default().bg(Color::DarkGray));

        let inner = block.inner(area);
        block.render(area, buf);

        let lines = vec![
            Line::from(vec![
                Span::styled("Current: ", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    self.current_depth.to_string(),
                    Style::default().fg(Color::White),
                ),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("New depth: ", Style::default().fg(Color::Cyan)),
                Span::styled(&self.input, Style::default().fg(Color::White)),
                Span::styled("_", Style::default().fg(Color::White).bg(Color::White)),
            ]),
            Line::from(""),
            Line::from(Span::styled(
                "Enter to confirm, Esc to cancel",
                Style::default().fg(Color::DarkGray),
            )),
        ];

        let paragraph = Paragraph::new(lines);
        paragraph.render(inner, buf);
    }
}

/// MultiPV input popup
pub struct MultiPVPopup {
    current_multipv: u32,
    input: String,
}

impl MultiPVPopup {
    pub fn new(current_multipv: u32, input: &str) -> Self {
        Self {
            current_multipv,
            input: input.to_string(),
        }
    }
}

impl Widget for MultiPVPopup {
    fn render(self, area: Rect, buf: &mut Buffer) {
        Clear.render(area, buf);

        let block = Block::default()
            .borders(Borders::ALL)
            .title(" Set MultiPV ")
            .title_alignment(Alignment::Center)
            .style(Style::default().bg(Color::DarkGray));

        let inner = block.inner(area);
        block.render(area, buf);

        let lines = vec![
            Line::from(vec![
                Span::styled("Current: ", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    format!("{} lines", self.current_multipv),
                    Style::default().fg(Color::White),
                ),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Number of lines: ", Style::default().fg(Color::Cyan)),
                Span::styled(&self.input, Style::default().fg(Color::White)),
                Span::styled("_", Style::default().fg(Color::White).bg(Color::White)),
            ]),
            Line::from(""),
            Line::from(Span::styled(
                "Enter to confirm, Esc to cancel",
                Style::default().fg(Color::DarkGray),
            )),
        ];

        let paragraph = Paragraph::new(lines);
        paragraph.render(inner, buf);
    }
}

/// Import popup for FEN/PGN selection
pub struct ImportPopup;

impl ImportPopup {
    pub fn new() -> Self {
        Self
    }
}

impl Widget for ImportPopup {
    fn render(self, area: Rect, buf: &mut Buffer) {
        Clear.render(area, buf);

        let block = Block::default()
            .borders(Borders::ALL)
            .title(" Import ")
            .title_alignment(Alignment::Center)
            .style(Style::default().bg(Color::DarkGray));

        let inner = block.inner(area);
        block.render(area, buf);

        let lines = vec![
            Line::from(Span::styled(
                "Choose import format:",
                Style::default().fg(Color::White),
            )),
            Line::from(""),
            Line::from(vec![
                Span::styled("  [f] ", Style::default().fg(Color::Yellow)),
                Span::styled("FEN - Position string", Style::default().fg(Color::White)),
            ]),
            Line::from(vec![
                Span::styled("  [p] ", Style::default().fg(Color::Yellow)),
                Span::styled("PGN - Game notation", Style::default().fg(Color::White)),
            ]),
            Line::from(vec![
                Span::styled("  [n] ", Style::default().fg(Color::Yellow)),
                Span::styled("New game - Start fresh", Style::default().fg(Color::White)),
            ]),
            Line::from(""),
            Line::from(Span::styled(
                "Esc to cancel",
                Style::default().fg(Color::DarkGray),
            )),
        ];

        let paragraph = Paragraph::new(lines);
        paragraph.render(inner, buf);
    }
}
