use serde::Serialize;

use crate::cli::Format;
use crate::db::Db;

#[derive(Serialize)]
pub struct BigramReport {
    pub generated_at: i64,
    pub filter: Filter,
    pub bigrams: Vec<BigramEntry>,
}

#[derive(Serialize)]
pub struct Filter {
    pub apps: Vec<String>,
    pub top:  Option<usize>,
}

#[derive(Serialize, Clone)]
pub struct BigramEntry {
    pub prev_key:  String,
    pub prev_mods: String,
    pub curr_key:  String,
    pub curr_mods: String,
    pub app:       String,
    pub count:     i64,
}

pub fn run(apps: &[String], top: Option<usize>, format: Format) {
    let db = Db::open().expect("failed to open database");
    let report = build(apps, top, &db);
    match format {
        Format::Json => println!("{}", serde_json::to_string_pretty(&report).expect("serialization failed")),
        Format::Text => print_text(&report),
    }
}

pub fn build(apps: &[String], top: Option<usize>, db: &Db) -> BigramReport {
    let rows = db.top_bigrams(apps, top).expect("query failed");
    BigramReport {
        generated_at: now_unix_ms(),
        filter: Filter { apps: apps.to_vec(), top },
        bigrams: rows.into_iter().map(|(prev_key, prev_mods, curr_key, curr_mods, app, count)| {
            BigramEntry { prev_key, prev_mods, curr_key, curr_mods, app, count }
        }).collect(),
    }
}

pub fn print_text(report: &BigramReport) {
    if report.bigrams.is_empty() {
        println!("No bigrams recorded yet.");
        return;
    }

    let header = if report.filter.apps.is_empty() {
        "Global bigram report".to_string()
    } else {
        format!("Bigram report — {}", report.filter.apps.join(", "))
    };
    println!("\n{header}");
    println!("{}", "─".repeat(42));

    let max_count = report.bigrams[0].count;
    let max_label_len = report.bigrams.iter().map(|e| display_label(e).len()).max().unwrap_or(1);

    for (i, entry) in report.bigrams.iter().enumerate() {
        let label = display_label(entry);
        let bar_len = (entry.count * 20 / max_count) as usize;
        let bar = "█".repeat(bar_len);
        println!("{:>2}.  {:<width$}  {:>6}  {:<20}  {}", i + 1, label, entry.count, bar, entry.app, width = max_label_len);
    }

    println!();
}

fn display_label(e: &BigramEntry) -> String {
    let prev = if e.prev_mods.is_empty() { e.prev_key.clone() } else { format!("{}+{}", e.prev_mods, e.prev_key) };
    let curr = if e.curr_mods.is_empty() { e.curr_key.clone() } else { format!("{}+{}", e.curr_mods, e.curr_key) };
    format!("{prev} -> {curr}")
}

fn now_unix_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64
}
