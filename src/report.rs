use serde::Serialize;

use crate::bigrams;
use crate::cli::Format;
use crate::db::Db;
use crate::trigrams;

#[derive(Serialize)]
pub struct Report {
    pub generated_at:  i64,
    pub recorded_since: Option<i64>,
    pub duration_s:    Option<i64>,
    pub filter:        Filter,
    pub keys:          Vec<KeyEntry>,
    pub apps:          Vec<AppEntry>,
    pub bigrams:       Vec<bigrams::BigramEntry>,
    pub trigrams:      Vec<trigrams::TrigramEntry>,
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

    let keys = if apps.is_empty() {
        db.top_keys_global(top).expect("query failed")
            .into_iter().map(|(key, modifiers, count)| (key, modifiers, String::new(), count)).collect()
    } else {
        db.top_keys(apps, top).expect("query failed")
    };
    let top_apps = if apps.is_empty() { db.top_apps(None).expect("query failed") } else { vec![] };
    let bigram_report   = bigrams::build(apps, top, &db);
    let trigram_report  = trigrams::build(apps, top, &db);

    let now = now_unix_ms();
    let recorded_since = db.first_ts().expect("query failed");
    let duration_s = recorded_since.map(|first| (now - first) / 1000);

    let report = Report {
        generated_at: now,
        recorded_since,
        duration_s,
        filter:  Filter { apps: apps.to_vec(), top },
        keys:    keys.into_iter().map(|(key, modifiers, app, count)| KeyEntry { key, modifiers, app, count }).collect(),
        apps:    top_apps.into_iter().map(|(class, count)| AppEntry { class, count }).collect(),
        bigrams:  bigram_report.bigrams,
        trigrams: trigram_report.trigrams,
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
    if let Some(dur) = report.duration_s {
        println!("Recording duration: {}", fmt_duration(dur));
    }
    println!("{}", "─".repeat(42));

    let max_count = report.keys[0].count;
    let max_label_len = report.keys.iter().map(|e| display_label(e).len()).max().unwrap_or(1);

    for (i, entry) in report.keys.iter().enumerate() {
        let label = display_label(entry);
        let bar_len = (entry.count * 20 / max_count) as usize;
        let bar = "█".repeat(bar_len);
        if report.filter.apps.is_empty() {
            println!("{:>2}.  {:<width$}  {:>6}  {}", i + 1, label, entry.count, bar, width = max_label_len);
        } else {
            println!("{:>2}.  {:<width$}  {:>6}  {:<20}  {}", i + 1, label, entry.count, bar, entry.app, width = max_label_len);
        }
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
            generated_at:   report.generated_at,
            recorded_since: report.recorded_since,
            duration_s:     report.duration_s,
            filter: bigrams::Filter { apps: report.filter.apps.clone(), top: report.filter.top },
            bigrams: report.bigrams.to_vec(),
        });
    }

    if !report.trigrams.is_empty() {
        trigrams::print_text(&trigrams::TrigramReport {
            generated_at:   report.generated_at,
            recorded_since: report.recorded_since,
            duration_s:     report.duration_s,
            filter: trigrams::Filter { apps: report.filter.apps.clone(), top: report.filter.top },
            trigrams: report.trigrams.to_vec(),
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

fn fmt_duration(secs: i64) -> String {
    let d = secs / 86400;
    let h = (secs % 86400) / 3600;
    let m = (secs % 3600) / 60;
    if d > 0 { format!("{d}d {h}h {m}m") }
    else if h > 0 { format!("{h}h {m}m") }
    else { format!("{m}m") }
}
