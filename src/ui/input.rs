use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, Borders, Widget},
};

/// Input mode for the application
#[derive(Debug, Clone, PartialEq)]
pub enum InputMode {
    /// Normal mode - keyboard shortcuts active
    Normal,
    /// Typing a move or command
    Command,
    /// Entering FEN string
    Fen,
    /// Entering PGN (multi-line)
    Pgn,
}

/// Input bar state
#[derive(Debug, Clone)]
pub struct InputState {
    /// Current input buffer
    pub buffer: String,
    /// Cursor position
    pub cursor: usize,
    /// Current input mode
    pub mode: InputMode,
    /// Error message to display
    pub error: Option<String>,
    /// Success message to display
    pub message: Option<String>,
    /// PGN buffer (for multi-line input)
    pub pgn_buffer: Vec<String>,
}

impl Default for InputState {
    fn default() -> Self {
        Self {
            buffer: String::new(),
            cursor: 0,
            mode: InputMode::Normal,
            error: None,
            message: None,
            pgn_buffer: Vec::new(),
        }
    }
}

impl InputState {
    /// Insert a character at cursor position
    pub fn insert(&mut self, c: char) {
        self.buffer.insert(self.cursor, c);
        self.cursor += 1;
        self.clear_messages();
    }

    /// Delete character before cursor
    pub fn backspace(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
            self.buffer.remove(self.cursor);
        }
        self.clear_messages();
    }

    /// Delete character at cursor
    pub fn delete(&mut self) {
        if self.cursor < self.buffer.len() {
            self.buffer.remove(self.cursor);
        }
        self.clear_messages();
    }

    /// Move cursor left
    pub fn move_left(&mut self) {
        self.cursor = self.cursor.saturating_sub(1);
    }

    /// Move cursor right
    pub fn move_right(&mut self) {
        self.cursor = (self.cursor + 1).min(self.buffer.len());
    }

    /// Move cursor to start
    pub fn move_start(&mut self) {
        self.cursor = 0;
    }

    /// Move cursor to end
    pub fn move_end(&mut self) {
        self.cursor = self.buffer.len();
    }

    /// Clear the input buffer
    pub fn clear(&mut self) {
        self.buffer.clear();
        self.cursor = 0;
        self.clear_messages();
    }

    /// Take the current buffer content
    pub fn take(&mut self) -> String {
        self.cursor = 0;
        std::mem::take(&mut self.buffer)
    }

    /// Set error message
    pub fn set_error(&mut self, msg: impl Into<String>) {
        self.error = Some(msg.into());
        self.message = None;
    }

    /// Set success message
    pub fn set_message(&mut self, msg: impl Into<String>) {
        self.message = Some(msg.into());
        self.error = None;
    }

    /// Clear all messages
    pub fn clear_messages(&mut self) {
        self.error = None;
        self.message = None;
    }

    /// Enter command mode
    pub fn enter_command_mode(&mut self) {
        self.mode = InputMode::Command;
        self.clear();
    }

    /// Enter FEN mode
    pub fn enter_fen_mode(&mut self) {
        self.mode = InputMode::Fen;
        self.clear();
        self.buffer = ":fen ".to_string();
        self.cursor = self.buffer.len();
    }

    /// Enter PGN mode
    pub fn enter_pgn_mode(&mut self) {
        self.mode = InputMode::Pgn;
        self.clear();
        self.pgn_buffer.clear();
    }

    /// Exit to normal mode
    pub fn exit_mode(&mut self) {
        self.mode = InputMode::Normal;
        self.clear();
        self.pgn_buffer.clear();
    }

    /// Check if in input mode (not normal)
    pub fn is_input_mode(&self) -> bool {
        self.mode != InputMode::Normal
    }
}

/// Input bar widget
pub struct InputWidget<'a> {
    state: &'a InputState,
}

impl<'a> InputWidget<'a> {
    pub fn new(state: &'a InputState) -> Self {
        Self { state }
    }
}

impl Widget for InputWidget<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let block = Block::default().borders(Borders::ALL);
        let inner = block.inner(area);
        block.render(area, buf);

        if inner.width < 3 {
            return;
        }

        // Determine what to show
        let (prefix, content, style) = if let Some(ref err) = self.state.error {
            ("!", err.as_str(), Style::default().fg(Color::Red))
        } else if let Some(ref msg) = self.state.message {
            ("", msg.as_str(), Style::default().fg(Color::Green))
        } else {
            let prefix = match self.state.mode {
                InputMode::Normal => ">",
                InputMode::Command => ">",
                InputMode::Fen => ">",
                InputMode::Pgn => "PGN>",
            };
            (prefix, self.state.buffer.as_str(), Style::default().fg(Color::White))
        };

        // Render prefix
        let prefix_style = Style::default().fg(Color::Cyan);
        buf.set_string(inner.x, inner.y, prefix, prefix_style);

        // Render content
        let content_x = inner.x + prefix.len() as u16 + 1;
        buf.set_string(content_x, inner.y, content, style);

        // Render cursor (only in input modes and when showing buffer)
        if self.state.is_input_mode() && self.state.error.is_none() && self.state.message.is_none() {
            let cursor_x = content_x + self.state.cursor as u16;
            if cursor_x < inner.x + inner.width {
                let cursor_char = self
                    .state
                    .buffer
                    .chars()
                    .nth(self.state.cursor)
                    .unwrap_or(' ');
                buf.set_string(
                    cursor_x,
                    inner.y,
                    cursor_char.to_string(),
                    Style::default()
                        .fg(Color::Black)
                        .bg(Color::White),
                );
            }
        }
    }
}

/// Help bar widget showing available shortcuts
pub struct HelpBarWidget {
    show_input_help: bool,
}

impl HelpBarWidget {
    pub fn new(show_input_help: bool) -> Self {
        Self { show_input_help }
    }
}

impl Widget for HelpBarWidget {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let shortcuts = if self.show_input_help {
            vec![
                ("Enter", "Submit"),
                ("Esc", "Cancel"),
            ]
        } else {
            vec![
                ("?", "Help"),
                ("i", "Import"),
                ("f", "Flip"),
                ("p", "Pause"),
                ("←/→", "Nav"),
                ("d", "Depth"),
                ("m", "Lines"),
                ("y", "Copy FEN"),
                ("q", "Quit"),
            ]
        };

        let mut x = area.x;
        for (key, action) in shortcuts {
            if x >= area.x + area.width {
                break;
            }

            // Key
            let key_style = Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan);
            buf.set_string(x, area.y, format!(" {} ", key), key_style);
            x += key.len() as u16 + 2;

            // Action
            let action_style = Style::default().fg(Color::DarkGray);
            buf.set_string(x, area.y, format!("{} ", action), action_style);
            x += action.len() as u16 + 1;
        }
    }
}
