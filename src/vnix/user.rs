use heapless::String;
use core::fmt::{Display, Formatter};

#[derive(Debug, Clone)]
pub struct Usr {
    pub name: String<256>
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
