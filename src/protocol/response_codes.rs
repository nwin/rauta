#[allow(non_camel_case_types)]
#[derive(Copy, Debug)]
/// Response codes defined by the IRC protocol.
pub enum ResponseCode {
    /// `Welcome to the Internet Relay Network <nick>!<user>@<host>`
    RPL_WELCOME = 001,
    /// `<subcommand> :<reason>`
    ERR_INVALIDCAPCMD = 410,
    /// `:No nickname given`
    ERR_NONICKNAMEGIVEN = 431,
    /// `<nick> :Erroneous nickname`
    ERR_ERRONEUSNICKNAME = 432,
    /// `<nick> :Nickname is already in use`
    ERR_NICKNAMEINUSE = 433,
    /// `<command> :Not enough parameters`
    ERR_NEEDMOREPARAMS = 461,
    /// `:Unauthorized command (already registered)`
    ERR_ALREADYREGISTRED = 462
}