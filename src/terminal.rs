use std::io::{self, Stdout, stdout};

use crossterm::{
    event::{DisableBracketedPaste, DisableMouseCapture, EnableBracketedPaste, EnableMouseCapture},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};

pub type Tui = Terminal<CrosstermBackend<Stdout>>;

pub struct TerminalSession {
    terminal: Tui,
}

impl TerminalSession {
    pub fn new() -> io::Result<Self> {
        enable_raw_mode()?;
        let mut output = stdout();
        // Bracketed paste turns a pasted URL into one event instead of one event per character.
        if let Err(error) = execute!(
            output,
            EnterAlternateScreen,
            EnableMouseCapture,
            EnableBracketedPaste
        ) {
            restore(&mut output);
            return Err(error);
        }

        let mut terminal = match Terminal::new(CrosstermBackend::new(output)) {
            Ok(terminal) => terminal,
            Err(error) => {
                restore(&mut stdout());
                return Err(error);
            }
        };
        if let Err(error) = terminal.clear() {
            restore(terminal.backend_mut());
            return Err(error);
        }
        Ok(Self { terminal })
    }

    pub fn terminal_mut(&mut self) -> &mut Tui {
        &mut self.terminal
    }
}

impl Drop for TerminalSession {
    fn drop(&mut self) {
        restore(self.terminal.backend_mut());
        let _ = self.terminal.show_cursor();
    }
}

fn restore(output: &mut impl io::Write) {
    let _ = disable_raw_mode();
    let _ = execute!(
        output,
        DisableBracketedPaste,
        LeaveAlternateScreen,
        DisableMouseCapture
    );
}
