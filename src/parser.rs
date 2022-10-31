use clap::{
    arg, builder::PossibleValue, command, value_parser, Arg, ArgAction, Command, Parser,
    Subcommand, ValueEnum,
};

use crate::bot::GitRepository;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, PartialEq, Debug)]
pub enum Commands {
    /// Subscribe to a repository
    Subscribe {
        repo: i32,

        #[arg(value_enum)]
        family: Subscribeable,

        #[arg(value_enum)]
        event_type: EventType,
    },
    Unsubscribe {
        repo: String,

        #[arg(value_enum)]
        family: Subscribeable,

        #[arg(value_enum)]
        event_type: EventType,
    },
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Debug)]
pub enum EventType {
    Push,
    Pull,
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Debug)]
pub enum IssueActions {
    Open,
    Close,
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Debug)]
pub enum Subscribeable {
    Issues,
    PR,
}

pub fn build_command(repositories: &[GitRepository]) -> Command {
    let repo_options = repositories
        .iter()
        .map(|repo| repo.name.as_str())
        .collect::<Vec<_>>()
        .as_slice();

    command!().subcommand(
        Command::new("subscribe")
            .arg(arg!(<Subscribeable>).value_parser(value_parser!(Subscribeable)))
            .arg(arg!(<IssueActions>).value_parser(value_parser!(IssueActions)))
            .arg(Arg::new("repo")),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_listen_issue_pull() {
        let cli = Cli::parse_from("THROWAWAY subscribe 558781383 issues pull".split(" "));
        assert_eq!(
            cli.command,
            Commands::Subscribe {
                repo: 558781383,
                family: Subscribeable::Issues,
                event_type: EventType::Pull
            }
        )
    }
}
