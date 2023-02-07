//! Parser for commands sent to the bot

use clap::{arg, command, Parser, Subcommand};

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
        /// Id of the repository
        repo: usize,
        #[command(subcommand)]
        family: Family,
    },

    /// Unsubscribe from an event
    Unsubscribe {
        /// Id of the repository
        repo: usize,
        #[command(subcommand)]
        family: Family,
    },

    // Change supported repositories
    Repositories {
        #[command(subcommand)]
        repo_subcommands: RepoSubcommands,
    },
}

#[derive(Subcommand, PartialEq, Eq, Debug)]
pub enum RepoSubcommands {
    // List all available repositories
    List,

    // Add a webhook for a new repository
    Add {
        // Name of repo owner (user or organisation)
        owner: String,

        // Name of repository
        repository: String,

        // REST-Api key
        // help: https://docs.github.com/en/authentication/managing-commit-signature-verification/adding-a-gpg-key-to-your-github-account
        api_key: String,
    },

    // Remove a repositories webhook
    Remove {
        // Id of repository to remove
        repository: usize,

        // REST-Api key
        // help: https://docs.github.com/en/authentication/managing-commit-signature-verification/adding-a-gpg-key-to-your-github-account
        api_key: String,
    },
}

#[derive(Subcommand, PartialEq, Eq, Debug)]
pub enum Family {
    Pr {
        #[arg(value_enum)]
        pr_action: PRAction,
    },
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
