#[allow(non_camel_case_types)]
#[derive(Copy, Debug, PartialEq)]
/// Response codes defined by the IRC protocol.
pub enum ResponseCode {
    /// `Welcome to the Internet Relay Network <nick>!<user>@<host>`
    RPL_WELCOME = 001,
    /// `<channel> :No topic is set`
    RPL_NOTOPIC = 331,
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
    ERR_ALREADYREGISTRED = 462,
    /// `<channel> :Cannot join channel (+l)`
    ERR_CHANNELISFULL = 471,
    /// `<char> :is unknown mode char to me for <channel>`
    ERR_UNKNOWNMODE = 472,
    /// `<channel> :Cannot join channel (+i)`
    ERR_INVITEONLYCHAN = 473,
    /// `<channel> :Cannot join channel (+b)`
    ERR_BANNEDFROMCHAN = 474,
    /// `<channel> :Cannot join channel (+k)`
    ERR_BADCHANNELKEY = 475,
}