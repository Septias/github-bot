use serde::{Deserialize, Serialize};
use strum_macros::Display;

use self::{issue::IssueEvent, pr::PREvent};

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct User {
    pub login: String,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct Repository {
    pub id: usize,
    pub name: String,
}

#[derive(Debug, Display)]
pub enum WebhookEvent {
    Issue(IssueEvent),
    PR(PREvent),
}

pub mod issue {
    use clap::ValueEnum;
    use serde::{Deserialize, Serialize};
    use strum_macros::Display;

    use super::{Repository, User};

    #[derive(
        Copy,
        Clone,
        PartialEq,
        Eq,
        PartialOrd,
        Ord,
        ValueEnum,
        Serialize,
        Deserialize,
        Debug,
        Display,
    )]
    #[serde(rename_all = "lowercase")]
    #[strum(serialize_all = "snake_case")]
    pub enum IssueAction {
        Opened,
        Edited,
        Deleted,
        Pinned,
        Unpinned,
        Closed,
        Reopened,
        Assigned,
        Unassigned,
        Labeled,
        Unlabeled,
        Locked,
        Unlocked,
        Transferred,
        Milestoned,
        Demilestoned,
    }
    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    pub struct IssueEvent {
        pub action: IssueAction,
        pub sender: User,
        pub repository: Repository,
    }
}

pub mod pr {
    use clap::ValueEnum;
    use serde::{Deserialize, Serialize};
    use strum_macros::Display;

    use super::{Repository, User};

    #[derive(
        Copy,
        Clone,
        PartialEq,
        Eq,
        PartialOrd,
        Ord,
        ValueEnum,
        Serialize,
        Deserialize,
        Debug,
        Display,
    )]
    #[serde(rename_all = "lowercase")]
    #[strum(serialize_all = "snake_case")]
    pub enum PRAction {
        Opened,
        Edited,
        Closed,
        Reopened,
        Assigned,
        Unassigned,
        ReviewRequested,
        ReviewRequestRemoved,
        Labeled,
        Unlabeled,
        Synchronized,
    }
    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    pub struct PREvent {
        pub action: PRAction,
        pub sender: User,
        pub repository: Repository,
    }
}


