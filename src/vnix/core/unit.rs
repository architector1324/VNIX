use core::str::Chars;
use core::fmt::{Display, Formatter};

use alloc::{format, vec};
use alloc::vec::Vec;
use alloc::boxed::Box;
use alloc::string::{String, ToString};

use super::kern::Addr;


#[derive(Debug)]
pub enum UnitParseErr {
    NotNone,
    NotBool,
    NotByte,
    NotInt,
    NotDec,
    NotStr,
    NotRef,
    NotStream,
    NotPair,
    NotList,
    NotMap,
    NotUnit,
    NotClosedBrackets,
    NotClosedQuotes,
    MissedSeparator,
    MissedWhitespace,
    UnexpectedEnd,
    UnexpectedChar,
    MissedDot,
    MissedPartAfterDot,
    RefNotString,
    RefInvalidPath,
    StreamInvalidServ,
}

#[derive(Debug, PartialEq, Clone)]
pub enum Unit {
    None,
    Bool(bool),
    Byte(u8),
    Int(i32),
    Dec(f32),
    Str(String),
    Ref(Vec<String>),
    Stream(Box<Unit>, (String, Addr)),
    Pair(Box<Unit>, Box<Unit>),
    Lst(Vec<Unit>),
    Map(Vec<(Unit, Unit)>)
}

#[derive(Debug, Clone)]
pub enum UnitBin {
    None = 0,
    Bool,
    Byte,
    Int,
    Dec,
    Str,
    Ref,
    Stream,
    AddrLoc,
    AddrRemote,
    Pair,
    Lst,
    Map,

    // optimization
    Zero,
    Int8,
    Int16,
    Size8,
    Size16,
    Size32
}

pub trait Schema: Clone {
    type Out;

    fn find(&self, glob: &Unit, u: &Unit) -> Option<Self::Out>;

    fn find_deep(&self, glob: &Unit, u: &Unit) -> Option<Self::Out> {
        match u {
            Unit::Ref(path) => {
                if let Some(u) = Unit::find_ref(path.into_iter().cloned(), glob) {
                    return self.find_deep(glob, &u);
                }
            },
            _ => return self.find(glob, u)
        }
        None
    }

    fn find_loc(&self, u: &Unit) -> Option<Self::Out> {
        self.find_deep(u, u)
    }
}

#[derive(Debug, Clone)]
pub struct SchemaNone;

#[derive(Debug, Clone)]
pub struct SchemaBool;

#[derive(Debug, Clone)]
pub struct SchemaByte;

#[derive(Debug, Clone)]
pub struct SchemaInt;

#[derive(Debug, Clone)]
pub struct SchemaDec;

#[derive(Debug, Clone)]
pub struct SchemaStr;

#[derive(Debug, Clone)]
pub struct SchemaRef;

#[derive(Debug, Clone)]
pub struct  SchemaUnit;

#[derive(Debug, Clone)]
pub struct SchemaPair<A, B>(pub A, pub B) where A: Schema, B: Schema;

#[derive(Debug, Clone)]
pub struct SchemaSeq<A>(pub A) where A: Schema;

#[derive(Debug, Clone)]
pub struct SchemaMapSeq<A, B>(pub A, pub B) where A: Schema, B: Schema;

#[derive(Debug, Clone)]
pub struct SchemaMapEntry<A>(pub Unit, pub A) where A: Schema;

#[derive(Debug, Clone)]
pub struct SchemaMap<A, B>(pub SchemaMapEntry<A>, pub B) where A: Schema, B: Schema;

#[derive(Debug, Clone)]
pub struct SchemaMapFirstRequire<A, B>(pub SchemaMapEntry<A>, pub B) where A: Schema, B: Schema;

#[derive(Debug, Clone)]
pub struct SchemaMapSecondRequire<A, B>(pub SchemaMapEntry<A>, pub B) where A: Schema, B: Schema;

#[derive(Debug, Clone)]
pub struct SchemaMapRequire<A, B>(pub SchemaMapEntry<A>, pub B) where A: Schema, B: Schema;

#[derive(Debug, Clone)]
pub enum Or<A, B> {
    First(A),
    Second(B)
}

#[derive(Debug, Clone)]
pub struct SchemaOr<A, B>(pub A, pub B) where A: Schema, B: Schema;

pub trait FromUnit: Sized {
    fn from_unit_loc(u: &Unit) -> Option<Self>;

    fn from_unit(_glob: &Unit, u: &Unit) -> Option<Self> {
        Self::from_unit_loc(u)
    }
}

pub struct DisplayShort<'a>(pub &'a Unit, pub usize);

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
                if s.as_str().chars().all(|c| c.is_alphanumeric() || c == '.' || c == '#' || c == '_') {
                    write!(f, "{}", s)
                } else {
                    write!(f, "`{}`", s.replace("\\r", "\r").replace("\\n", "\n"))
                }
            },
            Unit::Ref(path) => write!(f, "@{}", path.join(".")),
            Unit::Stream(msg, (serv, addr)) => write!(f, "{msg}@{serv}:{addr}"),
            Unit::Pair(u0, u1) => write!(f, "({} {})", u0, u1),
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

impl<'a> Display for DisplayShort<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        match self.0 {
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
                let mut s = s.clone();
                s.truncate(self.1);

                if s.len() >= self.1 {
                    s = format!("{}..", s);
                }

                if s.as_str().chars().all(|c| c.is_alphanumeric() || c == '.' || c == '#' || c == '_') {
                    write!(f, "{}", s)
                } else {
                    write!(f, "`{}`", s)
                }
            },
            Unit::Ref(path) => write!(f, "@{}", path.join(".")),
            Unit::Stream(msg, (serv, addr)) => write!(f, "{}@{serv}:{addr}", DisplayShort(msg, self.1)),
            Unit::Pair(u0, u1) => write!(f, "({} {})", DisplayShort(&u0, self.1), DisplayShort(&u1, self.1)),
            Unit::Lst(lst) => {
                write!(f, "[")?;

                for (i, u) in lst.iter().take(self.1).enumerate() {
                    if i == lst.len().min(self.1) - 1 && lst.len() > self.1 {
                        write!(f, "{}..", DisplayShort(&u, self.1))?;
                    } else if i == lst.len().min(self.1) - 1 {
                        write!(f, "{}", DisplayShort(&u, self.1))?;
                    } else {
                        write!(f, "{} ", DisplayShort(&u, self.1))?;
                    }
                }

                write!(f, "]")
            },
            Unit::Map(map) => {
                write!(f, "{{")?;

                for (i, (u0, u1)) in map.iter().take(self.1).enumerate() {
                    if i == map.len().min(self.1) - 1 && map.len() > self.1 {
                        write!(f, "{}:{}..", DisplayShort(&u0, self.1), DisplayShort(&u1, self.1))?;
                    } else if  i == map.len().min(self.1) - 1 {
                        write!(f, "{}:{}", DisplayShort(&u0, self.1), DisplayShort(&u1, self.1))?;
                    } else {
                        write!(f, "{}:{} ", DisplayShort(&u0, self.1), DisplayShort(&u1, self.1))?;
                    }
                }

                write!(f, "}}")
            }
        }
    }
}

impl Unit {
    pub fn size(&self) -> usize {
        match self {
            Unit::None | Unit::Bool(_) | Unit::Byte(_) | Unit::Int(_) | Unit::Dec(_) => core::mem::size_of::<Unit>(),
            Unit::Str(s) => s.len() + core::mem::size_of::<Unit>(),
            Unit::Ref(path) => path.into_iter().fold(0, |prev, s| prev + s.len()) + core::mem::size_of::<Unit>(),
            Unit::Stream(msg, _) => msg.size() + core::mem::size_of::<Unit>(),
            Unit::Pair(u0, u1) => u0.size() + u1.size() + core::mem::size_of::<Unit>(),
            Unit::Lst(lst) => lst.into_iter().fold(0, |prev, u| prev + u.size()) + core::mem::size_of::<Unit>(),
            Unit::Map(m) => m.into_iter().fold(0, |prev, (u0, u1)| prev + u0.size() + u1.size()) + core::mem::size_of::<Unit>()
        }
    }

    pub fn as_bytes(&self) -> Vec<u8> {
        match self {
            Unit::None => vec![UnitBin::None as u8],
            Unit::Bool(v) => vec![UnitBin::Bool as u8, if *v {1} else {0}],
            Unit::Byte(v) => vec![UnitBin::Byte as u8, *v],
            Unit::Int(v) => {
                match *v {
                    0 => vec![UnitBin::Zero as u8],
                    -128..=127 => vec![UnitBin::Int8 as u8, (*v as i8) as u8],
                    0..=255 => vec![UnitBin::Size8 as u8, *v as u8],
                    -32768..=32767 => {
                        let mut tmp = vec![UnitBin::Int16 as u8];
                        tmp.extend((*v as i16).to_le_bytes());
                        tmp
                    },
                    0..=65535 => {
                        let mut tmp = vec![UnitBin::Size16 as u8];
                        tmp.extend((*v as u16).to_le_bytes());
                        tmp
                    },
                    _ => {
                        let mut tmp = vec![UnitBin::Int as u8];
                        tmp.extend(v.to_le_bytes());
                        tmp
                    }
                }
            },
            Unit::Dec(v) => {
                let mut tmp = vec![UnitBin::Dec as u8];
                tmp.extend(v.to_le_bytes());
                tmp
            },
            Unit::Str(s) => {
                let mut tmp = vec![UnitBin::Str as u8];
                tmp.extend((s.len() as u32).to_le_bytes());
                tmp.extend(s.as_bytes());
                tmp
            },
            Unit::Ref(path) => {
                let mut tmp = vec![UnitBin::Ref as u8];
                let s = path.join(".");

                tmp.extend((s.len() as u32).to_le_bytes());
                tmp.extend(s.as_bytes());
                tmp
            },
            Unit::Stream(msg, (serv, addr)) => {
                let mut tmp = vec![UnitBin::Stream as u8];
                tmp.extend(msg.as_bytes());

                tmp.extend((serv.len() as u32).to_le_bytes());
                tmp.extend(serv.as_bytes());

                match addr {
                    Addr::Local => tmp.push(UnitBin::AddrLoc as u8),
                    Addr::Remote(addr) => {
                        tmp.push(UnitBin::AddrRemote as u8);
                        tmp.extend(addr.iter().flat_map(|e| e.to_le_bytes()));
                    }
                }
                tmp
            }
            Unit::Pair(u0, u1) => {
                let mut tmp = vec![UnitBin::Pair as u8];
                tmp.extend(u0.as_bytes());
                tmp.extend(u1.as_bytes());
                tmp
            },
            Unit::Lst(lst) => {
                let mut tmp = vec![UnitBin::Lst as u8];
                tmp.extend((lst.len() as u32).to_le_bytes());
                tmp.extend(lst.iter().flat_map(|u| u.as_bytes()));
                tmp
            },
            Unit::Map(m) => {
                let mut tmp = vec![UnitBin::Map as u8];
                tmp.extend((m.len() as u32).to_le_bytes());
                tmp.extend(m.iter().flat_map(|(u0, u1)| {
                    let mut tmp = u0.as_bytes();
                    tmp.extend(u1.as_bytes());
                    tmp
                }));
                tmp
            }
        }
    }

    fn parse_bytes_none<'a, I>(mut it: I) -> Result<(Unit, I), UnitParseErr> where I: Iterator<Item = &'a u8> {
        let b = it.next().ok_or(UnitParseErr::NotNone)?;

        if *b != UnitBin::None as u8 {
            return Err(UnitParseErr::NotNone)
        }
        Ok((Unit::None, it))
    }

    fn parse_bytes_bool<'a, I>(mut it: I) -> Result<(Unit, I), UnitParseErr> where I: Iterator<Item = &'a u8> {
        let b = it.next().ok_or(UnitParseErr::NotByte)?;

        if *b != UnitBin::Bool as u8 {
            return Err(UnitParseErr::NotByte);
        }

        let b = it.next().ok_or(UnitParseErr::NotByte)?;

        match *b {
            0 => return Ok((Unit::Bool(false), it)),
            1 => return Ok((Unit::Bool(true), it)),
            _ => return Err(UnitParseErr::NotByte)
        }
    }

    fn parse_bytes_byte<'a, I>(mut it: I) -> Result<(Unit, I), UnitParseErr> where I: Iterator<Item = &'a u8> {
        let b = it.next().ok_or(UnitParseErr::NotByte)?;

        if *b != UnitBin::Byte as u8 {
            return Err(UnitParseErr::NotByte);
        }

        let b = it.next().ok_or(UnitParseErr::NotByte)?;
        Ok((Unit::Byte(*b), it))
    }

    fn parse_bytes_int_zero<'a, I>(mut it: I) -> Result<(Unit, I), UnitParseErr> where I: Iterator<Item = &'a u8> {
        let b = it.next().ok_or(UnitParseErr::NotInt)?;

        if *b != UnitBin::Zero as u8 {
            return Err(UnitParseErr::NotInt);
        }

        Ok((Unit::Int(0), it))
    }

    fn parse_bytes_int_u8<'a, I>(mut it: I) -> Result<(Unit, I), UnitParseErr> where I: Iterator<Item = &'a u8> {
        let b = it.next().ok_or(UnitParseErr::NotInt)?;

        if *b != UnitBin::Size8 as u8 {
            return Err(UnitParseErr::NotInt);
        }

        let b = *it.next().ok_or(UnitParseErr::NotInt)?;
        Ok((Unit::Int(b as i32), it))
    }

    fn parse_bytes_int_i8<'a, I>(mut it: I) -> Result<(Unit, I), UnitParseErr> where I: Iterator<Item = &'a u8> {
        let b = it.next().ok_or(UnitParseErr::NotInt)?;

        if *b != UnitBin::Int8 as u8 {
            return Err(UnitParseErr::NotInt);
        }

        let b = *it.next().ok_or(UnitParseErr::NotInt)?;
        Ok((Unit::Int((b as i8) as i32), it))
    }

    fn parse_bytes_int_i16<'a, I>(mut it: I) -> Result<(Unit, I), UnitParseErr> where I: Iterator<Item = &'a u8> {
        let b = it.next().ok_or(UnitParseErr::NotInt)?;

        if *b != UnitBin::Int16 as u8 {
            return Err(UnitParseErr::NotInt);
        }

        let bytes = [
            *it.next().ok_or(UnitParseErr::NotInt)?,
            *it.next().ok_or(UnitParseErr::NotInt)?,
        ];

        let v = i16::from_le_bytes(bytes);
        Ok((Unit::Int(v as i32), it))
    }

    fn parse_bytes_int_u16<'a, I>(mut it: I) -> Result<(Unit, I), UnitParseErr> where I: Iterator<Item = &'a u8> {
        let b = it.next().ok_or(UnitParseErr::NotInt)?;

        if *b != UnitBin::Size16 as u8 {
            return Err(UnitParseErr::NotInt);
        }

        let bytes = [
            *it.next().ok_or(UnitParseErr::NotInt)?,
            *it.next().ok_or(UnitParseErr::NotInt)?,
        ];

        let v = u16::from_le_bytes(bytes);
        Ok((Unit::Int(v as i32), it))
    }

    fn parse_bytes_int<'a, I>(mut it: I) -> Result<(Unit, I), UnitParseErr> where I: Iterator<Item = &'a u8> {
        let b = it.next().ok_or(UnitParseErr::NotInt)?;

        if *b != UnitBin::Int as u8 {
            return Err(UnitParseErr::NotInt);
        }

        let bytes = [
            *it.next().ok_or(UnitParseErr::NotInt)?,
            *it.next().ok_or(UnitParseErr::NotInt)?,
            *it.next().ok_or(UnitParseErr::NotInt)?,
            *it.next().ok_or(UnitParseErr::NotInt)?
        ];

        let v = i32::from_le_bytes(bytes);
        Ok((Unit::Int(v), it))
    }

    fn parse_bytes_dec<'a, I>(mut it: I) -> Result<(Unit, I), UnitParseErr> where I: Iterator<Item = &'a u8> {
        let b = it.next().ok_or(UnitParseErr::NotDec)?;

        if *b != UnitBin::Dec as u8 {
            return Err(UnitParseErr::NotDec);
        }

        let bytes = [
            *it.next().ok_or(UnitParseErr::NotDec)?,
            *it.next().ok_or(UnitParseErr::NotDec)?,
            *it.next().ok_or(UnitParseErr::NotDec)?,
            *it.next().ok_or(UnitParseErr::NotDec)?
        ];

        let v = f32::from_le_bytes(bytes);
        Ok((Unit::Dec(v), it))
    }

    fn parse_bytes_str<'a, I>(mut it: I) -> Result<(Unit, I), UnitParseErr> where I: Iterator<Item = &'a u8> {
        let b = it.next().ok_or(UnitParseErr::NotStr)?;

        if *b != UnitBin::Str as u8 {
            return Err(UnitParseErr::NotStr);
        }

        let bytes = [
            *it.next().ok_or(UnitParseErr::NotStr)?,
            *it.next().ok_or(UnitParseErr::NotStr)?,
            *it.next().ok_or(UnitParseErr::NotStr)?,
            *it.next().ok_or(UnitParseErr::NotStr)?
        ];

        let len = u32::from_le_bytes(bytes);
        let tmp = (0..len).into_iter().filter_map(|_| it.next()).cloned().collect::<Vec<u8>>();
        let s = String::from_utf8(tmp).map_err(|_| UnitParseErr::NotStr)?;

        Ok((Unit::Str(s), it))
    }

    fn parse_bytes_ref<'a, I>(mut it: I) -> Result<(Unit, I), UnitParseErr> where I: Iterator<Item = &'a u8> {
        let b = it.next().ok_or(UnitParseErr::NotRef)?;

        if *b != UnitBin::Ref as u8 {
            return Err(UnitParseErr::NotRef);
        }

        let bytes = [
            *it.next().ok_or(UnitParseErr::NotRef)?,
            *it.next().ok_or(UnitParseErr::NotRef)?,
            *it.next().ok_or(UnitParseErr::NotRef)?,
            *it.next().ok_or(UnitParseErr::NotRef)?
        ];

        let len = u32::from_le_bytes(bytes);
        let tmp = (0..len).into_iter().filter_map(|_| it.next()).cloned().collect::<Vec<u8>>();
        let s = String::from_utf8(tmp).map_err(|_| UnitParseErr::NotRef)?;

        let path = s.split(".").map(|s| s.to_string()).collect::<Vec<_>>();

        for p in &path {
            if !p.chars().all(|c| c.is_alphanumeric() || c == '#' || c == '_') {
                return Err(UnitParseErr::RefInvalidPath);
            }
        }

        Ok((Unit::Ref(path), it))
    }

    fn parse_bytes_pair<'a, I>(mut it: I) -> Result<(Unit, I), UnitParseErr> where I: Iterator<Item = &'a u8> + Clone {
        let b = it.next().ok_or(UnitParseErr::NotPair)?;

        if *b != UnitBin::Pair as u8 {
            return Err(UnitParseErr::NotPair);
        }

        let (u0, it) = Unit::parse_bytes(it)?;
        let (u1, it) = Unit::parse_bytes(it)?;

        Ok((Unit::Pair(Box::new(u0), Box::new(u1)), it))
    }

    fn parse_bytes_lst<'a, I>(mut it: I) -> Result<(Unit, I), UnitParseErr> where I: Iterator<Item = &'a u8> + Clone {
        let b = it.next().ok_or(UnitParseErr::NotList)?;

        if *b != UnitBin::Lst as u8 {
            return Err(UnitParseErr::NotList);
        }

        let bytes = [
            *it.next().ok_or(UnitParseErr::NotList)?,
            *it.next().ok_or(UnitParseErr::NotList)?,
            *it.next().ok_or(UnitParseErr::NotList)?,
            *it.next().ok_or(UnitParseErr::NotList)?
        ];

        let len = u32::from_le_bytes(bytes);
        let mut tmp = Vec::new();

        for _ in 0..len {
            let (u, next) = Unit::parse_bytes(it)?;
            tmp.push(u);
            it = next;
        }
        Ok((Unit::Lst(tmp), it))
    }

    fn parse_bytes_map<'a, I>(mut it: I) -> Result<(Unit, I), UnitParseErr> where I: Iterator<Item = &'a u8> + Clone {
        let b = it.next().ok_or(UnitParseErr::NotMap)?;

        if *b != UnitBin::Map as u8 {
            return Err(UnitParseErr::NotMap);
        }

        let bytes = [
            *it.next().ok_or(UnitParseErr::NotMap)?,
            *it.next().ok_or(UnitParseErr::NotMap)?,
            *it.next().ok_or(UnitParseErr::NotMap)?,
            *it.next().ok_or(UnitParseErr::NotMap)?
        ];

        let len = u32::from_le_bytes(bytes);
        let mut tmp = Vec::new();

        for _ in 0..len {
            let (u0, next) = Unit::parse_bytes(it)?;
            let (u1, next) = Unit::parse_bytes(next)?;
            tmp.push((u0, u1));
            it = next;
        }
        Ok((Unit::Map(tmp), it))
    }

    fn parse_bytes_stream<'a, I>(mut it: I) -> Result<(Unit, I), UnitParseErr> where I: Iterator<Item = &'a u8> + Clone {
        if *it.next().ok_or(UnitParseErr::NotStream)? != UnitBin::Stream as u8 {
            return Err(UnitParseErr::NotStream);
        }

        // msg
        let (msg, mut it) = Unit::parse_bytes(it)?;

        // serv
        let bytes = [
            *it.next().ok_or(UnitParseErr::NotStream)?,
            *it.next().ok_or(UnitParseErr::NotStream)?,
            *it.next().ok_or(UnitParseErr::NotStream)?,
            *it.next().ok_or(UnitParseErr::NotStream)?
        ];

        let len = u32::from_le_bytes(bytes);
        let tmp = (0..len).into_iter().filter_map(|_| it.next()).cloned().collect::<Vec<u8>>();
        let serv = String::from_utf8(tmp).map_err(|_| UnitParseErr::NotStream)?;

        // addr
        let addr = match *it.next().ok_or(UnitParseErr::NotStream)? {
            _b if _b == UnitBin::AddrLoc as u8 => Addr::Local,
            _b if _b == UnitBin::AddrRemote as u8 => {
                let addr = (0..8).map(|_| {
                    let bytes = [
                        *it.next()?,
                        *it.next()?,
                    ];

                    Some(u16::from_le_bytes(bytes))
                }).collect::<Option<Vec<_>>>().ok_or(UnitParseErr::NotStream)?.try_into().map_err(|_| UnitParseErr::NotStream)?;

                Addr::Remote(addr)
            },
            _ => return Err(UnitParseErr::NotStream)
        };

        Ok((Unit::Stream(Box::new(msg), (serv, addr)), it))
    }

    pub fn parse_bytes<'a, I>(it: I) -> Result<(Unit, I), UnitParseErr> where I: Iterator<Item = &'a u8> + Clone {
        match *it.clone().next().ok_or(UnitParseErr::NotUnit)? {
            _b if _b == UnitBin::None as u8 => Unit::parse_bytes_none(it),
            _b if _b == UnitBin::Bool as u8 => Unit::parse_bytes_bool(it),
            _b if _b == UnitBin::Byte as u8 => Unit::parse_bytes_byte(it),
            _b if _b == UnitBin::Int as u8 => Unit::parse_bytes_int(it),
            _b if _b == UnitBin::Zero as u8 => Unit::parse_bytes_int_zero(it),
            _b if _b == UnitBin::Size8 as u8 => Unit::parse_bytes_int_u8(it),
            _b if _b == UnitBin::Size16 as u8 => Unit::parse_bytes_int_u16(it),
            _b if _b == UnitBin::Int8 as u8 => Unit::parse_bytes_int_i8(it),
            _b if _b == UnitBin::Int16 as u8 => Unit::parse_bytes_int_i16(it),
            _b if _b == UnitBin::Dec as u8 => Unit::parse_bytes_dec(it),
            _b if _b == UnitBin::Str as u8 => Unit::parse_bytes_str(it),
            _b if _b == UnitBin::Ref as u8 => Unit::parse_bytes_ref(it),
            _b if _b == UnitBin::Pair as u8 => Unit::parse_bytes_pair(it),
            _b if _b == UnitBin::Lst as u8 => Unit::parse_bytes_lst(it),
            _b if _b == UnitBin::Map as u8 => Unit::parse_bytes_map(it),
            _b if _b == UnitBin::Stream as u8 => Unit::parse_bytes_stream(it),
            _ => Err(UnitParseErr::NotUnit)
        }
    }

    fn parse_ch<'a>(ch: char, mut it: Chars<'a>) -> Result<Chars<'a>, UnitParseErr> {
        if ch == it.next().ok_or(UnitParseErr::UnexpectedChar)? {
            return Ok(it)
        }

        Err(UnitParseErr::UnexpectedChar)
    }

    fn parse_ws<'a>(mut it: Chars<'a>) -> Result<Chars<'a>, UnitParseErr> {
        if it.next().ok_or(UnitParseErr::MissedWhitespace)?.is_ascii_whitespace() {
            let mut tmp = it.clone();

            while let Some(c) = it.next() {
                if !c.is_ascii_whitespace() {
                    break;
                }
                tmp = it.clone();
            }
            return Ok(tmp)
        }

        Err(UnitParseErr::MissedWhitespace)
    }

    fn parse_none<'a>(it: Chars<'a>) -> Result<(Self, Chars<'a>), UnitParseErr> {
        let it = Unit::parse_ch('-', it)?;
        Ok((Unit::None, it))
    }

    fn parse_bool<'a>(it: Chars<'a>) -> Result<(Self, Chars<'a>), UnitParseErr> {
        let (val, it) = if let Ok(it) = Unit::parse_ch('t', it.clone()) {
            (true, it)
        } else if let Ok(it) = Unit::parse_ch('f', it) {
            (false, it)
        } else {
            return Err(UnitParseErr::NotBool);
        };

        if let Some(ch) = it.clone().next() {
            if ch.is_alphanumeric() {
                return Err(UnitParseErr::NotBool);
            }
        }

        Ok((Unit::Bool(val), it))
    }

    fn parse_byte<'a>(mut it: Chars<'a>) -> Result<(Self, Chars<'a>), UnitParseErr> {
        if let Some(s) = it.as_str().get(0..4) {
            it.next().unwrap();
            it.next().unwrap();
            it.next().unwrap();
            it.next().unwrap();

            if let Ok(v) = u8::from_str_radix(s.trim_start_matches("0x"), 16) {
                return Ok((Unit::Byte(v), it))
            }
        }

        Err(UnitParseErr::NotByte)
    }

    fn parse_int<'a>(mut it: Chars<'a>) -> Result<(Self, Chars<'a>), UnitParseErr> {
        let mut s = String::new();
        let mut tmp = it.clone();

        while let Some(c) = it.next() {
            if !(c.is_numeric() || c == '-') {
                break;
            }

            s.push(c);
            tmp = it.clone();
        }

        if let Ok(v) = s.parse::<i32>() {
            return Ok((Unit::Int(v), tmp));
        }

        Err(UnitParseErr::NotInt)
    }

    fn parse_dec<'a>(it: Chars<'a>) -> Result<(Self, Chars<'a>), UnitParseErr> {
        let (fst, it) = Unit::parse_int(it)?;
        let it = Unit::parse_ch('.', it)?;
        let (scd, it) = Unit::parse_int(it)?;

        let s = format!("{}.{}", fst, scd);
        let out = s.parse::<f32>().map_err(|_| UnitParseErr::NotDec)?;

        return Ok((Unit::Dec(out), it));
    }

    fn parse_str<'a>(mut it: Chars<'a>) -> Result<(Self, Chars<'a>), UnitParseErr> {
        if let Some(c) = it.next() {
            // `complex string`
            if c == '`' {
                let mut s = String::new();
                let mut tmp = it.clone();

                while let Some(c) = it.next() {
                    if c == '`' {
                        break;
                    }

                    s.push(c);
                    tmp = it.clone();
                }

                if let Some(c) = tmp.next() {
                    if c == '`' {
                        return Ok((Unit::Str(s), tmp));
                    } else {
                        return Err(UnitParseErr::NotClosedQuotes);
                    }
                } else {
                    return Err(UnitParseErr::NotClosedQuotes);
                }
            }

            // 'complex string'
            if c == '\'' {
                let mut s = String::new();
                let mut tmp = it.clone();

                while let Some(c) = it.next() {
                    if c == '\'' {
                        break;
                    }

                    s.push(c);
                    tmp = it.clone();
                }

                if let Some(c) = tmp.next() {
                    if c == '\'' {
                        return Ok((Unit::Str(s), tmp));
                    } else {
                        return Err(UnitParseErr::NotClosedQuotes);
                    }
                } else {
                    return Err(UnitParseErr::NotClosedQuotes);
                }
            }

            // "complex string"
            if c == '"' {
                let mut s = String::new();
                let mut tmp = it.clone();

                while let Some(c) = it.next() {
                    if c == '"' {
                        break;
                    }

                    s.push(c);
                    tmp = it.clone();
                }

                if let Some(c) = tmp.next() {
                    if c == '"' {
                        return Ok((Unit::Str(s), tmp));
                    } else {
                        return Err(UnitParseErr::NotClosedQuotes);
                    }
                } else {
                    return Err(UnitParseErr::NotClosedQuotes);
                }
            }

            // abc.123#
            if c.is_alphanumeric() || c == '.' || c == '#' || c == '_' {
                let mut s = String::new();
                let mut tmp = it.clone();

                s.push(c);

                while let Some(c) = it.next() {
                    if !(c.is_alphanumeric() || c == '.' || c == '#' || c == '_') {
                        break;
                    }

                    s.push(c);
                    tmp = it.clone();
                }

                return Ok((Unit::Str(s), tmp));
            }
        }
        Err(UnitParseErr::NotStr)
    }

    fn parse_ref<'a>(it: Chars<'a>) -> Result<(Self, Chars<'a>), UnitParseErr> {
        let it = Unit::parse_ch('@', it)?;

        let (path, it) = Unit::parse_str(it)?;

        let path = path.as_str().ok_or(UnitParseErr::RefNotString)?;
        let path = path.split(".").map(|s| s.to_string()).collect::<Vec<_>>();

        for p in &path {
            if !p.chars().all(|c| c.is_alphanumeric() || c == '#' || c == '_') {
                return Err(UnitParseErr::RefInvalidPath);
            }
        }

        return Ok((Unit::Ref(path), it));
    }

    fn parse_pair<'a>(it: Chars<'a>) -> Result<(Self, Chars<'a>), UnitParseErr> {
        let it = Unit::parse_ch('(', it)?;

        let (u0, it) = Unit::parse(it)?;

        let it = Unit::parse_ws(it)?;

        let (u1, it) = Unit::parse(it)?;

        let it = Unit::parse_ch(')', it)?;

        return Ok((
            Unit::Pair(
                Box::new(u0),
                Box::new(u1)
            ),
            it
        ));
    }

    fn parse_list<'a>(it: Chars<'a>) -> Result<(Self, Chars<'a>), UnitParseErr> {
        let mut it = Unit::parse_ch('[', it)?;

        let mut lst = Vec::new();

        loop {
            if let Ok(tmp) = Unit::parse_ws(it.clone()) {
                it = tmp;
            }

            let (u, tmp) = Unit::parse(it)?;
            lst.push(u);

            it = tmp;

            if let Ok(tmp) = Unit::parse_ws(it.clone()) {
                it = tmp;
            }

            if let Ok(it) = Unit::parse_ch(']', it.clone()) {
                return Ok((Unit::Lst(lst), it))
            }
        }
    }

    fn parse_map<'a>(it: Chars<'a>) -> Result<(Self, Chars<'a>), UnitParseErr> {
        let mut it = Unit::parse_ch('{', it)?;

        let mut map = Vec::new();

        loop {
            if let Ok(tmp) = Unit::parse_ws(it.clone()) {
                it = tmp;
            }

            let (u0, tmp) = Unit::parse(it)?;
            it = tmp;

            if let Ok(tmp) = Unit::parse_ws(it.clone()) {
                it = tmp;
            }

            it = Unit::parse_ch(':', it)?;

            if let Ok(tmp) = Unit::parse_ws(it.clone()) {
                it = tmp;
            }

            let (u1, tmp) = Unit::parse(it)?;
            it = tmp;

            map.push((u0, u1));

            if let Ok(tmp) = Unit::parse_ws(it.clone()) {
                it = tmp;
            }

            if let Ok(it) = Unit::parse_ch('}', it.clone()) {
                return Ok((Unit::Map(map), it))
            }
        }
    }

    fn parse_stream<'a>(it: Chars<'a>) -> Result<(Self, Chars<'a>), UnitParseErr> {
        let (u, it) = Unit::parse_loc(it)?;
        let mut tmp = it.clone();

        if let Some(ch) = tmp.next() {
            if ch == '@' {
                let (serv, tmp) = Unit::parse_str(tmp)?;
                let serv = serv.as_str().ok_or(UnitParseErr::StreamInvalidServ)?;

                return Ok((Unit::Stream(Box::new(u), (serv, Addr::Local)), tmp))
            }
        }

        Ok((u, it))
    }

    fn parse_loc<'a>(it: Chars<'a>) -> Result<(Self, Chars<'a>), UnitParseErr> {
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

        // none
        if let Ok((u, it)) = Unit::parse_none(it.clone()) {
            return Ok((u, it));
        }

        // str
        if let Ok((u, it)) = Unit::parse_str(it.clone()) {
            return Ok((u, it));
        }

        // pair
        if let Ok((u, it)) = Unit::parse_pair(it.clone()) {
            return Ok((u, it));
        }

        // ref
        if let Ok((u, it)) = Unit::parse_ref(it.clone()) {
            return Ok((u, it));
        }

        // list
        if let Ok((u, it)) = Unit::parse_list(it.clone()) {
            return Ok((u, it));
        }

        // map
        if let Ok((u, it)) = Unit::parse_map(it.clone()) {
            return Ok((u, it));
        }

        Err(UnitParseErr::NotUnit)
    }

    pub fn parse<'a>(it: Chars<'a>) -> Result<(Self, Chars<'a>), UnitParseErr> {
        // stream
        Unit::parse_stream(it)
    }

    pub fn find_ref<I>(mut path: I, u: &Unit) -> Option<Unit> where I: Iterator<Item = String> {
        match u {
            Unit::Map(m) => {
                if let Some(path_s) = path.next() {
                    if let Some((_, next)) = m.iter().filter_map(|(u0, u1)| Some((u0.as_str()?, u1))).find(|(s, _)| *s == path_s) {
                        return Unit::find_ref(path, next);
                    } else if path_s == "all" {
                        return Some(u.clone());
                    }
                } else {
                    return Some(u.clone());
                }
            },
            Unit::Lst(lst) => {
                if let Some(path_s) = path.next() {
                    if let Some(idx) = path_s.parse::<usize>().ok() {
                        if let Some(next) = lst.get(idx) {
                            return Unit::find_ref(path, next);
                        }
                    }
                } else {
                    return Some(u.clone())
                }
            },
            Unit::Pair(u0, u1) => {
                if let Some(path_s) = path.next() {
                    if let Some(idx) = path_s.parse::<usize>().ok() {
                        match idx {
                            0 => return Unit::find_ref(path, u0),
                            1 => return Unit::find_ref(path, u1),
                            _ => ()
                        }
                    }
                } else {
                    return Some(u.clone())
                }
            }
            _ => return Some(u.clone())
        }
        None
    }

    pub fn merge_ref<I>(mut path: I, val: Unit, u: Unit) -> Option<Unit> where I: Iterator<Item = String> {
        if let Some(path_s) = path.next() {
            match u {
                Unit::Map(m) => {
                    if let Some((_, next)) = m.iter().filter_map(|(u0, u1)| Some((u0.as_str()?, u1))).find(|(s, _)| *s == path_s) {
                        if let Some(val) = Unit::merge_ref(path, val, next.clone()) {
                            let val = next.clone().merge(val);
                            let u = Unit::Map(vec![(Unit::Str(path_s), val)]);

                            return Some(Unit::Map(m).merge(u))
                        }
                    } else {
                        if let Some(val) = Unit::merge_ref(path, val, Unit::Map(Vec::new())) {
                            let u = Unit::Map(vec![(Unit::Str(path_s), val)]);
                            return Some(Unit::Map(m).merge(u))
                        }
                    }
                },
                Unit::Lst(mut lst) => {
                    if let Some(idx) = path_s.parse::<usize>().ok() {
                        if let Some(u) = lst.get_mut(idx) {
                            let val = Unit::merge_ref(path, val, u.clone());
                            if let Some(val) = val {
                               *u = val;
                            }
                        }
                    }
                    return Some(Unit::Lst(lst));
                },
                Unit::Pair(u0, u1) => {
                    if let Some(idx) = path_s.parse::<usize>().ok() {
                        match idx {
                            0 => {
                                let val = Unit::merge_ref(path, val, *u0);
                                if let Some(val) = val {
                                    return Some(Unit::Pair(Box::new(val), u1))
                                }
                            },
                            1 => {
                                let val = Unit::merge_ref(path, val, *u1);
                                if let Some(val) = val {
                                    return Some(Unit::Pair(u0, Box::new(val)))
                                }
                            },
                            _ => return Some(Unit::Pair(u0, u1))
                        };
                    }
                },
                _ => {
                    if let Some(val) = Unit::merge_ref(path, val, Unit::Map(Vec::new())) {
                        let u = Unit::Map(vec![(Unit::Str(path_s), val)]);
                        return Some(u)
                    }
                }
            }
        } else {
            return Some(val);
        }
        None
    }

    pub fn as_none(&self) -> Option<()> {
        if let Unit::None = self {
            return Some(())
        }
        None
    }

    pub fn as_bool(&self) -> Option<bool> {
        if let Unit::Bool(v) = self {
            return Some(*v)
        }
        None
    }

    pub fn as_byte(&self) -> Option<u8> {
        if let Unit::Byte(v) = self {
            return Some(*v)
        }
        None
    }

    pub fn as_int(&self) -> Option<i32> {
        if let Unit::Int(v) = self {
            return Some(*v)
        }
        None
    }

    pub fn as_dec(&self) -> Option<f32> {
        if let Unit::Dec(v) = self {
            return Some(*v)
        }
        None
    }

    pub fn as_str(&self) -> Option<String> {
        if let Unit::Str(s) = self {
            return Some(s.clone())
        }
        None
    }

    pub fn as_ref(&self) -> Option<Vec<String>> {
        if let Unit::Ref(path) = self {
            return Some(path.clone());
        }
        None
    }

    pub fn as_pair(&self) -> Option<(Box<Unit>, Box<Unit>)> {
        if let Unit::Pair(u0, u1) = self {
            return Some((u0.clone(), u1.clone()))
        }
        None
    }

    pub fn as_vec(&self) -> Option<Vec<Unit>> {
        if let Unit::Lst(lst) = self {
            return Some(lst.clone());
        }
        None
    }

    pub fn as_vec_typed<A, B>(&self, f: B) -> Option<Vec<A>> where A: Clone, B: Fn(&Self) -> Option<A> {
        if let Unit::Lst(lst) = self {
            return Some(lst.iter().filter_map(|u| f(u)).collect());
        }
        None
    }

    pub fn as_map(&self) -> Option<Vec<(Unit, Unit)>> {
        if let Unit::Map(m) = self {
            return Some(m.clone());
        }
        None
    }

    pub fn as_map_find(&self, sch: &str) -> Option<Unit> {
        if let Unit::Map(m) = self {
            return m.iter()
                .filter_map(|(u0, u1)| Some((u0.as_str()?, u1)))
                .map(|(s, u)| {
                    if s == sch {
                        return Some(u.clone());
                    }
                    None
                }).next()?;
        }
        None
    }

    pub fn merge(self, u: Unit) -> Unit {
        match u.clone() {
            Unit::Map(m) => {
                if let Some(tmp) = self.as_map() {
                    let it = m.into_iter().map(|(u0, u1)| {
                        if let Some((_, next)) = tmp.iter().find(|(n, _)| n.clone() == u0) {
                            let u1 = next.clone().merge(u1);
                            return (u0, u1);
                        }
                        (u0, u1)
                    });

                    let res = tmp.iter().cloned().filter(|(n, _)| it.clone().find(|(prev, _)| n.clone() == prev.clone()).is_none()).chain(it.clone()).collect();
                    return Unit::Map(res);
                }
            },
            Unit::Pair(u0, u1) => {
                if self.as_pair().is_some() {
                    return Unit::Pair(u0, u1);
                }

                if let Some(mut tmp) = self.as_map() {
                    tmp.retain(|(u, _)| u.clone() == *u0);
                    tmp.push((*u0, *u1));
                    return Unit::Map(tmp);
                }

                if let Some(mut tmp) = self.as_vec() {
                    tmp.retain(|u| u.clone() == Unit::Pair(u0.clone(), u1.clone()));
                    tmp.push(Unit::Pair(u0.clone(), u1.clone()));
                    return Unit::Lst(tmp);
                }
            }
            Unit::Lst(lst) => {
                // if let Some(mut tmp) = self.as_vec() {
                //     tmp.retain(|u| {
                //         lst.iter().find(|n| *u == **n).is_none()
                //     });

                //     tmp.extend(lst);
                //     return Unit::Lst(tmp);
                // }
                if self.as_vec().is_some() {
                    return Unit::Lst(lst);
                }
            },
            _ => return u
        }
        u
    }
}

impl Schema for SchemaNone {
    type Out = ();

    fn find(&self, _glob: &Unit, u: &Unit) -> Option<Self::Out> {
        if let Unit::None = u {
            return Some(());
        }
        None
    }
}

impl Schema for SchemaBool {
    type Out = bool;

    fn find(&self, _glob: &Unit, u: &Unit) -> Option<Self::Out> {
        if let Unit::Bool(b) = u {
            return Some(*b);
        }
        None
    }
}

impl Schema for SchemaByte {
    type Out = u8;

    fn find(&self, _glob: &Unit, u: &Unit) -> Option<Self::Out> {
        if let Unit::Byte(b) = u {
            return Some(*b);
        }
        None
    }
}

impl Schema for SchemaInt {
    type Out = i32;

    fn find(&self, _glob: &Unit, u: &Unit) -> Option<Self::Out> {
        if let Unit::Int(v) = u {
            return Some(*v);
        }
        None
    }
}

impl Schema for SchemaDec {
    type Out = f32;

    fn find(&self, _glob: &Unit, u: &Unit) -> Option<Self::Out> {
        if let Unit::Dec(v) = u {
            return Some(*v);
        }
        None
    }
}

impl Schema for SchemaStr {
    type Out = String;

    fn find(&self, _glob: &Unit, u: &Unit) -> Option<Self::Out> {
        if let Unit::Str(s) = u {
            return Some(s.clone());
        }
        None
    }
}

impl Schema for SchemaUnit {
    type Out = Unit;

    fn find(&self, _glob: &Unit, u: &Unit) -> Option<Self::Out> {
        Some(u.clone())
    }
}

impl Schema for SchemaRef {
    type Out = Vec<String>;

    fn find_deep(&self, glob: &Unit, u: &Unit) -> Option<Self::Out> {
        self.find(glob, u)
    }

    fn find(&self, _glob: &Unit, u: &Unit) -> Option<Self::Out> {
        if let Unit::Ref(path) = u {
            return Some(path.clone())
        }
        None
    }
}

impl<A, B> Schema for SchemaPair<A, B> where A: Schema, B: Schema {
    type Out = (A::Out, B::Out);

    fn find(&self, glob: &Unit, u: &Unit) -> Option<Self::Out> {
        if let Unit::Pair(u0, u1) = u {
            return Some((self.0.find_deep(glob, u0)?, self.1.find_deep(glob, u1)?));
        }
        None
    }
}

impl<A> Schema for SchemaSeq<A> where A: Schema {
    type Out = Vec<A::Out>;

    fn find(&self, glob: &Unit, u: &Unit) -> Option<Self::Out> {
        if let Unit::Lst(lst) = u {
            return Some(lst.iter().filter_map(|u| self.0.find_deep(glob, u)).collect());
        }
        None
    }
}

impl<A, B> Schema for SchemaMapSeq<A, B> where A: Schema, B: Schema {
    type Out = Vec<(A::Out, B::Out)>;

    fn find(&self, glob: &Unit, u: &Unit) -> Option<Self::Out> {
        if let Unit::Map(m) = u {
            return Some(m.iter().filter_map(|(u0, u1)| Some((self.0.find_deep(glob, u0)?, self.1.find_deep(glob, u1)?))).collect());
        }
        None
    }
}

impl<A> Schema for SchemaMapEntry<A> where A: Schema {
    type Out = A::Out;

    fn find(&self, glob: &Unit, u: &Unit) -> Option<Self::Out> {
        if let Unit::Map(m) = u {
            if let Some(u) = m.iter().find(|(u, _)| self.0 == u.clone()).map(|(_, u)| u) {
                return self.1.find_deep(glob, u);
            }
        }
        None
    }
}

impl<A, B> Schema for SchemaMap<A, B> where A: Schema, B: Schema {
    type Out = (Option<A::Out>, Option<B::Out>);

    fn find(&self, glob: &Unit, u: &Unit) -> Option<Self::Out> {
        if u.as_map().is_some() {
            return Some((self.0.find_deep(glob, u), self.1.find_deep(glob, u)));
        }

        None
    }
}

impl<A, B> Schema for SchemaMapFirstRequire<A, B> where A: Schema, B: Schema {
    type Out = (A::Out, Option<B::Out>);

    fn find(&self, glob: &Unit, u: &Unit) -> Option<Self::Out> {
        if u.as_map().is_some() {
            return Some((self.0.find_deep(glob, u)?, self.1.find_deep(glob, u)));
        }

        None
    }
}

impl<A, B> Schema for SchemaMapSecondRequire<A, B> where A: Schema, B: Schema {
    type Out = (Option<A::Out>, B::Out);

    fn find(&self, glob: &Unit, u: &Unit) -> Option<Self::Out> {
        if u.as_map().is_some() {
            return Some((self.0.find_deep(glob, u), self.1.find_deep(glob, u)?));
        }

        None
    }
}

impl<A, B> Schema for SchemaMapRequire<A, B> where A: Schema, B: Schema {
    type Out = (A::Out, B::Out);

    fn find(&self, glob: &Unit, u: &Unit) -> Option<Self::Out> {
        if u.as_map().is_some() {
            return Some((self.0.find_deep(glob, u)?, self.1.find_deep(glob, u)?));
        }

        None
    }
}

impl<A, B> Schema for SchemaOr<A, B> where A: Schema, B: Schema {
    type Out = Or<A::Out, B::Out>;

    fn find(&self, glob: &Unit, u: &Unit) -> Option<Self::Out> {
        if let Some(v) = self.0.find_deep(glob, u) {
            return Some(Or::First(v));
        }
        Some(Or::Second(self.1.find_deep(glob, u)?))
    }
}
