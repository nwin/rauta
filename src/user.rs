//! Module containing everything related to users

#[derive(Debug, PartialEq)]
pub enum Status {
	Disconnected,
	Connected,
	RegistrationPending,
	Registered
}

#[derive(Debug)]
pub struct User {
    pub nick: String,
    pub user: String,
    pub host: String,
    pub realname: String,
    pub status: Status
}
