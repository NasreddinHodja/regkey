use std::sync::mpsc;

use evdev::KeyCode;

use crate::db::Db;
use crate::window;

pub enum Event {
    Key { key: String, modifiers: String, ts: i64 },
    AppChange { class: String, title: String },
}

pub fn run(window_ms: u64) {
    let (tx, rx) = mpsc::channel::<Event>();

    window::detect_provider().spawn(tx.clone());

    let keyboards: Vec<_> = evdev::enumerate()
        .filter(|(_, dev)| is_keyboard(dev))
        .collect();

    if keyboards.is_empty() {
        eprintln!("regkey: no keyboard devices found — try running with sudo or add yourself to the 'input' group");
        std::process::exit(1);
    }

    for (path, device) in keyboards {
        let name = device.name().unwrap_or("unknown").to_string();
        eprintln!("regkey: listening on {} ({})", path.display(), name);
        let tx = tx.clone();
        std::thread::spawn(move || read_device(device, tx));
    }

    eprintln!("regkey: recording — press Ctrl-C to stop");
    drop(tx);

    ctrlc::set_handler(|| {
        eprintln!("\nregkey: stopped.");
        std::process::exit(0);
    }).expect("failed to set Ctrl-C handler");

    let db = Db::open().expect("failed to open database");
    let mut app_class = String::new();
    let mut app_title = String::new();
    let mut prev:      Option<(String, String, i64)> = None;
    let mut prev_prev: Option<(String, String, i64)> = None;

    for event in rx {
        match event {
            Event::AppChange { class, title } => {
                app_class  = class;
                app_title  = title;
                prev       = None;
                prev_prev  = None;
            }
            Event::Key { key, modifiers, ts } => {
                let win = window_ms as i64;
                if let Some((ref pk, ref pm, pts)) = prev
                    && ts - pts < win
                {
                    if let Some((ref ppk, ref ppm, ppts)) = prev_prev
                        && pts - ppts < win
                    {
                        db.insert_trigram(ts, ppk, ppm, pk, pm, &key, &modifiers, &app_class)
                            .expect("trigram insert failed");
                    }
                    db.insert_bigram(ts, pk, pm, &key, &modifiers, &app_class)
                        .expect("bigram insert failed");
                }
                db.insert(ts, &key, &modifiers, &app_class, &app_title)
                    .expect("db insert failed");
                prev_prev = prev.take();
                prev = Some((key, modifiers, ts));
            }
        }
    }
}

fn is_keyboard(dev: &evdev::Device) -> bool {
    dev.supported_keys()
        .is_some_and(|keys| keys.contains(KeyCode::KEY_A))
}

fn read_device(mut device: evdev::Device, tx: mpsc::Sender<Event>) {
    let mut mods = ModifierState::default();

    loop {
        let events = match device.fetch_events() {
            Ok(e)  => e,
            Err(_) => break,
        };

        for ev in events {
            if ev.event_type() != evdev::EventType::KEY { continue; }

            let code = KeyCode::new(ev.code());

            match ev.value() {
                0 => mods.release(code),
                1 => {
                    mods.press(code);
                    if !ModifierState::is_modifier(code) {
                        let event = Event::Key {
                            key:       key_name(code),
                            modifiers: mods.as_string(),
                            ts:        now_ms(),
                        };
                        if tx.send(event).is_err() { return; }
                    }
                }
                _ => {}
            }
        }
    }
}

#[derive(Default)]
struct ModifierState {
    shift: bool,
    ctrl:  bool,
    alt:   bool,
    sup:   bool,
}

impl ModifierState {
    fn press(&mut self, code: KeyCode) {
        match code {
            KeyCode::KEY_LEFTSHIFT | KeyCode::KEY_RIGHTSHIFT => self.shift = true,
            KeyCode::KEY_LEFTCTRL  | KeyCode::KEY_RIGHTCTRL  => self.ctrl  = true,
            KeyCode::KEY_LEFTALT   | KeyCode::KEY_RIGHTALT   => self.alt   = true,
            KeyCode::KEY_LEFTMETA  | KeyCode::KEY_RIGHTMETA  => self.sup   = true,
            _ => {}
        }
    }

    fn release(&mut self, code: KeyCode) {
        match code {
            KeyCode::KEY_LEFTSHIFT | KeyCode::KEY_RIGHTSHIFT => self.shift = false,
            KeyCode::KEY_LEFTCTRL  | KeyCode::KEY_RIGHTCTRL  => self.ctrl  = false,
            KeyCode::KEY_LEFTALT   | KeyCode::KEY_RIGHTALT   => self.alt   = false,
            KeyCode::KEY_LEFTMETA  | KeyCode::KEY_RIGHTMETA  => self.sup   = false,
            _ => {}
        }
    }

    fn is_modifier(code: KeyCode) -> bool {
        matches!(code,
            KeyCode::KEY_LEFTSHIFT | KeyCode::KEY_RIGHTSHIFT |
            KeyCode::KEY_LEFTCTRL  | KeyCode::KEY_RIGHTCTRL  |
            KeyCode::KEY_LEFTALT   | KeyCode::KEY_RIGHTALT   |
            KeyCode::KEY_LEFTMETA  | KeyCode::KEY_RIGHTMETA
        )
    }

    fn as_string(&self) -> String {
        let mut parts = Vec::new();
        if self.ctrl  { parts.push("ctrl");  }
        if self.alt   { parts.push("alt");   }
        if self.sup   { parts.push("super"); }
        if self.shift { parts.push("shift"); }
        parts.join("+")
    }
}

fn key_name(code: KeyCode) -> String {
    let raw = format!("{code:?}");
    raw.strip_prefix("KEY_").unwrap_or(&raw).to_lowercase()
}

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64
}
