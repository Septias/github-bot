use std::collections::HashMap;

use crate::{
    parser::{Commands, EventFamily},
    shared::{issue::IssueAction, pr::PRAction},
};
use deltachat::chat::ChatId;
use surrealdb::{Datastore, Session};

pub enum WebhookActions {
    Issue(IssueAction),
    Pr(PRAction),
}

pub struct DB {
    db: Datastore,
    session: Session,
}

impl DB {
    pub async fn new() -> Self {
        let db = Datastore::new("file://temp.db").await.unwrap();
        Self {
            db,
            session: Session::for_kv(),
        }
    }
    pub async fn init(&self) {
        let ast = include_str!("statements/initdb.sql");
        let sess = self.db.execute(ast, &self.session, None, false).await;
    }

    async fn execute(&self, ast: &str) {
        self.db.execute(ast, &self.session, None, false);
    }

    pub async fn add_subscriber(&self, command: Commands, chat: ChatId) {
        let (list, repo) = match command {
            Commands::Subscribe {
                repo,
                issue_action,
                family,
            } => match family {
                EventFamily::Issues => (format!("pr_{}", issue_action), repo),
                EventFamily::PR => (format!("pr_{}", issue_action), repo),
            },
            _ => panic!("trying to remove a listener is permitted"),
        };
        self.execute(&format!("UPDATE {repo}:{repo} SET {list} += [{chat}]"))
            .await;
    }

    pub async fn get_subscribers(&self, repo: usize, action: WebhookActions) {
        let list = match action {
            WebhookActions::Issue(issue_action) => format!("issue_{issue_action}"),
            WebhookActions::Pr(pr_action) => format!("pr_{pr_action}"),
        };
        self.execute(&format!("SELECT {list} FROM {repo}:{repo}"))
            .await;
    }
}
