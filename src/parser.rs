use clap::{Parser, Subcommand, ValueEnum};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Subscribe to a repository
    Subscribe { repo: String, event_type: EventType },
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
enum EventType {
    Push,
    Pull,
}

pub fn test() {
    let cli = Cli::parse();
    match &cli.command {
        Commands::Subscribe { repo, .. } => {
            println!("{repo:?}")
        }
    }
}
