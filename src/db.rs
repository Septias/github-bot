use deltachat::chat::ChatId;
use surrealdb::{Datastore, Session};

use crate::parser::{Commands, EventType, Subscribeable};

pub enum DBAction {
    SubscribeIssuePull,
    SubscribeIssuePush,
    SubscribePRPush,
    SubscribePRPull,
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

    pub async fn add_listener(&self, command: Commands, chat: ChatId) {
        match command {
            Commands::Subscribe {
                repo,
                event_type,
                family,
            } => match family {
                Subscribeable::Issues => match event_type {
                    EventType::Push => {
                        self.execute(&format!(
                            "UPDATE {}:{} SET issue_pull += [{}]",
                            repo, repo, chat
                        ))
                        .await
                    }
                    EventType::Pull => todo!(),
                },
                Subscribeable::PR => todo!(),
            },
            Commands::Unsubscribe {
                repo,
                event_type,
                family,
            } => todo!(),
        }
    }
}
