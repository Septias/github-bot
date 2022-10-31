use clap::{arg, command, Parser, Subcommand, ValueEnum};

use crate::shared::{issue::IssueAction, pr::PRAction};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, PartialEq, Debug)]
pub enum Commands {
    /// Subscribe to a repository event
    Subscribe {
        repo: usize,

        #[arg(value_enum)]
        family: EventFamily,

        #[arg(value_enum)]
        action: IssueAction,
    },

    /// Unsubscribe from a repository event
    Unsubscribe {
        repo: String,

        #[arg(value_enum)]
        family: EventFamily,

        #[arg(value_enum)]
        pr_action: PRAction,
    },
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Debug)]
pub enum EventFamily {
    Issues,
    PR,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_listen_issue_pull() {
        let cli = Cli::parse_from("THROWAWAY subscribe 558781383 issues opened".split(" "));
        assert_eq!(
            cli.command,
            Commands::Subscribe {
                repo: 558781383,
                family: EventFamily::Issues,
                issue_action: IssueAction::Opened
            }
        )
    }
}
