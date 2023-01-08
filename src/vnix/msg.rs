use heapless::pool::Box;
use heapless::String;
use core::fmt::{Display, Formatter, Write};

use sha3::{Digest, Sha3_256};
use base64ct::{Base64, Encoding};

use super::kern::KernErr;
use super::unit::Unit;
use super::user::Usr;

#[derive(Debug)]
pub struct Msg<'a> {
    pub msg: Box<Unit>,
    pub ath: &'a Usr,
    pub hash: String<256>
}

impl<'a> Display for Msg<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write!(f, "{{ath:{} msg:{} hsh:{}}}", self.ath, self.msg, self.hash)
    }
}

impl<'a> Msg<'a> {
    pub fn new(ath: &'a Usr, msg: Box<Unit>) -> Result<Self, KernErr> {
        let mut s = String::<256>::new();
        write!(s, "{}", msg).map_err(|_| KernErr::MemoryOut)?;

        let h = Sha3_256::digest(s.as_bytes());
        let mut buf = [0; 256];

        let hash = Base64::encode(&h[..], &mut buf).map_err(|_| KernErr::EncodeFault)?;

        Ok(Msg {
            ath,
            msg,
            hash: hash.into()
        })
    }
}