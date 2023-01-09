use core::str::Chars;
use core::fmt::{Display, Formatter, Write};
use heapless::{String, Vec, LinearMap, pool::Box};

use super::kern::{Kern, KernErr};


#[derive(Debug)]
pub enum UnitParseErr {
    NotNone,
    NotBool,
    NotByte,
    NotInt,
    NotDec,
    NotStr,
    NotPair,
    NotList,
    NotMap,
    NotUnit,
    NotClosedBrackets,
    NotClosedQuotes,
    MissedSeparator,
    MissedDot,
    MissedPartAfterDot
}

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
            Unit::None => write!(f, "-"),
            Unit::Bool(b) => {
                if *b {
                    write!(f, "t")
                } else {
                    write!(f, "f")
                }
            },
            Unit::Byte(b) => write!(f, "{:#02x}", b),
            Unit::Int(i) => write!(f, "{}", i),
            Unit::Dec(d) => write!(f, "{}", d),
            Unit::Str(s) => {
                if s.as_str().chars().all(|c| c.is_alphanumeric()) {
                    write!(f, "{}", s)
                } else {
                    write!(f, "`{}`", s)
                }
            },
            Unit::Pair(p) => write!(f, "({} {})", p.0, p.1),
            Unit::Lst(lst) => {
                write!(f, "[")?;

                for (i, u) in lst.iter().enumerate() {
                    if i == lst.len() - 1 {
                        write!(f, "{}", u)?;
                    } else {
                        write!(f, "{} ", u)?;
                    }
                }

                write!(f, "]")
            },
            Unit::Map(map) => {
                write!(f, "{{")?;

                for (i, (u0, u1)) in map.iter().enumerate() {
                    if i == map.len() - 1 {
                        write!(f, "{}:{}", u0, u1)?;
                    } else {
                        write!(f, "{}:{} ", u0, u1)?;
                    }
                }

                write!(f, "}}")
            }
        }
    }
}

impl Unit {
    fn parse_none<'a>(mut it: Chars<'a>) -> Result<(Self, Chars<'a>), KernErr> {
        if let Some(c) = it.next() {
            if c == '-' {
                return Ok((Unit::None, it));
            }
        }
        Err(KernErr::ParseErr(UnitParseErr::NotNone))
    }

    fn parse_bool<'a>(mut it: Chars<'a>) -> Result<(Self, Chars<'a>), KernErr> {
        if let Some(c) = it.next() {
            // bool
            if c == 't' {
                return Ok((Unit::Bool(true), it));
            }

            if c == 'f' {
                return Ok((Unit::Bool(false), it));
            }
        }
        Err(KernErr::ParseErr(UnitParseErr::NotBool))
    }

    fn parse_byte<'a>(mut it: Chars<'a>) -> Result<(Self, Chars<'a>), KernErr> {
        if let Some(s) = it.as_str().get(0..4) {
            it.next().unwrap();
            it.next().unwrap();
            it.next().unwrap();
            it.next().unwrap();

            if let Ok(v) = u8::from_str_radix(s.trim_start_matches("0x"), 16) {
                return Ok((Unit::Byte(v), it))
            }
        }

        Err(KernErr::ParseErr(UnitParseErr::NotByte))
    }

    fn parse_int<'a>(mut it: Chars<'a>) -> Result<(Self, Chars<'a>), KernErr> {
        let mut s = String::<256>::new();
        let mut tmp = it.clone();

        while let Some(c) = it.next() {
            if !c.is_numeric() {
                break;
            }

            s.push(c).map_err(|_| KernErr::MemoryOut)?;
            tmp = it.clone();
        }

        if let Ok(v) = s.parse::<i32>() {
            return Ok((Unit::Int(v), tmp));
        }

        Err(KernErr::ParseErr(UnitParseErr::NotInt))
    }

    fn parse_dec<'a>(it: Chars<'a>) -> Result<(Self, Chars<'a>), KernErr> {
        if let Ok((fst, mut it)) = Unit::parse_int(it) {
            if let Some(c) = it.next() {
                if c != '.' {
                    return Err(KernErr::ParseErr(UnitParseErr::MissedDot));
                }

                if let Ok((scd, it)) = Unit::parse_int(it) {
                    let mut s = String::<256>::new();
                    write!(s, "{}.{}", fst, scd).map_err(|_| KernErr::MemoryOut)?;

                    let out = s.parse::<f32>().map_err(|_| KernErr::ParseErr(UnitParseErr::NotDec))?;

                    return Ok((Unit::Dec(out), it));
                } else {
                    return Err(KernErr::ParseErr(UnitParseErr::MissedPartAfterDot));
                }
            }
        }
        Err(KernErr::ParseErr(UnitParseErr::NotDec))
    }

    fn parse_str<'a>(mut it: Chars<'a>) -> Result<(Self, Chars<'a>), KernErr> {
        if let Some(c) = it.next() {
            // `complex string`
            if c == '`' {
                let mut s = String::<256>::new();
                let mut tmp = it.clone();

                while let Some(c) = it.next() {
                    if c == '`' {
                        break;
                    }

                    s.push(c).map_err(|_| KernErr::MemoryOut)?;
                    tmp = it.clone();
                }

                if let Some(c) = tmp.next() {
                    if c == '`' {
                        return Ok((Unit::Str(s), tmp));
                    } else {
                        return Err(KernErr::ParseErr(UnitParseErr::NotClosedQuotes));
                    }
                } else {
                    return Err(KernErr::ParseErr(UnitParseErr::NotClosedQuotes));
                }
            }

            // abc123
            if c.is_alphanumeric() {
                let mut s = String::<256>::new();
                let mut tmp = it.clone();

                s.push(c).map_err(|_| KernErr::MemoryOut)?;

                while let Some(c) = it.next() {
                    if !c.is_alphanumeric() {
                        break;
                    }

                    s.push(c).map_err(|_| KernErr::MemoryOut)?;
                    tmp = it.clone();
                }

                return Ok((Unit::Str(s), tmp));
            }
        }
        Err(KernErr::ParseErr(UnitParseErr::NotStr))
    }

    fn parse_pair<'a>(mut it: Chars<'a>, kern: &mut Kern) -> Result<(Self, Chars<'a>), KernErr> {
        if let Some(c) = it.next() {
            if c == '(' {
                let u0 = Unit::parse(it, kern)?;
                it = u0.1;

                if let Some(c) = it.next() {
                    if c == ' ' {
                        let u1 = Unit::parse(it, kern)?;
                        it = u1.1;

                        if let Some(c) = it.next() {
                            if c == ')' {
                                return Ok((
                                    Unit::Pair((
                                        kern.unit(u0.0)?,
                                        kern.unit(u1.0)?
                                    )),
                                    it
                                ))
                            } else {
                                return Err(KernErr::ParseErr(UnitParseErr::NotClosedBrackets));
                            }
                        }
                    } else {
                        return Err(KernErr::ParseErr(UnitParseErr::MissedSeparator));
                    }
                } else {
                    return Err(KernErr::ParseErr(UnitParseErr::NotClosedBrackets));
                }
            }
        }

        Err(KernErr::ParseErr(UnitParseErr::NotPair))
    }

    fn parse_list<'a>(mut it: Chars<'a>, kern: &mut Kern) -> Result<(Self, Chars<'a>), KernErr> {
        if let Some(c) = it.next() {
            if c == '[' {
                let mut lst = Vec::new();

                loop {
                    let u = Unit::parse(it, kern)?;
                    it = u.1;

                    if let Some(c) = it.next() {
                        if c == ' ' {
                            let u = kern.unit(u.0)?;
                            lst.push(u).map_err(|_| KernErr::MemoryOut)?;
                        } else if c == ']' {
                            let u = kern.unit(u.0)?;
                            lst.push(u).map_err(|_| KernErr::MemoryOut)?;

                            return Ok((Unit::Lst(lst), it))
                        } else {
                            return Err(KernErr::ParseErr(UnitParseErr::MissedSeparator));
                        }
                    } else {
                        return Err(KernErr::ParseErr(UnitParseErr::NotClosedBrackets));
                    }
                }
            }
        }

        Err(KernErr::ParseErr(UnitParseErr::NotList))
    }

    fn parse_map<'a>(mut it: Chars<'a>, kern: &mut Kern) -> Result<(Self, Chars<'a>), KernErr> {
        if let Some(c) = it.next() {
            if c == '{' {
                let mut map = LinearMap::new();

                loop {
                    let u0 = Unit::parse(it, kern)?;
                    it = u0.1;

                    if let Some(c) = it.next() {
                        if c == ':' {
                            let u1 = Unit::parse(it, kern)?;
                            it = u1.1;

                            if let Some(c) = it.next() {
                                if c == ' ' {
                                    let u0 = kern.unit(u0.0)?;
                                    let u1 = kern.unit(u1.0)?;

                                    map.insert(u0, u1).map_err(|_| KernErr::MemoryOut)?;
                                } else if c == '}' {
                                    let u0 = kern.unit(u0.0)?;
                                    let u1 = kern.unit(u1.0)?;

                                    map.insert(u0, u1).map_err(|_| KernErr::MemoryOut)?;

                                    return Ok((Unit::Map(map), it))
                                } else {
                                    return Err(KernErr::ParseErr(UnitParseErr::MissedSeparator));
                                }
                            } else {
                                return Err(KernErr::ParseErr(UnitParseErr::NotClosedBrackets));
                            }
                        } else {
                            return Err(KernErr::ParseErr(UnitParseErr::MissedSeparator));
                        }
                    } else {
                        return Err(KernErr::ParseErr(UnitParseErr::NotClosedBrackets));
                    }
                }
            }
        }

        Err(KernErr::ParseErr(UnitParseErr::NotMap))
    }

    pub fn parse<'a>(it: Chars<'a>, kern: &mut Kern) -> Result<(Self, Chars<'a>), KernErr> {
        // none
        if let Ok((u, it)) = Unit::parse_none(it.clone()) {
            return Ok((u, it));
        }

        // bool
        if let Ok((u, it)) = Unit::parse_bool(it.clone()) {
            return Ok((u, it));
        }

        // byte
        if let Ok((u, it)) = Unit::parse_byte(it.clone()) {
            return Ok((u, it));
        }

        // dec
        if let Ok((u, it)) = Unit::parse_dec(it.clone()) {
            return Ok((u, it));
        }

        // int
        if let Ok((u, it)) = Unit::parse_int(it.clone()) {
            return Ok((u, it));
        }

        // str
        if let Ok((u, it)) = Unit::parse_str(it.clone()) {
            return Ok((u, it));
        }

        // pair
        if let Ok((u, it)) = Unit::parse_pair(it.clone(), kern) {
            return Ok((u, it));
        }

        // list
        if let Ok((u, it)) = Unit::parse_list(it.clone(), kern) {
            return Ok((u, it));
        }

        // map
        if let Ok((u, it)) = Unit::parse_map(it.clone(), kern) {
            return Ok((u, it));
        }

        Err(KernErr::ParseErr(UnitParseErr::NotUnit))
    }
}
