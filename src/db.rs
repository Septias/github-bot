use crate::parser::{Commands, Family};
use anyhow::{bail, Result};
use deltachat::chat::ChatId;
use surrealdb::{
    sql::{Number, Object, Value},
    Datastore, Session,
};

#[derive(Default)]
pub struct Repository<'a> {
    pub name: &'a str,
    pub owner: &'a str,
    pub hook_id: usize,
    pub id: usize,
    pub url: &'a str,
}

pub struct DB {
    db: Datastore,
    session: Session,
}

#[allow(unused)]
impl DB {
    pub async fn new(store: &str) -> Self {
        let db = Datastore::new(store).await.unwrap();
        Self {
            db,
            session: Session::for_kv().with_ns("bot").with_db("bot"),
        }
    }

    async fn execute(&self, ast: &str) -> Result<Vec<surrealdb::Response>, surrealdb::Error> {
        self.db.execute(ast, &self.session, None, false).await
    }

    /// Return which db_table has to be queried for the specific action
    fn get_list_prefix(family: Family) -> String {
        match family {
            crate::parser::Family::Pr { pr_action } => format!("pr_{}", pr_action),
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
                let numbers = unwrap_array(obj.values().next().unwrap());
                if let Value::Array(arr) = numbers {
                    let ids = arr
                        .iter()
                        .map(|num| {
                            if let Value::Number(Number::Int(chat_id)) = num {
                                return ChatId::new(*chat_id as u32);
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

    /// Add a repository to the collection of repositories
    pub async fn add_repository<'a>(&self, repo: Repository<'a>) -> Result<()> {
        let Repository {
            name,
            hook_id,
            id,
            url,
            owner,
        } = repo;
        let stm = format!("CREATE repo:{id} SET repo_id = {id}, url = '{url}', hook_id = {hook_id}, owner='{owner}', name = '{name}'");
        self.execute(&stm).await?;
        Ok(())
    }

    /// Remove repository from the collection of repositories
    pub async fn remove_repository(&self, id: usize) -> Result<()> {
        self.execute(&format!("DELETE repos:{id}")).await?;
        Ok(())
    }

    /// Get the ids of all available repositories
    pub async fn get_repository_ids(&self) -> Result<Vec<usize>> {
        let stm = include_str!("queries/repo_ids.sql");
        let mut resp = self.execute(stm).await?;
        let mut resp = resp.remove(0).result?;

        if let Value::Array(arr) = resp {
            Ok(arr
                .into_iter()
                .filter_map(|obj| {
                    if let Value::Object(obj) = obj {
                        let Object(inner) = obj;
                        Some(inner.into_values().next().unwrap().as_int() as usize)
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>())
        } else {
            bail!("Error while retrieving repo ids")
        }
    }

    /// Get the hook-id of one repository
    pub async fn get_hook_id(&self, id: usize) -> Result<usize> {
        let mut resp = self
            .execute(&format!("SELECT hook_id FROM repo:{id}"))
            .await?;
        let mut resp = resp.remove(0).result?;

        if let Value::Array(mut arr) = resp {
            if let Value::Object(obj) = arr.remove(0) {
                let Object(inner) = obj;
                return Ok(inner.into_values().next().unwrap().as_int() as usize);
            }
        };
        bail!("something went wrong")
    }

    /// Get the owner of one repository
    pub async fn get_owner(&self, id: usize) -> Result<String> {
        let mut resp = self
            .execute(&format!("SELECT owner FROM repo:{id}"))
            .await?;
        let mut resp = resp.remove(0).result?;

        if let Value::Array(mut arr) = resp {
            if let Value::Object(obj) = arr.remove(0) {
                let Object(inner) = obj;
                return Ok(inner.into_values().next().unwrap().as_string());
            }
        };
        bail!("something went wrong")
    }

    /// Get the name of one repository
    pub async fn get_name(&self, id: usize) -> Result<String> {
        let mut resp = self.execute(&format!("SELECT name FROM repo:{id}")).await?;
        let mut resp = resp.remove(0).result?;

        if let Value::Array(mut arr) = resp {
            if let Value::Object(obj) = arr.remove(0) {
                let Object(inner) = obj;
                return Ok(inner.into_values().next().unwrap().as_string());
            }
        };
        bail!("something went wrong")
    }
}

/// Takes a surreal::Value and tries to unwrap it as long as
/// it only is an array with one element which is another array
fn unwrap_array(val: &Value) -> &Value {
    if let Value::Array(arr) = val {
        if arr.len() == 1 && !matches!(arr[0], Value::Number(_)) {
            return unwrap_array(&arr[0]);
        }
    };
    val
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_get_repository_ids() {
        let db = DB::new("memory").await;
        db.add_repository(Repository {
            id: 12,
            ..Default::default()
        })
        .await
        .unwrap();
        assert_eq!(db.get_repository_ids().await.unwrap(), [12]);
    }

    #[tokio::test]
    async fn test_remove() {
        let db = DB::new("memory").await;
        db.add_repository(Repository {
            id: 12,
            ..Default::default()
        })
        .await
        .unwrap();
        db.remove_repository(12).await.unwrap();
        assert_eq!(db.get_repository_ids().await.unwrap(), [] as [usize; 0]);
    }

    #[tokio::test]
    async fn test_get_hook_id() {
        let db = DB::new("memory").await;
        db.add_repository(Repository {
            hook_id: 23,
            id: 12,
            url: "",
            ..Default::default()
        })
        .await
        .unwrap();
        assert_eq!(db.get_hook_id(12).await.unwrap(), 23);
    }

    #[tokio::test]
    async fn test_get_owner() {
        let db = DB::new("memory").await;
        db.add_repository(Repository {
            owner: "Me",
            id: 12,
            ..Default::default()
        })
        .await
        .unwrap();
        assert_eq!(db.get_owner(12).await.unwrap(), "Me".to_string());
    }

    #[tokio::test]
    async fn test_get_name() {
        let db = DB::new("memory").await;
        db.add_repository(Repository {
            name: "ligma",
            id: 12,
            ..Default::default()
        })
        .await
        .unwrap();
        assert_eq!(db.get_name(12).await.unwrap(), "ligma".to_string());
    }
}
