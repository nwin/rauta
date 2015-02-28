//! Module containing everything related to users

#[derive(Debug)]
pub struct User {
    pub nick: String,
    pub user: String,
    pub host: String,
    pub realname: String,
    pub registered: bool
}
