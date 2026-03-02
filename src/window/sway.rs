use std::sync::mpsc;

use crate::record::Event;
use super::WindowProvider;

pub struct SwayProvider;

impl WindowProvider for SwayProvider {
    fn spawn(self: Box<Self>, tx: mpsc::Sender<Event>) {
        std::thread::spawn(move || {
            // Stub: Sway IPC JSON protocol not yet implemented.
            let _ = tx.send(Event::AppChange { class: "sway".to_string(), title: String::new() });
        });
    }
}
