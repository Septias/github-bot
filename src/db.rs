use crate::parser::{Commands, Family};
use anyhow::Result;
use deltachat::chat::ChatId;
use surrealdb::{
    sql::{Number, Value},
    Datastore, Session,
};

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

    /// Return which db_table has to be queried for the specific action
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

    /// Add a ChatId as subscriber to an action
    pub async fn add_subscriber(&self, command: Commands, chat: ChatId) {
        self.change_subscriber(command, chat, true).await;
    }

    /// Remove a ChatId from subscribers to an action
    pub async fn remove_subscriber(&self, command: Commands, chat: ChatId) {
        self.change_subscriber(command, chat, false).await;
    }

    /// Return all ChatIds which subscribed to to an action
    pub async fn get_subscribers(&self, repo: usize, family: Family) -> Result<Vec<ChatId>> {
        let list = Self::get_list_prefix(family);
        let mut query = self
            .execute(&format!("SELECT {list} FROM {repo}:{repo}"))
            .await?;

        let rsp1 = query.remove(0);
        if let Ok(val) = rsp1.result {
            let obj = unwrap_array(&val);
            if let Value::Object(obj) = obj {
                let numbers = unwrap_array(obj.values().nth(0).unwrap());
                if let Value::Array(arr) = numbers {
                    let ids = arr
                        .iter()
                        .map(|num| {
                            if let Value::Number(num) = num {
                                if let Number::Int(chat_id) = num {
                                    return ChatId::new(*chat_id as u32);
                                }
                            }
                            panic!("can't convert number");
                        })
                        .collect::<Vec<_>>();
                    return Ok(ids);
                };
            };
        };
        Ok(vec![])
    }
}

/// Takes a surreal::Value and tries to unwrap it as long as
/// It only is an array with one element which is another array
fn unwrap_array(val: &Value) -> &Value {
    if let Value::Array(arr) = val {
        if arr.len() == 1 && !matches!(arr[0], Value::Number(_)) {
            return unwrap_array(&arr[0]);
        }
    };
    val
}
