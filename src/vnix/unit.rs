use core::{fmt::{Display, Formatter, write}, str::FromStr};
use heapless::{String, Vec, LinearMap};

use super::msg::MsgParseErr;


#[derive(Debug, PartialEq, Clone)]
pub enum Unit<'a> {
    None,
    Bool(bool),
    Byte(u8),
    Int(i32),
    Dec(f32),
    Str(String<256>),
    Lst(Vec<&'a Unit<'a>, 256>),
    Map(LinearMap<&'a Unit<'a>, &'a Unit<'a>, 256>)
}

impl<'a> Eq for Unit<'a> {}

impl<'a> Display for Unit<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        match self {
            Unit::None => write(f, core::format_args!("-")),
            Unit::Bool(b) => write(f, core::format_args!("{}", b)),
            Unit::Byte(b) => write(f, core::format_args!("{}", b)),
            Unit::Int(i) => write(f, core::format_args!("{}", i)),
            Unit::Dec(d) => write(f, core::format_args!("{}", d)),
            Unit::Str(s) => {
                if s.contains(" ") {
                    write(f, core::format_args!("`{}`", s))
                } else {
                    write(f, core::format_args!("{}", s))
                }
            },
            Unit::Lst(lst) => {
                write(f, core::format_args!("["))?;

                for (i, u) in lst.iter().enumerate() {
                    if i == lst.len() - 1 {
                        write(f, core::format_args!("{}", u))?;
                    } else {
                        write(f, core::format_args!("{} ", u))?;
                    }
                }

                write(f, core::format_args!("]"))
            },
            Unit::Map(map) => {
                write(f, core::format_args!("{{"))?;

                for (i, (u0, u1)) in map.iter().enumerate() {
                    if i == map.len() - 1 {
                        write(f, core::format_args!("{}:{}", u0, u1))?;
                    } else {
                        write(f, core::format_args!("{}:{} ", u0, u1))?;
                    }
                }

                write(f, core::format_args!("}}"))
            }
        }
    }
}

impl<'a> FromStr for Unit<'a> {
    type Err = MsgParseErr;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s == "-" {
            return Ok(Unit::None)
        }

        Err(MsgParseErr::NotUnit)
    }
}
