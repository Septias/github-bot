use crate::{
    parser::{Commands, EventFamily},
    shared::{issue::IssueAction, pr::PRAction},
};
use deltachat::chat::ChatId;
use log::info;
use surrealdb::{Datastore, Session};

pub enum WebhookAction {
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
        self.db
            .execute(ast, &self.session, None, false)
            .await
            .unwrap();
    }

    async fn execute(&self, ast: &str) -> Result<Vec<surrealdb::Response>, surrealdb::Error> {
        self.db.execute(ast, &self.session, None, false).await
    }

    pub async fn add_subscriber(&self, command: Commands, chat: ChatId) {
        let (list, repo) = match command {
            Commands::Subscribe {
                repo,
                issue_action,
                family,
            } => match family {
                EventFamily::Issues => (format!("issue_{}", issue_action), repo),
                EventFamily::PR => (format!("pr_{}", issue_action), repo),
            },
            _ => panic!("you can't use `add_subscriber` to remove a subsriber"),
        };
        self.execute(&format!(
            "UPDATE {repo}:{repo} SET {list} += [{}]",
            chat.to_u32()
        ))
        .await
        .unwrap();
    }

    pub async fn get_subscribers(&self, repo: usize, action: WebhookAction) {
        let list = match action {
            WebhookAction::Issue(issue_action) => format!("issue_{issue_action}"),
            WebhookAction::Pr(pr_action) => format!("pr_{pr_action}"),
        };
        let query = self.execute(&format!("SELECT {list} FROM {repo}")).await;

        info!("{:?}", query);
    }
}
