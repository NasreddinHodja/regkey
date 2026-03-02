use std::sync::mpsc;

use crate::record::Event;
use super::WindowProvider;

pub struct NullProvider;

impl WindowProvider for NullProvider {
    fn spawn(self: Box<Self>, tx: mpsc::Sender<Event>) {
        std::thread::spawn(move || {
            let _ = tx.send(Event::AppChange { class: String::new(), title: String::new() });
        });
    }
}
