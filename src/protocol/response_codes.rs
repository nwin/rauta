#[allow(non_camel_case_types)]
#[derive(Copy, Debug)]
/// Response codes defined by the IRC protocol.
pub enum ResponseCode {
    ERR_INVALIDCAPCMD = 410,
    ERR_NEEDMOREPARAMS = 461,
}