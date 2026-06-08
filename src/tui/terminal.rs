use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io::{self, Stdout};

use crate::error::{AppError, Result};

pub type BackendTerminal = Terminal<CrosstermBackend<Stdout>>;

pub struct TerminalManager {
    pub terminal: BackendTerminal,
}

impl TerminalManager {
    pub fn new() -> Result<Self> {
        enable_raw_mode().map_err(AppError::Io)?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen).map_err(AppError::Io)?;
        let backend = CrosstermBackend::new(io::stdout());
        let terminal = Terminal::new(backend).map_err(AppError::Io)?;
        Ok(Self { terminal })
    }

    pub fn restore() -> Result<()> {
        disable_raw_mode().map_err(AppError::Io)?;
        execute!(io::stdout(), LeaveAlternateScreen).map_err(AppError::Io)?;
        Ok(())
    }
}

impl Drop for TerminalManager {
    fn drop(&mut self) {
        let _ = Self::restore();
    }
}
