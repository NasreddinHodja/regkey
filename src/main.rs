use clap::Parser;

mod bigrams;
mod cli;
mod db;
mod record;
mod report;
mod window;

fn main() {
    let cli = cli::Cli::parse();
    match cli.command {
        cli::Command::Record { window } => record::run(window),
        cli::Command::Clear { app } => {
            let db = db::Db::open().expect("failed to open database");
            let deleted = db.clear(app.as_deref()).expect("clear failed");
            println!("Deleted {deleted} keystroke(s).");
        }
        cli::Command::Report { app, top, format } => report::run(&app, top, format),
        cli::Command::Bigrams { app, top, format } => bigrams::run(&app, top, format),
    }
}
