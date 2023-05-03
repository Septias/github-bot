//! Some shared types

use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug)]
pub struct AppInfo {
    name: String,
    author_name: String,
    author_email: String,
    source_code_url: String,
    description: String,
}