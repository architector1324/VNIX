use alloc::format;
use alloc::string::String;

use core::fmt::{Display, Formatter};

use sha3::{Digest, Sha3_256};
use base64ct::{Base64, Encoding};

use super::kern::KernErr;
use super::unit::Unit;
use super::user::Usr;

#[derive(Debug)]
pub struct Msg {
    pub msg: Unit,
    pub ath: String,
    pub hash: String,
    pub sign: String
}

impl Display for Msg {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write!(f, "{{ath:{} msg:{} hsh:{} sign:{}}}", self.ath, self.msg, self.hash, self.sign)
    }
}

impl Msg {
    pub fn new(usr: Usr, msg: Unit) -> Result<Self, KernErr> {
        let s = format!("{}", msg);

        let h = Sha3_256::digest(s.as_bytes());

        let hash = Base64::encode_string(&h[..]);
        let sign = usr.sign(&msg)?;

        Ok(Msg {
            ath: usr.name,
            msg,
            hash: hash.into(),
            sign
        })
    }
}
