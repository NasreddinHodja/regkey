use serde::Serialize;

use crate::bigrams;
use crate::cli::Format;
use crate::db::Db;

#[derive(Serialize)]
pub struct Report {
    pub generated_at: i64,
    pub filter:  Filter,
    pub keys:    Vec<KeyEntry>,
    pub apps:    Vec<AppEntry>,
    pub bigrams: Vec<bigrams::BigramEntry>,
}

#[derive(Serialize)]
pub struct Filter {
    pub apps: Vec<String>,
    pub top:  Option<usize>,
}

#[derive(Serialize)]
pub struct KeyEntry {
    pub key:       String,
    pub modifiers: String,
    pub app:       String,
    pub count:     i64,
}

#[derive(Serialize)]
pub struct AppEntry {
    pub class: String,
    pub count: i64,
}

pub fn run(apps: &[String], top: Option<usize>, format: Format) {
    let db = Db::open().expect("failed to open database");

    let keys = db.top_keys(apps, top).expect("query failed");
    let top_apps = if apps.is_empty() { db.top_apps(None).expect("query failed") } else { vec![] };
    let bigram_report = bigrams::build(apps, top, &db);

    let report = Report {
        generated_at: now_unix_ms(),
        filter:  Filter { apps: apps.to_vec(), top },
        keys:    keys.into_iter().map(|(key, modifiers, app, count)| KeyEntry { key, modifiers, app, count }).collect(),
        apps:    top_apps.into_iter().map(|(class, count)| AppEntry { class, count }).collect(),
        bigrams: bigram_report.bigrams,
    };

    match format {
        Format::Json => println!("{}", serde_json::to_string_pretty(&report).expect("serialization failed")),
        Format::Text => print_text(&report),
    }
}

fn print_text(report: &Report) {
    if report.keys.is_empty() {
        println!("No keystrokes recorded yet.");
        return;
    }

    let header = if report.filter.apps.is_empty() {
        "Global keystroke report".to_string()
    } else {
        format!("Keystroke report — {}", report.filter.apps.join(", "))
    };
    println!("\n{header}");
    println!("{}", "─".repeat(42));

    let max_count = report.keys[0].count;
    let max_label_len = report.keys.iter().map(|e| display_label(e).len()).max().unwrap_or(1);

    for (i, entry) in report.keys.iter().enumerate() {
        let label = display_label(entry);
        let bar_len = (entry.count * 20 / max_count) as usize;
        let bar = "█".repeat(bar_len);
        println!("{:>2}.  {:<width$}  {:>6}  {:<20}  {}", i + 1, label, entry.count, bar, entry.app, width = max_label_len);
    }

    if !report.apps.is_empty() {
        println!("\nTop apps");
        println!("{}", "─".repeat(42));
        let max_app_len = report.apps.iter().map(|e| e.class.len()).max().unwrap_or(1);
        for entry in &report.apps {
            println!("  {:<width$}  {} keystrokes", entry.class, entry.count, width = max_app_len);
        }
    }

    if !report.bigrams.is_empty() {
        bigrams::print_text(&bigrams::BigramReport {
            generated_at: report.generated_at,
            filter: bigrams::Filter { apps: report.filter.apps.clone(), top: report.filter.top },
            bigrams: report.bigrams.iter().cloned().collect(),
        });
    }

    println!();
}

fn display_label(entry: &KeyEntry) -> String {
    if entry.modifiers.is_empty() {
        entry.key.clone()
    } else {
        format!("{}+{}", entry.modifiers, entry.key)
    }
}

fn now_unix_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64
}
