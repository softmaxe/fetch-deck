use anyhow::Result;
use yt_dlp_tui::{app::App, terminal::TerminalSession};

#[tokio::main]
async fn main() -> Result<()> {
    let mut app = App::new()?;
    let mut terminal = TerminalSession::new()?;
    app.run(terminal.terminal_mut()).await
}
