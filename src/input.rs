use crossterm::event::{self, Event};
use tokio::sync::mpsc;

/// Reads terminal events on a dedicated thread so the UI loop can await them
/// together with runtime events instead of polling on a timer.
pub fn spawn_reader() -> mpsc::UnboundedReceiver<Event> {
    let (sender, receiver) = mpsc::unbounded_channel();
    std::thread::Builder::new()
        .name("fetchdeck-input".to_owned())
        .spawn(move || {
            while let Ok(event) = event::read() {
                if sender.send(event).is_err() {
                    break;
                }
            }
        })
        .expect("input thread should start");
    receiver
}
