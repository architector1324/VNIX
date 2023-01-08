use heapless::String;
use core::fmt::{Display, Formatter};

use super::kern::KernErr;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Usr {
    pub name: String<256>,
    // pub key: [u8; 256]
}

impl Display for Usr {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        if self.name.contains(" ") {
            write!(f, "`{}`", self.name)
        } else {
            write!(f, "{}", self.name)
        }
    }
}

impl Usr {
    pub fn new(name: &str) -> Result<Self, KernErr> {
        Ok(Usr {
            name: name.into()
        })
    }
}
