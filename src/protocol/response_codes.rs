#[allow(non_camel_case_types)]
#[derive(Copy, Debug)]
/// Response codes defined by the IRC protocol.
pub enum ResponseCode {
    /// <subcommand> :<reason>  
    ERR_INVALIDCAPCMD = 410,
    /// :No nickname given  
    ERR_NONICKNAMEGIVEN = 431,
    /// <nick> :Erroneous nickname
    ERR_ERRONEUSNICKNAME = 432,
    /// <nick> :Nickname is already in use
    ERR_NICKNAMEINUSE = 433,
    /// <command> :Not enough parameters
    ERR_NEEDMOREPARAMS = 461,
}