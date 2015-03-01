#[allow(non_camel_case_types)]
#[derive(Copy, Debug, PartialEq)]
/// Response codes defined by the IRC protocol.
pub enum ResponseCode {
    /// `Welcome to the Internet Relay Network <nick>!<user>@<host>`
    RPL_WELCOME = 001,
    /// `"<name> :End of WHO list"`
    RPL_ENDOFWHO = 315,
    /// `<channel> <mode> <mode params>`
    RPL_CHANNELMODEIS = 324,
    /// `<channel> :No topic is set`
    RPL_NOTOPIC = 331,
    /// `<channel> <invitemask>`
    RPL_INVITELIST = 346,
    /// `<channel> :End of channel invite list`
    RPL_ENDOFINVITELIST = 347,
    /// `<channel> <exceptionmask>`
    RPL_EXCEPTLIST = 348,
    /// `<channel> :End of channel exception list `
    RPL_ENDOFEXCEPTLIST = 349,
    /// `<channel> <banmask>`
    RPL_BANLIST = 367,
    /// `"<channel> <user> <host> <server> <nick> ( "H" / "G" > ["*"] [ ( "@" / "+" ) ] :<hopcount> <real name>"`
    RPL_WHOREPLY = 352,
    /// `"( "=" / "*" / "@" ) <channel> :[ "@" / "+" ] <nick> *( " " [ "@" / "+" ] <nick> )`
    /// "@" is used for secret channels, "*" for private channels, and "=" for others (public channels).
    RPL_NAMREPLY = 353,
    /// `<channel> :End of NAMES list`
    RPL_ENDOFNAMES = 366,
    /// `<channel> :End of channel ban list`
    RPL_ENDOFBANLIST = 368,
    /// `<channel name> :No such channel`
    ERR_NOSUCHCHANNEL = 403,
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
    /// `<channel> :You're not channel operator`
    ERR_CHANOPRIVSNEEDED = 482,
    /// `:Cannot change mode for other users`
    ERR_USERSDONTMATCH = 502,
}

