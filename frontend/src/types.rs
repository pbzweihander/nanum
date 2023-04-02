use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct User {
    pub primary_email: String,
    pub emails: Vec<String>,
    pub exp: i64,
}
