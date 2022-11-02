use anyhow::bail;
use thiserror::Error;

const BODY: &str = r#"
{
    "name": "web",
    "active": true,
    "events": [
        "issues",
        "pull_request"
    ],
    "config": {
        "url": "http://188.68.57.24:8080/receive",
        "content_type": "json",
        "insecure_ssl": "0"
    }
}
"#;

#[derive(Error, Debug)]
pub enum HookError {
    #[error("Unknown error")]
    Unknown,

    #[error("The hook already exists")]
    AlreadyExists,
}

pub async fn create_hook(user: &str, repo: usize, key: &str) -> anyhow::Result<()> {
    let client = reqwest::Client::new();
    let url = format!("https://api.github.com/repos/{user}/{repo}/hooks");
    let res = client
        .post(&url)
        .header("Accept", "application/vnd.github+json")
        .header("Authorization", format!("Bearer {key}"))
        .header("User-Agent", "deltachat-github-bot")
        .body(BODY)
        .send()
        .await?;

    let status = res.status();
    if status == 201 {
        Ok(())
    } else if status == 422 {
        Err(HookError::AlreadyExists)?
    } else {
        Err(HookError::Unknown)?
    }
}

pub async fn remove_hook(user: &str, repo: usize, hook: usize, key: &str) -> anyhow::Result<()> {
    let client = reqwest::Client::new();
    let url = format!("https://api.github.com/repos/{user}/{repo}/hooks/{hook}");
    let res = client
        .delete(&url)
        .header("Accept", "application/vnd.github+json")
        .header("Authorization", format!("Bearer {key}"))
        .header("User-Agent", "deltachat-github-bot")
        .send()
        .await?;
    if res.status() == 204 {
        Ok(())
    } else {
        bail!("something went wrong: {}", res.status())
    }
}
