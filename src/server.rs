use anyhow::{anyhow, bail};
use log::error;
use serde::{Deserialize, Serialize};
use std::{
    string::ParseError,
    sync::{Arc, Mutex},
    thread,
};
use thiserror::Error;
use tide::{Request, Server as TideServer};
use tokio::sync::mpsc::Sender;

use self::{issue::IssueEvent, pr::PREvent};

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
pub enum WebhookEvent {
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

#[derive(Clone)]
pub struct ServerState {
    pub channel: Arc<Sender<WebhookEvent>>,
}

pub struct Server {
    server: TideServer<ServerState>,
}

async fn handler(mut req: Request<ServerState>) -> tide::Result {
    let event = match receive_webhoook(&mut req).await {
        Ok(event) => {
            req.state().channel.send(event).await;
        }
        Err(err) => error!("{err}"),
    };
    Ok("".into())
}

impl Server {
    pub fn new(channel: Sender<WebhookEvent>) -> Self {
        let mut server = tide::with_state(ServerState {
            channel: Arc::new(channel),
        });
        server.at("receive").post(handler);
        Self { server }
    }

    pub fn start(&self) -> tokio::task::JoinHandle<()> {
        let server = self.server.clone();
        tokio::spawn(async move {
            server.listen("0.0.0.0:8080");
        })
    }

    pub fn stop(self) {}
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

async fn receive_webhoook(req: &mut Request<ServerState>) -> Result<WebhookEvent, Error> {
    match req.param("X-GitHub-Event") {
        Ok(event_type) if event_type == "pull_request" => Ok(WebhookEvent::Issue(
            serde_json::from_str::<IssueEvent>(&req.body_string().await.unwrap())?,
        )),
        Ok(event_type) if event_type == "issue" => {
            Ok(WebhookEvent::PR(serde_json::from_str::<PREvent>(
                &req.body_string().await.unwrap(),
            )?))
        }
        Ok(_) => Err(Error::NotCovered),
        Err(_) => Err(Error::Other(anyhow!("Missing header `X-GitHub-Event`"))),
    }
}

#[cfg(test)]
mod tests {
    use crate::server::{
        issue::{IssueAction, IssueEvent},
        pr::{PRAction, PREvent},
        User,
    };

    use super::WebhookEvent;

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
