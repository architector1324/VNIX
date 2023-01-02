use core::fmt::{Display, Formatter, write};

use heapless::String;

#[derive(Debug, PartialEq, Clone)]
pub enum Unit {
    None,
    Bool(bool),
    Byte(u8),
    Int(i32),
    Dec(f32),
    Str(String<256>)
}

impl Eq for Unit {}


impl Display for Unit {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        match self {
            Unit::None => write(f, core::format_args!("-")),
            Unit::Str(s) => {
                if s.contains(" ") {
                    write(f, core::format_args!("`{}`", s))
                } else {
                    write(f, core::format_args!("{}", s))
                }
            }
            _ => write(f, core::format_args!("{}", self))
        }
    }
}
