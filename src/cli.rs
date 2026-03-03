use clap::{Parser, Subcommand, ValueEnum};

#[derive(Parser)]
#[command(name = "regkey", about = "Keystroke recorder and reporter")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Start recording keystrokes
    Record {
        /// Bigram time window in milliseconds
        #[arg(long, default_value_t = 2000)]
        window: u64,
    },
    /// Clear recorded keystrokes
    Clear {
        /// Clear only keystrokes for a specific application class
        #[arg(long)]
        app: Option<String>,
    },
    /// Show keystroke report
    Report {
        /// Filter to one or more app classes (comma-separated, e.g. kitty,emacs)
        #[arg(long, value_delimiter = ',')]
        app: Vec<String>,
        /// Limit results to top N (default: show all)
        #[arg(long)]
        top: Option<usize>,
        /// Output format
        #[arg(long, default_value = "json")]
        format: Format,
    },
    /// Show bigram (key sequence) report
    Bigrams {
        /// Filter to one or more app classes (comma-separated)
        #[arg(long, value_delimiter = ',')]
        app: Vec<String>,
        /// Limit results to top N (default: show all)
        #[arg(long)]
        top: Option<usize>,
        /// Output format
        #[arg(long, default_value = "json")]
        format: Format,
    },
    /// Show trigram (three-key sequence) report
    Trigrams {
        /// Filter to one or more app classes (comma-separated)
        #[arg(long, value_delimiter = ',')]
        app: Vec<String>,
        /// Limit results to top N (default: show all)
        #[arg(long)]
        top: Option<usize>,
        /// Output format
        #[arg(long, default_value = "json")]
        format: Format,
    },
}

#[derive(Clone, ValueEnum)]
pub enum Format {
    Json,
    Text,
}
