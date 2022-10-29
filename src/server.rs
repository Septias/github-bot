use anyhow::{anyhow, bail};
use serde::{Deserialize, Serialize};
use std::{
    string::ParseError,
    sync::{Arc, Mutex},
};
use thiserror::Error;
use web_server::{HttpMethod, Request, Response};

use self::{issue::IssueEvent, pr::PREvent};

struct Server {
    storage: String,
}

fn make_str(httpmethod: HttpMethod) -> &'static str {
    match httpmethod {
        HttpMethod::GET => "get",
        HttpMethod::POST => "post",
        _ => "some method",
    }
}

#[derive(Error, Debug)]
enum Error {
    #[error("Unable to parse Request")]
    ParseError(#[from] serde_json::Error),

    #[error("Not Covered")]
    NotCovered,

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct User {
    pub login: String,
}

#[derive(Debug)]
enum Event {
    Issue(IssueEvent),
    PR(PREvent),
}
mod issue {
    use serde::{Deserialize, Serialize};

    use super::User;

    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    #[serde(rename_all = "lowercase")]
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
    }
}

mod pr {
    use serde::{Deserialize, Serialize};

    use super::User;

    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    #[serde(rename_all = "lowercase")]
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
    }
}

fn receive_webhoook(req: Request) -> Result<Event, Error> {
    match req.params.get("X-GitHub-Event") {
        Some(event_type) if event_type == "pull_request" => Ok(Event::Issue(
            serde_json::from_str::<IssueEvent>(&req.get_body())?,
        )),
        Some(event_type) if event_type == "issue" => {
            Ok(Event::PR(serde_json::from_str::<PREvent>(&req.get_body())?))
        }
        Some(_) => Err(Error::NotCovered),
        None => Err(Error::Other(anyhow!("Missing header `X-GitHub-Event`"))),
    }
}

pub fn start_server() {
    web_server::new()
        .post(
            "/receive",
            Box::new(|req, resp| {
                receive_webhoook(req);
                resp
            }),
        )
        .launch(8080);
}

#[cfg(test)]
mod tests {
    use crate::server::{
        issue::{IssueAction, IssueEvent},
        pr::{PRAction, PREvent},
        User,
    };

    use super::Event;

    #[test]
    fn test_issue_closed() {
        let mock = include_str!("../mock/issue_close.json");
        assert_eq!(
            serde_json::from_str::<IssueEvent>(mock).unwrap(),
            IssueEvent {
                action: IssueAction::Closed,
                sender: User {
                    login: "Septias".to_owned()
                }
            }
        );
    }
    #[test]
    fn test_issue_opened() {
        let mock = include_str!("../mock/issue_open.json");
        assert_eq!(
            serde_json::from_str::<IssueEvent>(mock).unwrap(),
            IssueEvent {
                action: IssueAction::Opened,
                sender: User {
                    login: "Septias".to_owned()
                }
            }
        );
    }
    fn test_pr_closed() {
        let mock = include_str!("../mock/pr_closed.json");
        assert_eq!(
            serde_json::from_str::<PREvent>(mock).unwrap(),
            PREvent {
                action: PRAction::Closed,
                sender: User {
                    login: "Septias".to_owned()
                }
            }
        );
    }
    #[test]
    fn test_pr_opened() {
        let mock = include_str!("../mock/pr_opened.json");
        assert_eq!(
            serde_json::from_str::<PREvent>(mock).unwrap(),
            PREvent {
                action: PRAction::Opened,
                sender: User {
                    login: "Septias".to_owned()
                }
            }
        );
    }
}
