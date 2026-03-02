use std::sync::mpsc;

use crate::record::Event;

pub mod hyprland;
pub mod null;
pub mod sway;

pub trait WindowProvider: Send {
    fn spawn(self: Box<Self>, tx: mpsc::Sender<Event>);
}

pub fn detect_provider() -> Box<dyn WindowProvider> {
    if std::env::var("HYPRLAND_INSTANCE_SIGNATURE").is_ok() {
        Box::new(hyprland::HyprlandProvider)
    } else if std::env::var("SWAYSOCK").is_ok() {
        Box::new(sway::SwayProvider)
    } else {
        Box::new(null::NullProvider)
    }
}
