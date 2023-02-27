//! Local server to receive Githubs webhooks
use anyhow::anyhow;
use log::{error, info};
use std::sync::Arc;
use thiserror::Error;
use tide::{Request, Server as TideServer};
use tokio::sync::mpsc::Sender;

use crate::{
    shared::{issue::IssueEvent, pr::PREvent, WebhookEvent},
    PORT,
};

#[derive(Error, Debug)]
enum Error {
    #[error("Unable to parse Request")]
    Parse(#[from] serde_json::Error),

    #[error("Not Covered")]
    NotCovered,

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

#[derive(Clone)]
pub struct ServerState {
    pub channel: Arc<Sender<WebhookEvent>>,
}

pub struct Server {
    server: TideServer<ServerState>,
}

async fn handler(mut req: Request<ServerState>) -> tide::Result {
    match receive_webhoook(&mut req).await {
        Ok(event) => {
            info!("received webhook");
            req.state().channel.send(event).await.unwrap();
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
        let handle = tokio::spawn(async move {
            server.listen(format!("0.0.0.0{PORT}")).await.unwrap();
        });
        handle
    }

    pub fn stop(self) {}
}

async fn receive_webhoook(req: &mut Request<ServerState>) -> Result<WebhookEvent, Error> {
    match req.header("X-GitHub-Event") {
        Some(event_type) if event_type == "issues" => Ok(WebhookEvent::Issue(
            serde_json::from_str::<IssueEvent>(&req.body_string().await.unwrap())?,
        )),
        Some(event_type) if event_type == "pull_request" => Ok(WebhookEvent::PR(
            serde_json::from_str::<PREvent>(&req.body_string().await.unwrap())?,
        )),
        Some(_) => Err(Error::NotCovered),
        None => Err(Error::Other(anyhow!("Missing header `X-GitHub-Event`"))),
    }
}

#[cfg(test)]
mod tests {
    use crate::shared::{
        issue::{Issue, IssueAction, IssueEvent},
        pr::{PRAction, PREvent, PR},
        Repository, User,
    };

    #[test]
    fn test_issue_closed() {
        let mock = include_str!("../mock/issue_close.json");
        assert_eq!(
            serde_json::from_str::<IssueEvent>(mock).unwrap(),
            IssueEvent {
                action: IssueAction::Closed,
                sender: User {
                    login: "Septias".to_owned()
                },
                repository: Repository {
                    id: 558781383,
                    name: "testrepo".to_owned(),
                    url: "https://api.github.com/repos/Septias/testrepo".to_string(),
                },
                issue: Issue {
                    id: 1427422736,
                    title: "test".to_owned(),
                    url: "https://api.github.com/repos/Septias/testrepo/issues/1".to_owned(),
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
                },
                repository: Repository {
                    id: 558781383,
                    name: "testrepo".to_owned(),
                    url: "https://api.github.com/repos/Septias/testrepo".to_string(),
                },
                issue: Issue {
                    id: 1427422736,
                    title: "test".to_owned(),
                    url: "https://api.github.com/repos/Septias/testrepo/issues/1".to_owned(),
                }
            }
        );
    }

    #[test]
    fn test_pr_closed() {
        let mock = include_str!("../mock/pr_closed.json");
        assert_eq!(
            serde_json::from_str::<PREvent>(mock).unwrap(),
            PREvent {
                action: PRAction::Closed,
                sender: User {
                    login: "Septias".to_owned()
                },
                repository: Repository {
                    id: 558781383,
                    name: "testrepo".to_owned(),
                    url: "https://api.github.com/repos/Septias/testrepo".to_string(),
                },
                pull_request: PR {
                    id: 1103900553,
                    title: "PR 2".to_owned(),
                    url: "https://api.github.com/repos/Septias/testrepo/pulls/3".to_owned(),
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
                },
                repository: Repository {
                    url: "https://api.github.com/repos/Septias/testrepo".to_string(),
                    id: 558781383,
                    name: "testrepo".to_owned(),
                },
                pull_request: PR {
                    id: 1103900553,
                    title: "PR 2".to_owned(),
                    url: "https://api.github.com/repos/Septias/testrepo/pulls/3".to_owned(),
                }
            }
        );
    }
}
