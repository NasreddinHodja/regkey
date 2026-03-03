use std::io::{BufRead, BufReader, Read, Write};
use std::os::unix::net::UnixStream;
use std::sync::mpsc;

use crate::record::Event;
use super::WindowProvider;

pub struct HyprlandProvider;

impl WindowProvider for HyprlandProvider {
    fn spawn(self: Box<Self>, tx: mpsc::Sender<Event>) {
        std::thread::spawn(move || {
            let runtime = std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "/run/user/1000".into());
            let sig = std::env::var("HYPRLAND_INSTANCE_SIGNATURE").unwrap();

            if let Some(event) = query_active_window(&runtime, &sig)
                && tx.send(event).is_err()
            {
                return;
            }

            let socket2 = format!("{}/hypr/{}/.socket2.sock", runtime, sig);
            let stream = match UnixStream::connect(&socket2) {
                Ok(s) => s,
                Err(e) => { eprintln!("regkey: hyprland socket2: {e}"); return; }
            };

            for line in BufReader::new(stream).lines() {
                let Ok(line) = line else { break };
                if let Some(rest) = line.strip_prefix("activewindow>>") {
                    let (class, title) = rest.split_once(',').unwrap_or((rest, ""));
                    if tx.send(Event::AppChange {
                        class: class.to_string(),
                        title: title.to_string(),
                    }).is_err() { break; }
                }
            }
        });
    }
}

fn query_active_window(runtime: &str, sig: &str) -> Option<Event> {
    let socket1 = format!("{}/hypr/{}/.socket.sock", runtime, sig);
    let mut stream = UnixStream::connect(&socket1).ok()?;
    stream.write_all(b"j/activewindow").ok()?;

    let mut response = String::new();
    stream.read_to_string(&mut response).ok()?;

    let v: serde_json::Value = serde_json::from_str(&response).ok()?;
    let class = v["class"].as_str().unwrap_or("").to_string();
    let title = v["title"].as_str().unwrap_or("").to_string();

    Some(Event::AppChange { class, title })
}
