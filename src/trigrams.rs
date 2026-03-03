use serde::Serialize;

use crate::cli::Format;
use crate::db::Db;

#[derive(Serialize)]
pub struct TrigramReport {
    pub generated_at:   i64,
    pub recorded_since: Option<i64>,
    pub duration_s:     Option<i64>,
    pub filter:         Filter,
    pub trigrams:       Vec<TrigramEntry>,
}

#[derive(Serialize)]
pub struct Filter {
    pub apps: Vec<String>,
    pub top:  Option<usize>,
}

#[derive(Serialize, Clone)]
pub struct TrigramEntry {
    pub first_key:  String,
    pub first_mods: String,
    pub mid_key:    String,
    pub mid_mods:   String,
    pub last_key:   String,
    pub last_mods:  String,
    pub app:        String,
    pub count:      i64,
}

pub fn run(apps: &[String], top: Option<usize>, format: Format) {
    let db = Db::open().expect("failed to open database");
    let report = build(apps, top, &db);
    match format {
        Format::Json => println!("{}", serde_json::to_string_pretty(&report).expect("serialization failed")),
        Format::Text => print_text(&report),
    }
}

pub fn build(apps: &[String], top: Option<usize>, db: &Db) -> TrigramReport {
    let rows = db.top_trigrams(apps, top).expect("query failed");
    let now = now_unix_ms();
    let recorded_since = db.first_ts().expect("query failed");
    let duration_s = recorded_since.map(|first| (now - first) / 1000);
    TrigramReport {
        generated_at: now,
        recorded_since,
        duration_s,
        filter: Filter { apps: apps.to_vec(), top },
        trigrams: rows.into_iter().map(|(first_key, first_mods, mid_key, mid_mods, last_key, last_mods, app, count)| {
            TrigramEntry { first_key, first_mods, mid_key, mid_mods, last_key, last_mods, app, count }
        }).collect(),
    }
}

pub fn print_text(report: &TrigramReport) {
    if report.trigrams.is_empty() {
        println!("No trigrams recorded yet.");
        return;
    }

    let header = if report.filter.apps.is_empty() {
        "Global trigram report".to_string()
    } else {
        format!("Trigram report — {}", report.filter.apps.join(", "))
    };
    println!("\n{header}");
    if let Some(dur) = report.duration_s {
        println!("Recording duration: {}", fmt_duration(dur));
    }
    println!("{}", "─".repeat(42));

    let max_count = report.trigrams[0].count;
    let max_label_len = report.trigrams.iter().map(|e| display_label(e).len()).max().unwrap_or(1);

    for (i, entry) in report.trigrams.iter().enumerate() {
        let label = display_label(entry);
        let bar_len = (entry.count * 20 / max_count) as usize;
        let bar = "█".repeat(bar_len);
        println!("{:>2}.  {:<width$}  {:>6}  {:<20}  {}", i + 1, label, entry.count, bar, entry.app, width = max_label_len);
    }

    println!();
}

fn display_label(e: &TrigramEntry) -> String {
    let first = if e.first_mods.is_empty() { e.first_key.clone() } else { format!("{}+{}", e.first_mods, e.first_key) };
    let mid   = if e.mid_mods.is_empty()   { e.mid_key.clone()   } else { format!("{}+{}", e.mid_mods,   e.mid_key)   };
    let last  = if e.last_mods.is_empty()  { e.last_key.clone()  } else { format!("{}+{}", e.last_mods,  e.last_key)  };
    format!("{first} -> {mid} -> {last}")
}

fn fmt_duration(secs: i64) -> String {
    let d = secs / 86400;
    let h = (secs % 86400) / 3600;
    let m = (secs % 3600) / 60;
    if d > 0 { format!("{d}d {h}h {m}m") }
    else if h > 0 { format!("{h}h {m}m") }
    else { format!("{m}m") }
}

fn now_unix_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64
}
