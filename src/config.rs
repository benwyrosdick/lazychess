use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub engine: EngineConfig,
    pub ui: UiConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineConfig {
    /// Path to Stockfish binary (None = auto-detect from $PATH)
    pub path: Option<String>,
    /// Search depth
    pub depth: u32,
    /// Number of best lines to show
    pub multipv: u32,
    /// CPU threads for analysis
    pub threads: u32,
    /// Hash table size in MB
    pub hash: u32,
    /// Draw avoidance (-100 to 100)
    pub contempt: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiConfig {
    /// View board from black's perspective
    pub flip_board: bool,
    /// Show coordinate labels
    pub show_coordinates: bool,
    /// Highlight the last move
    pub highlight_last_move: bool,
    /// Piece display style: "unicode", "nerd", or "ascii"
    #[serde(default = "default_piece_style")]
    pub piece_style: String,
}

fn default_piece_style() -> String {
    "nerd".to_string()
}

impl Default for Config {
    fn default() -> Self {
        Self {
            engine: EngineConfig::default(),
            ui: UiConfig::default(),
        }
    }
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            path: None,
            depth: 20,
            multipv: 3,
            threads: 4,
            hash: 256,
            contempt: 0,
        }
    }
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            flip_board: false,
            show_coordinates: true,
            highlight_last_move: true,
            piece_style: "nerd".to_string(),
        }
    }
}

impl UiConfig {
    /// Get the piece style enum from the config string
    pub fn get_piece_style(&self) -> crate::chess::PieceStyle {
        match self.piece_style.to_lowercase().as_str() {
            "unicode" => crate::chess::PieceStyle::Unicode,
            "nerd" | "nerdfont" | "nerd_font" => crate::chess::PieceStyle::NerdFont,
            "ascii" | "letter" | "letters" => crate::chess::PieceStyle::Ascii,
            _ => crate::chess::PieceStyle::NerdFont,
        }
    }
}

impl Config {
    /// Get the config file path
    pub fn config_path() -> Option<PathBuf> {
        dirs::config_dir().map(|p| p.join("lazychess").join("config.toml"))
    }

    /// Load config from file, or create default if not exists
    pub fn load() -> Result<Self> {
        let path = Self::config_path();

        match path {
            Some(p) if p.exists() => {
                let contents = fs::read_to_string(&p)
                    .with_context(|| format!("Failed to read config from {:?}", p))?;
                let config: Config = toml::from_str(&contents)
                    .with_context(|| format!("Failed to parse config from {:?}", p))?;
                Ok(config)
            }
            _ => Ok(Config::default()),
        }
    }

    /// Save config to file
    pub fn save(&self) -> Result<()> {
        let path = Self::config_path().context("Could not determine config directory")?;

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create config directory {:?}", parent))?;
        }

        let contents = toml::to_string_pretty(self).context("Failed to serialize config")?;
        fs::write(&path, contents)
            .with_context(|| format!("Failed to write config to {:?}", path))?;

        Ok(())
    }

    /// Get the Stockfish path, either from config or by searching $PATH
    pub fn stockfish_path(&self) -> Option<String> {
        if let Some(ref path) = self.engine.path {
            return Some(path.clone());
        }

        // Try to find stockfish in $PATH
        which::which("stockfish")
            .ok()
            .map(|p| p.to_string_lossy().to_string())
    }
}
