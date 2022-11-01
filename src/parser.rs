use clap::{arg, command, Parser, Subcommand, ValueEnum};

use crate::shared::{issue::IssueAction, pr::PRAction};

#[derive(Parser)]
#[command(author = None, version = None, about = None, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, PartialEq, Eq, Debug)]
pub enum Commands {
    /// Subscribe to an event
    Subscribe {
        repo: usize,
        #[command(subcommand)]
        family: Family,
    },

    /// Unsubscribe from an event
    Unsubscribe {
        repo: String,
        #[command(subcommand)]
        family: Family,
    },

    // Change supported repositories
    Repository {
        #[arg(value_enum)]
        action: RepoAction,

        // Name of repo owner (user or organisation)
        user: String,

        // Name of repository
        repository: String,

        // REST-Api key
        api_key: String,
    },
}

#[derive(ValueEnum, Clone, PartialEq, Eq, Debug)]
pub enum RepoAction {
    Add,
    Remove,
}

#[derive(Subcommand, PartialEq, Eq, Debug)]
pub enum Family {
    // Subscribe to an PR event
    PR {
        #[arg(value_enum)]
        pr_action: PRAction,
    },
    // Subscribe to an issue event
    Issue {
        #[arg(value_enum)]
        issue_action: IssueAction,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_listen_issue_pull() {
        let cli = Cli::parse_from("wat subscribe 558781383 issue opened".split(' '));
        assert_eq!(
            cli.command,
            Commands::Subscribe {
                repo: 558781383,
                family: Family::Issue {
                    issue_action: IssueAction::Opened
                }
            }
        )
    }
}
