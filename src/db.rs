use crate::parser::{Commands, Family};
use anyhow::Result;
use deltachat::chat::ChatId;
use log::info;
use surrealdb::{Datastore, Session};

pub struct DB {
    db: Datastore,
    session: Session,
}

impl DB {
    pub async fn new() -> Self {
        let db = Datastore::new("file://bot.db").await.unwrap();
        Self {
            db,
            session: Session::for_kv().with_ns("bot").with_db("bot"),
        }
    }
    pub async fn init(&self) {
        let ast = include_str!("statements/initdb.sql");
        self.execute(ast).await.unwrap();
    }

    async fn execute(&self, ast: &str) -> Result<Vec<surrealdb::Response>, surrealdb::Error> {
        self.db.execute(&ast, &self.session, None, false).await
    }

    fn get_list_prefix(family: Family) -> String {
        match family {
            crate::parser::Family::PR { pr_action } => format!("pr_{}", pr_action),
            crate::parser::Family::Issue { issue_action } => {
                format!("issue_{}", issue_action)
            }
        }
    }

    async fn change_subscriber(&self, command: Commands, chat: ChatId, add: bool) {
        if let Commands::Subscribe { repo, family } = command {
            let list = Self::get_list_prefix(family);
            let action = if add { "+=" } else { "-=" };
            self.execute(&format!(
                "UPDATE {repo}:{repo} SET {list} {action} [{}]",
                chat.to_u32()
            ))
            .await
            .unwrap();
        }
    }

    pub async fn add_subscriber(&self, command: Commands, chat: ChatId) {
        self.change_subscriber(command, chat, true).await;
    }

    pub async fn _remove_subscriber(&self, command: Commands, chat: ChatId) {
        self.change_subscriber(command, chat, false).await;
    }

    pub async fn get_subscribers(&self, repo: usize, family: Family) -> Result<()> {
        let list = Self::get_list_prefix(family);
        let query = self
            .execute(&format!("SELECT {list} FROM {repo}:{repo}"))
            .await?;
        info!("{:#?}", query[0]);
        Ok(())
    }
}
