use core::fmt::{Display, Formatter, write};
use heapless::{String, Vec, LinearMap, pool::Box};

use super::{msg::MsgParseErr, kern::{Kern, KernErr}};


#[derive(Debug, PartialEq)]
pub enum Unit {
    None,
    Bool(bool),
    Byte(u8),
    Int(i32),
    Dec(f32),
    Str(String<256>),
    Pair((Box<Unit>, Box<Unit>)),
    Lst(Vec<Box<Unit>, 128>),
    Map(LinearMap<Box<Unit>, Box<Unit>, 128>)
}

impl Eq for Unit {}

impl Display for Unit {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        match self {
            Unit::None => write(f, core::format_args!("-")),
            Unit::Bool(b) => {
                if *b {
                    write(f, core::format_args!("t"))
                } else {
                    write(f, core::format_args!("f"))
                }
            },
            Unit::Byte(b) => write(f, core::format_args!("{:#02x}", b)),
            Unit::Int(i) => write(f, core::format_args!("{}", i)),
            Unit::Dec(d) => write(f, core::format_args!("{}", d)),
            Unit::Str(s) => {
                if s.contains(" ") {
                    write(f, core::format_args!("`{}`", s))
                } else {
                    write(f, core::format_args!("{}", s))
                }
            },
            Unit::Pair(p) => write(f, core::format_args!("({} {})", p.0, p.1)),
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

impl Unit {
    pub fn parse(s: &str, kern: &mut Kern) -> Result<Self, KernErr> {
        // none
        if s == "-" {
            return Ok(Unit::None);
        }

        // bool
        if s == "t" {
            return Ok(Unit::Bool(true));
        }

        if s == "f" {
            return Ok(Unit::Bool(false));
        }

        // int
        if let Ok(v) = s.parse::<i32>() {
            return Ok(Unit::Int(v));
        }

        // dec
        if let Ok(v) = s.parse::<f32>() {
            return Ok(Unit::Dec(v));
        }

        // byte
        if s.len() >= 2 {
            if let Ok(v) = u8::from_str_radix(s.trim_start_matches("0x"), 16) {
                return Ok(Unit::Byte(v));
            }
        }

        // str
        if s.starts_with("`") && s.ends_with("`") {
            return Ok(Unit::Str(s.strip_prefix("`").unwrap().strip_suffix("`").unwrap().into()));
        }

        if s.chars().all(|c| c.is_alphanumeric()) {
            return Ok(Unit::Str(s.into()));
        }

        // // pair
        // if s.starts_with("(") && s.ends_with(")") {
        //     if let Some(p) = s.strip_prefix("(").unwrap().strip_suffix(")").unwrap().split_once(" ") {
        //         kern.cli.println(core::format_args!("{:?}", p));

        //         let u0 = Unit::parse(p.0, kern)?;
        //         let u1 = Unit::parse(p.1, kern)?;

        //         return Ok(Unit::Pair((
        //             kern.unit(u0)?,
        //             kern.unit(u1)?
        //         )))
        //     }
        // }

        // // list
        // if s.starts_with("[") && s.ends_with("]") {
        //     let mut lst = Vec::new();

        //     let mut s = s.strip_prefix("[").unwrap().strip_suffix("]").unwrap();

        //     while let Some(p) = s.split_once(" ") {
        //         kern.cli.println(core::format_args!("{:?}", p));

        //         let u = Unit::parse(p.0, kern)?;
        //         lst.push(kern.unit(u)?).map_err(|_| KernErr::MemoryOut)?;

        //         s = p.1;
        //     }

        //     return Ok(Unit::Lst(lst))
        // }

        Err(KernErr::ParseErr(MsgParseErr::NotUnit))
    }
}
