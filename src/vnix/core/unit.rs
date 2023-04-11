use alloc::rc::Rc;
use alloc::vec::Vec;
use alloc::boxed::Box;
use alloc::{format, vec};
use alloc::string::String;
use num::Zero;

use core::pin::Pin;
use core::slice::Iter;
use core::fmt::Display;
use core::ops::{Generator, GeneratorState};
use core::cmp::PartialOrd;

use spin::Mutex;

use num::cast::ToPrimitive;
use num::bigint::{BigInt, Sign};
use num::rational::BigRational;

use crate::vnix::utils::Maybe;
use crate::vnix::core::task::TaskRun;

use crate::vnix::core::driver::MemSizeUnits;
use crate::{thread, thread_await, task_result, maybe, maybe_ok};

use super::task::ThreadAsync;
use super::kern::{Addr, KernErr, Kern};


#[derive(Debug, PartialEq, PartialOrd, Clone)]
pub struct Int(pub Rc<BigInt>);

#[derive(Debug, PartialEq, PartialOrd, Clone)]
pub struct Dec(pub Rc<BigRational>);

pub type Path = Vec<String>;

#[derive(Debug, PartialEq, Clone)]
pub enum UnitType {
    None,
    Bool(bool),
    Byte(u8),
    Int(Int),
    Dec(Dec),
    Str(Rc<String>),
    Ref(Rc<Path>),
    Stream(Unit, Rc<String>, Rc<Addr>),
    Pair(Unit, Unit),
    List(Rc<Vec<Unit>>),
    Map(Rc<Vec<(Unit, Unit)>>)
}

#[derive(Debug, PartialEq, Clone)]
pub struct Unit(Rc<UnitType>);

#[derive(Debug, Clone)]
pub enum UnitBin {
    None = 0,
    Bool,
    Byte,
    Int,
    IntNat,
    IntBig,
    Dec,
    DecBig,
    Str,
    Ref,
    Stream,
    AddrLoc,
    AddrRemote,
    Pair,
    List,
    Map,

    // Optimization
    Zero,
    Int8,
    Int16,
    Int24,
    Uint8,
    Uint16,
    Uint24,
    Str8,
    Str16,
    Str24,
    List8,
    List16,
    List24,
    Map8,
    Map16,
    Map24,
    PairUint8Uint24,
    PairUint16Uint24,
    PairUint24Uint24,
    PairUint16Int32
}

#[derive(Debug, Clone, PartialEq)]
pub enum UnitParseErr {
    NotUnit,
    UnexpectedEnd,
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
    UnexpectedChar,
    RefInvalidPath,
    DevideByZero,
    InvalidSign,
    InvalidAddr,
    StreamInvalidServ,
}

pub trait UnitNew {
    fn none() -> Unit;
    fn bool(v: bool) -> Unit;
    fn byte(v: u8) -> Unit;
    fn int(v: i32) -> Unit;
    fn uint(v: u32) -> Unit;
    fn int_big(v: BigInt) -> Unit;
    fn int_share(v: Rc<BigInt>) -> Unit;
    fn dec(v: f32) -> Unit;
    fn dec_big(v: BigRational) -> Unit;
    fn dec_share(v: Rc<BigRational>) -> Unit;
    fn str(s: &str) -> Unit;
    fn str_share(s: Rc<String>) -> Unit;
    fn path(path: &[&str]) -> Unit;
    fn path_share(path: Rc<Vec<String>>) -> Unit;
    fn stream_loc(u: Unit, serv: &str) -> Unit;
    fn stream(u: Unit, serv: &str, addr: Addr) -> Unit;
    fn pair(u0: Unit, u1: Unit) -> Unit;
    fn list(lst: &[Unit]) -> Unit;
    fn list_share(lst: Rc<Vec<Unit>>) -> Unit;
    fn map(map: &[(Unit, Unit)]) -> Unit;
}

pub trait UnitAs {
    fn as_none(self) -> Option<()>;
    fn as_bool(self) -> Option<bool>;
    fn as_byte(self) -> Option<u8>;
    fn as_int(self) -> Option<i32>;
    fn as_uint(self) -> Option<u32>;
    fn as_int_big(self) -> Option<Rc<BigInt>>;
    fn as_dec(self) -> Option<f32>;
    fn as_dec_big(self) -> Option<Rc<BigRational>>;
    fn as_str(self) -> Option<Rc<String>>;
    fn as_path(self) -> Option<Rc<Path>>;
    fn as_stream(self) -> Option<(Unit, String, Addr)>;
    fn as_pair(self) -> Option<(Unit, Unit)>;
    fn as_list(self) -> Option<Rc<Vec<Unit>>>;
    fn as_map(self) -> Option<Rc<Vec<(Unit, Unit)>>>;
    fn as_map_find(self, sch: &str) -> Option<Unit>;
}

pub trait UnitAsBytes {
    fn as_bytes(self) -> Vec<u8>;
}

pub struct UnitParseBytesIter {
    dat: Vec<u8>,
    pos: usize
}

pub trait UnitParse<'a, T: 'a, I> {
    fn parse(it: I) -> Result<(Unit, I), UnitParseErr>;
    fn parse_none(it: I) -> Result<(Unit, I), UnitParseErr>;
    fn parse_bool(it: I) -> Result<(Unit, I), UnitParseErr>;
    fn parse_byte(it: I) -> Result<(Unit, I), UnitParseErr>;
    fn parse_int(it: I) -> Result<(Unit, I), UnitParseErr>;
    fn parse_dec(it: I) -> Result<(Unit, I), UnitParseErr>;
    fn parse_str(it: I) -> Result<(Unit, I), UnitParseErr>;
    fn parse_ref(it: I) -> Result<(Unit, I), UnitParseErr>;
    fn parse_stream(it: I) -> Result<(Unit, I), UnitParseErr>;
    fn parse_pair(it: I) -> Result<(Unit, I), UnitParseErr>;
    fn parse_list(it: I) -> Result<(Unit, I), UnitParseErr>;
    fn parse_map(it: I) -> Result<(Unit, I), UnitParseErr>;

    fn parse_list_partial(_it: I) -> Result<(usize, I), UnitParseErr> {
        unimplemented!()
    }

    fn parse_ch(_expect: char, _it: I) -> Result<I, UnitParseErr> {
        unimplemented!()
    }

    fn parse_ws(_it: I) -> Result<(usize, I), UnitParseErr> {
        unimplemented!()
    }
}

#[derive(Debug, Clone)]
pub struct DisplayStr(pub Unit);

#[derive(Debug, Clone)]
pub struct DisplayNice(pub usize, pub usize, pub Unit);

#[derive(Debug, Clone)]
pub struct DisplayShort(pub usize, pub Unit);

pub type UnitTypeReadAsync<'a, T> = ThreadAsync<'a, Maybe<(T, Rc<String>), KernErr>>;
pub type UnitReadAsync<'a> = UnitTypeReadAsync<'a, Unit>;

pub trait UnitReadAsyncI {
    fn read_async<'a>(self, ath: Rc<String>, orig: Unit, kern: &'a Mutex<Kern>) -> UnitReadAsync<'a>;
    fn as_map_find_async<'a>(self, sch: String, ath: Rc<String>, orig: Unit, kern: &'a Mutex<Kern>) -> UnitReadAsync<'a>;
}

#[macro_export]
macro_rules! read_async {
    ($msg:expr, $ath:expr, $orig:expr, $kern:expr) => {
        thread_await!($msg.clone().read_async($ath.clone(), $orig.clone(), $kern))
    };
}

#[macro_export]
macro_rules! as_async {
    ($msg:expr, $as:ident, $ath:expr, $orig:expr, $kern:expr) => {
        {
            match crate::read_async!($msg, $ath, $orig, $kern) {
                Ok(res) => {
                    if let Some((msg, ath)) = res {
                        if let Some(u) = msg.$as() {
                            Ok(Some((u, ath)))
                        } else {
                            Ok(None)
                        }
                    } else {
                        Ok(None)
                    }
                },
                Err(e) => Err(e)
            }
        }
    };
}

#[macro_export]
macro_rules! as_map_find_async {
    ($msg:expr, $sch:expr, $ath:expr, $orig:expr, $kern:expr) => {
        thread_await!($msg.clone().as_map_find_async($sch.into(), $ath.clone(), $orig.clone(), $kern))
    };
}

#[macro_export]
macro_rules! as_map_find_as_async {
    ($msg:expr, $sch:expr, $as:ident, $ath:expr, $orig:expr, $kern:expr) => {
        match crate::as_map_find_async!($msg, $sch, $ath, $orig, $kern) {
            Ok(res) => Ok(res.and_then(|(u, ath)| Some((u.$as()?, ath)))),
            Err(e) => Err(e)
        }
    };
}

pub trait UnitModify {
    fn find<'a, I>(&self, path: I) -> Option<Unit> where I: Iterator<Item = &'a str> + Clone;
    fn replace<'a, I>(self, path: I, what: Unit) -> Option<Unit> where I: Iterator<Item = &'a str> + Clone;

    fn merge_with(self, what: Unit) -> Unit;
    fn merge<'a, I>(self, path: I, what: Unit) -> Option<Unit> where I: Iterator<Item = &'a str> + Clone;
}

impl UnitNew for Unit {
    fn none() -> Unit {
        Unit::new(UnitType::None)
    }

    fn bool(v: bool) -> Unit {
        Unit::new(UnitType::Bool(v))
    }

    fn byte(v: u8) -> Unit {
        Unit::new(UnitType::Byte(v))
    }

    fn int(v: i32) -> Unit {
        Unit::int_big(BigInt::from(v))
    }

    fn uint(v: u32) -> Unit {
        Unit::int_big(BigInt::from(v))
    }

    fn int_big(v: BigInt) -> Unit {
        Unit::int_share(Rc::new(v))
    }

    fn int_share(v: Rc<BigInt>) -> Unit {
        Unit::new(UnitType::Int(Int(v)))
    }

    fn dec(v: f32) -> Unit {
        let v = BigRational::from_float(v).unwrap_or_default();
        Unit::dec_big(v)
    }

    fn dec_big(v: BigRational) -> Unit {
        Unit::dec_share(Rc::new(v))
    }

    fn dec_share(v: Rc<BigRational>) -> Unit {
        Unit::new(UnitType::Dec(Dec(v)))
    }

    fn str(s: &str) -> Unit {
        Unit::new(UnitType::Str(Rc::new(s.into())))
    }

    fn str_share(s: Rc<String>) -> Unit {
        Unit::new(UnitType::Str(s))
    }

    fn path(path: &[&str]) -> Unit {
        Unit::new(UnitType::Ref(Rc::new(path.into_iter().cloned().map(|s| format!("{s}")).collect())))
    }

    fn path_share(path: Rc<Vec<String>>) -> Unit {
        Unit::new(UnitType::Ref(path))
    }

    fn stream_loc(u: Unit, serv: &str) -> Unit {
        Unit::new(UnitType::Stream(u, Rc::new(serv.into()), Rc::new(Addr::Local)))
    }

    fn stream(u: Unit, serv: &str, addr: Addr) -> Unit {
        Unit::new(UnitType::Stream(u, Rc::new(serv.into()), Rc::new(addr)))
    }

    fn pair(u0: Unit, u1: Unit) -> Unit {
        Unit::new(UnitType::Pair(u0, u1))
    }

    fn list(lst: &[Unit]) -> Unit {
        Unit::new(UnitType::List(Rc::new(lst.to_vec())))
    }

    fn list_share(lst: Rc<Vec<Unit>>) -> Unit {
        Unit::new(UnitType::List(lst))
    }

    fn map(map: &[(Unit, Unit)]) -> Unit {
        Unit::new(UnitType::Map(Rc::new(map.to_vec())))
    }
}

impl UnitAs for Unit {
    fn as_none(self) -> Option<()> {
        if let UnitType::None = self.0.as_ref() {
            return Some(())
        }
        None
    }

    fn as_bool(self) -> Option<bool> {
        if let UnitType::Bool(v) = self.0.as_ref() {
            return Some(*v)
        }
        None
    }

    fn as_byte(self) -> Option<u8> {
        if let UnitType::Byte(v) = self.0.as_ref() {
            return Some(*v)
        }
        None
    }

    fn as_int(self) -> Option<i32> {
        if let UnitType::Int(v) = self.0.as_ref() {
            if let Some(v) = v.to_small() {
                return Some(v)
            }
        }
        None
    }

    fn as_uint(self) -> Option<u32> {
        if let UnitType::Int(v) = self.0.as_ref() {
            if let Some(v) = v.to_nat() {
                return Some(v)
            }
        }
        None
    }

    fn as_int_big(self) -> Option<Rc<BigInt>> {
        if let UnitType::Int(v) = self.0.as_ref() {
            return Some(v.0.clone())
        }
        None
    }

    fn as_dec(self) -> Option<f32> {
        if let UnitType::Dec(v) = self.0.as_ref() {
            if let Some(v) = v.to_small() {
                return Some(v)
            }
        }
        None
    }

    fn as_dec_big(self) -> Option<Rc<BigRational>> {
        if let UnitType::Dec(v) = self.0.as_ref() {
            return Some(v.0.clone())
        }
        None
    }

    fn as_str(self) -> Option<Rc<String>> {
        if let UnitType::Str(s) = self.0.as_ref() {
            return Some(s.clone())
        }
        None
    }

    fn as_path(self) -> Option<Rc<Path>> {
        if let UnitType::Ref(path) = self.0.as_ref() {
            return Some(path.clone())
        }
        None
    }

    fn as_stream(self) -> Option<(Unit, String, Addr)> {
        if let UnitType::Stream(u, serv, addr) = self.0.as_ref() {
            return Some((u.clone(), Rc::unwrap_or_clone(serv.clone()), Rc::unwrap_or_clone(addr.clone())))
        }
        None
    }

    fn as_pair(self) -> Option<(Unit, Unit)> {
        if let UnitType::Pair(u0, u1) = self.0.as_ref() {
            return Some((u0.clone(), u1.clone()))
        }
        None
    }

    fn as_list(self) -> Option<Rc<Vec<Unit>>> {
        if let UnitType::List(lst) = self.0.as_ref() {
            return Some(lst.clone())
        }
        None
    }

    fn as_map(self) -> Option<Rc<Vec<(Unit, Unit)>>> {
        if let UnitType::Map(map) = self.0.as_ref() {
            return Some(map.clone())
        }
        None
    }

    fn as_map_find(self, sch: &str) -> Option<Unit> {
        if let UnitType::Map(map) = self.0.as_ref() {
            return map.iter()
                .filter_map(|(u0, u1)| Some((u0.clone().as_str()?, u1.clone())))
                .find_map(|(s, u)| {
                    if s.as_str() == sch {
                        return Some(u)
                    }
                    None
                })
        }
        None
    }
}

impl UnitReadAsyncI for Unit {
    fn read_async<'a>(self, ath: Rc<String>, orig: Unit, kern: &'a Mutex<Kern>) -> UnitReadAsync<'a> {
        thread!({
            match self.0.as_ref() {
                UnitType::Ref(path) => Ok(orig.find(path.iter().map(|s| s.as_str())).map(|u| (u, ath))),
                UnitType::Stream(msg, serv, _addr) => {
                    let run = TaskRun(msg.clone(), Rc::unwrap_or_clone(serv.clone()));
                    let id = kern.lock().reg_task(&ath, "unit.read", run)?;

                    let res = maybe!(task_result!(id, kern));
                    let msg = maybe_ok!(res.msg.as_map_find("msg"));

                    Ok(Some((msg, Rc::new(res.ath))))
                },
                _ => Ok(Some((self.clone(), ath)))
            }
        })
    }

    fn as_map_find_async<'a>(self, sch: String, ath: Rc<String>, orig: Unit, kern: &'a Mutex<Kern>) -> UnitReadAsync<'a> {
        thread!({
            if let Some(msg) = self.as_map_find(&sch) {
                return thread_await!(msg.read_async(ath, orig, kern))
            }
            Ok(None)
        })
    }
}

fn char_no_quoted(c: char) -> bool {
    c.is_alphanumeric() || c == '.' || c == '#' || c == '_' || c == '.'
}

impl Display for DisplayStr {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self.0.0.as_ref() {
            UnitType::Str(s) => write!(f, "{}", s.replace("\\n", "\n").replace("\\r", "\r")),
            _ => write!(f, "{}", self.0)
        }
    }
}

impl Display for DisplayShort {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self.1.0.as_ref() {
            UnitType::None => write!(f, "-"),
            UnitType::Bool(v) => write!(f, "{}", if *v {"t"} else {"f"}),
            UnitType::Byte(v) => write!(f, "{:#02x}", *v),
            UnitType::Int(v) => write!(f, "{}", self.shrt(format!("{}", v.0))),
            UnitType::Dec(v) =>
                match v.to_small() {
                    Some(v) => write!(f, "{v}"),
                    None => write!(f, "{}", self.shrt(format!("{}", v.0))) // FIXME: use `<i>.<i>` format
                }
            UnitType::Str(s) => {
                if s.as_str().chars().all(char_no_quoted) {
                    write!(f, "{}", self.shrt(Rc::unwrap_or_clone(s.clone())))
                } else {
                    write!(f, "`{}`", self.shrt(Rc::unwrap_or_clone(s.clone())))
                }
            },
            UnitType::Ref(path) => write!(f, "@{}", self.shrt(path.join("."))),
            UnitType::Stream(msg, serv, addr) => write!(f, "{}@{serv}:{addr}", DisplayShort(self.0, msg.clone())),
            UnitType::Pair(u0, u1) => write!(f, "({} {})", DisplayShort(self.0, u0.clone()), DisplayShort(self.0, u1.clone())),
            UnitType::List(lst) => {
                let end = if lst.len() > self.0 {
                    " .."
                } else {
                    ""
                };
                write!(f, "[{}{}]", lst.iter().map(|u| format!("{}", DisplayShort(self.0, u.clone()))).take(self.0).collect::<Vec<_>>().join(" "), end)
            },
            UnitType::Map(map) => {
                let end = if map.len() > self.0 {
                    " .."
                } else {
                    ""
                };
                write!(f, "{{{}{}}}", map.iter().map(|(u0, u1)| format!("{}:{}", DisplayShort(self.0, u0.clone()), DisplayShort(self.0, u1.clone()))).take(self.0).collect::<Vec<_>>().join(" "), end)
            }
        }
    }
}

impl Display for DisplayNice {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self.2.0.as_ref() {
            UnitType::None => write!(f, "-"),
            UnitType::Bool(v) => write!(f, "{}", if *v {"t"} else {"f"}),
            UnitType::Byte(v) => write!(f, "{:#02x}", *v),
            UnitType::Int(v) => write!(f, "{}", v.0),
            UnitType::Dec(v) =>
                match v.to_small() {
                    Some(v) => write!(f, "{v}"),
                    None => write!(f, "{}", v.0) // FIXME: use `<i>.<i>` format
                }
            UnitType::Str(s) => {
                if s.as_str().chars().all(char_no_quoted) {
                    write!(f, "{}", s.replace("\n", "\\n").replace("\r", "\\r"))
                } else {
                    write!(f, "`{}`", s.replace("\n", "\\n").replace("\r", "\\r"))
                }
            },
            UnitType::Ref(path) => write!(f, "@{}", path.join(".")),
            UnitType::Stream(msg, serv, addr) => write!(f, "{}@{serv}:{addr}", DisplayNice(self.0, self.1, msg.clone())),
            UnitType::Pair(u0, u1) => write!(f, "({u0} {u1})"),
            UnitType::List(lst) => write!(f, "[\n{}\n{}]", lst.iter().map(|u| format!("{}{}", " ".repeat(self.1 * (self.0 + 1)), DisplayNice(self.0 + 1, self.1, u.clone()))).collect::<Vec<_>>().join("\n"), " ".repeat(self.1 * (self.0))),
            UnitType::Map(map) => write!(f, "{{\n{}\n{}}}", map.iter().map(|(u0, u1)| format!("{}{}:{}", " ".repeat(self.1 * (self.0 + 1)), DisplayNice(self.0 + 1, self.1, u0.clone()), DisplayNice(self.0 + 1, self.1, u1.clone()))).collect::<Vec<_>>().join("\n"), " ".repeat(self.1 * (self.0))),
        }
    }
}

impl Display for Unit {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self.0.as_ref() {
            UnitType::None => write!(f, "-"),
            UnitType::Bool(v) => write!(f, "{}", if *v {"t"} else {"f"}),
            UnitType::Byte(v) => write!(f, "{:#02x}", *v),
            UnitType::Int(v) => write!(f, "{}", v.0),
            UnitType::Dec(v) =>
                match v.to_small() {
                    Some(v) => write!(f, "{v}"),
                    None => write!(f, "{}", v.0) // FIXME: use `<i>.<i>` format
                }
            UnitType::Str(s) => {
                if s.as_str().chars().all(char_no_quoted) {
                    write!(f, "{}", s.replace("\n", "\\n").replace("\r", "\\r"))
                } else {
                    write!(f, "`{}`", s.replace("\n", "\\n").replace("\r", "\\r"))
                }
            },
            UnitType::Ref(path) => write!(f, "@{}", path.join(".")),
            UnitType::Stream(msg, serv, addr) => write!(f, "{msg}@{serv}:{addr}"),
            UnitType::Pair(u0, u1) => write!(f, "({u0} {u1})"),
            UnitType::List(lst) => write!(f, "[{}]", lst.iter().map(|u| format!("{u}")).collect::<Vec<_>>().join(" ")),
            UnitType::Map(map) => write!(f, "{{{}}}", map.iter().map(|(u0, u1)| format!("{u0}:{u1}")).collect::<Vec<_>>().join(" ")),
        }
    }
}

impl PartialOrd for Unit {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        match self.0.as_ref() {
            UnitType::Bool(a) =>
                match other.0.as_ref() {
                    UnitType::Bool(b) => a.partial_cmp(b),
                    _ => None
                },
            UnitType::Byte(a) =>
                match other.0.as_ref() {
                    UnitType::Byte(b) => a.partial_cmp(b),
                    _ => None
                },
            UnitType::Int(a) =>
                match other.0.as_ref() {
                    UnitType::Int(b) => a.partial_cmp(b),
                    UnitType::Dec(b) => a.0.as_ref().partial_cmp(&b.0.to_integer()),
                    _ => None
                },
            UnitType::Dec(a) =>
            match other.0.as_ref() {
                UnitType::Dec(b) => a.partial_cmp(b),
                UnitType::Int(b) => a.0.to_integer().partial_cmp(b.0.as_ref()),
                _ => None
            },
            UnitType::Str(a) =>
                match other.0.as_ref() {
                    UnitType::Str(b) => {
                        let a = a.chars().next()?;
                        let b = b.chars().next()?;
                        a.partial_cmp(&b)
                    },
                    _ => None
                }
            _ => None
        }
    }
}

impl UnitAsBytes for Unit {
    fn as_bytes(self) -> Vec<u8> {
        match self.0.as_ref() {
            UnitType::None => vec![UnitBin::None as u8],
            UnitType::Bool(v) => vec![UnitBin::Bool as u8, if *v {1} else {0}],
            UnitType::Byte(v) => vec![UnitBin::Byte as u8, *v],
            UnitType::Int(v) => {
                if v.0.is_zero() {
                    return vec![UnitBin::Zero as u8]
                }

                if let Some(v) = v.to_nat() {
                    let b = v.to_le_bytes();
                    return match v {
                        0..=255 => [UnitBin::Uint8 as u8].into_iter().chain(b.into_iter().take(1)).collect(),
                        256..=65535 => [UnitBin::Uint16 as u8].into_iter().chain(b.into_iter().take(2)).collect(),
                        65536..=16777215 => [UnitBin::Uint24 as u8].into_iter().chain(b.into_iter().take(3)).collect(),
                        _ => [UnitBin::IntNat as u8].into_iter().chain(b).collect()
                    }
                }

                if let Some(v) = v.to_small() {
                    let b = v.to_le_bytes();
                    return match v {
                        -128..=127 => [UnitBin::Int8 as u8].into_iter().chain(b.into_iter().take(1)).collect(),
                        -32768..=32767 => [UnitBin::Int16 as u8].into_iter().chain(b.into_iter().take(2)).collect(),
                        -8388608..=8388607 => [UnitBin::Int24 as u8].into_iter().chain(b.into_iter().take(3)).collect(),
                        _ => [UnitBin::Int as u8].into_iter().chain(b).collect()
                    };
                }

                let (s, b) = v.0.to_bytes_le();
                let len = (b.len() as u32).to_le_bytes();
                [UnitBin::IntBig as u8].into_iter()
                    .chain([if let Sign::Minus = s {1} else {0}])
                    .chain(len)
                    .chain(b)
                    .collect()
            },
            UnitType::Dec(v) =>
                match v.to_small() {
                    Some(v) => [UnitBin::Dec as u8].into_iter().chain(v.to_le_bytes()).collect(),
                    None => {
                        let (s, b0) = v.0.numer().to_bytes_le();
                        let len0 = (b0.len() as u32).to_le_bytes();

                        let (_, b1) = v.0.denom().to_bytes_le();
                        let len1 = (b1.len() as u32).to_le_bytes();

                        [UnitBin::DecBig as u8].into_iter()
                            .chain([if let Sign::Minus = s {1} else {0}])
                            .chain(len0)
                            .chain(b0)
                            .chain(len1)
                            .chain(b1)
                            .collect()
                    }
                },
            UnitType::Str(s) => {
                let len = s.len();
                let len_b = (s.len() as u32).to_le_bytes();

                let head_b = match len {
                    0..=255 => [UnitBin::Str8 as u8].into_iter().chain(len_b.into_iter().take(1)).collect::<Vec<u8>>(),
                    0..=65535 => [UnitBin::Str16 as u8].into_iter().chain(len_b.into_iter().take(2)).collect::<Vec<u8>>(),
                    0..=16777215 => [UnitBin::Str24 as u8].into_iter().chain(len_b.into_iter().take(3)).collect::<Vec<u8>>(),
                    _ => [UnitBin::Str as u8].into_iter().chain(len_b).collect::<Vec<u8>>()
                };

                head_b.into_iter()
                .chain(s.as_bytes().into_iter().cloned())
                .collect()
            },
            UnitType::Ref(path) => {
                let s = path.join(".");

                [UnitBin::Ref as u8].into_iter()
                .chain((s.len() as u32).to_le_bytes())
                .chain(s.as_bytes().into_iter().cloned())
                .collect()
            },
            UnitType::Stream(msg, serv, addr) => [UnitBin::Stream as u8].into_iter()
                .chain(msg.clone().as_bytes())
                .chain((serv.len() as u32).to_le_bytes())
                .chain(serv.as_bytes().into_iter().cloned())
                .chain(match addr.as_ref() {
                    Addr::Local => vec![UnitBin::AddrLoc as u8],
                    Addr::Remote(addr) => [UnitBin::AddrRemote as u8].into_iter().chain(addr.into_iter().flat_map(|e| e.to_le_bytes())).collect::<Vec<u8>>()
                }).collect(),
            UnitType::Pair(u0, u1) => {
                if let Some((u0, u1)) = u0.clone().as_uint().and_then(|u0| Some((u0, u1.clone().as_uint()?))) {
                    if u1 <= 16777215 {
                        let u0_b = u0.to_le_bytes().into_iter();
                        let u1_b = u1.to_le_bytes().into_iter().take(3);
                        match u0 {
                            0..=255 => return [UnitBin::PairUint8Uint24 as u8].into_iter().chain(u0_b.take(1)).chain(u1_b).collect(),
                            0..=65535 => return [UnitBin::PairUint16Uint24 as u8].into_iter().chain(u0_b.take(2)).chain(u1_b).collect(),
                            0..=16777215 => return [UnitBin::PairUint24Uint24 as u8].into_iter().chain(u0_b.take(3)).chain(u1_b).collect(),
                            _ => ()
                        }
                    }
                }

                [UnitBin::Pair as u8].into_iter()
                .chain(u0.clone().as_bytes())
                .chain(u1.clone().as_bytes())
                .collect()
            }
            UnitType::List(lst) => {
                let len = lst.len();
                let len_b = (lst.len() as u32).to_le_bytes();

                let head_b = match len {
                    0..=255 => [UnitBin::List8 as u8].into_iter().chain(len_b.into_iter().take(1)).collect::<Vec<u8>>(),
                    0..=65535 => [UnitBin::List16 as u8].into_iter().chain(len_b.into_iter().take(2)).collect::<Vec<u8>>(),
                    0..=16777215 => [UnitBin::List24 as u8].into_iter().chain(len_b.into_iter().take(3)).collect::<Vec<u8>>(),
                    _ => [UnitBin::List as u8].into_iter().chain(len_b).collect::<Vec<u8>>()
                };

                head_b.into_iter()
                .chain(lst.iter().flat_map(|u| u.clone().as_bytes()))
                .collect()
            },
            UnitType::Map(map) => {
                let len = map.len();
                let len_b = (map.len() as u32).to_le_bytes();

                let head_b = match len {
                    0..=255 => [UnitBin::Map8 as u8].into_iter().chain(len_b.into_iter().take(1)).collect::<Vec<u8>>(),
                    0..=65535 => [UnitBin::Map16 as u8].into_iter().chain(len_b.into_iter().take(2)).collect::<Vec<u8>>(),
                    0..=16777215 => [UnitBin::Map24 as u8].into_iter().chain(len_b.into_iter().take(3)).collect::<Vec<u8>>(),
                    _ => [UnitBin::Map as u8].into_iter().chain(len_b).collect::<Vec<u8>>()
                };

                head_b.into_iter()
                .chain(
                    map.iter().flat_map(|(u0, u1)| u0.clone().as_bytes().into_iter().chain(u1.clone().as_bytes()).collect::<Vec<u8>>())
                )
                .collect()
            }
        }
    }
}

impl<'a> UnitParse<'a, u8, Iter<'a, u8>> for Unit {
    fn parse(it: Iter<'a, u8>) -> Result<(Unit, Iter<'a, u8>), UnitParseErr> {
        match *it.clone().next().ok_or(UnitParseErr::UnexpectedEnd)? {
            _b if _b == UnitBin::None as u8 => Self::parse_none(it),
            _b if _b == UnitBin::Bool as u8 => Self::parse_bool(it),
            _b if _b == UnitBin::Byte as u8 => Self::parse_byte(it),
            _b if _b == UnitBin::Zero as u8 => Self::parse_int(it),
            _b if _b == UnitBin::Int as u8 => Self::parse_int(it),
            _b if _b == UnitBin::Int8 as u8 => Self::parse_int(it),
            _b if _b == UnitBin::Int16 as u8 => Self::parse_int(it),
            _b if _b == UnitBin::Int24 as u8 => Self::parse_int(it),
            _b if _b == UnitBin::IntNat as u8 => Self::parse_int(it),
            _b if _b == UnitBin::Uint8 as u8 => Self::parse_int(it),
            _b if _b == UnitBin::Uint16 as u8 => Self::parse_int(it),
            _b if _b == UnitBin::Uint24 as u8 => Self::parse_int(it),
            _b if _b == UnitBin::IntBig as u8 => Self::parse_int(it),
            _b if _b == UnitBin::Dec as u8 => Self::parse_dec(it),
            _b if _b == UnitBin::DecBig as u8 => Self::parse_dec(it),
            _b if _b == UnitBin::Str as u8 => Self::parse_str(it),
            _b if _b == UnitBin::Str8 as u8 => Self::parse_str(it),
            _b if _b == UnitBin::Str16 as u8 => Self::parse_str(it),
            _b if _b == UnitBin::Str24 as u8 => Self::parse_str(it),
            _b if _b == UnitBin::Ref as u8 => Self::parse_ref(it),
            _b if _b == UnitBin::Pair as u8 => Self::parse_pair(it),
            _b if _b == UnitBin::PairUint8Uint24 as u8 => Self::parse_pair(it),
            _b if _b == UnitBin::PairUint16Uint24 as u8 => Self::parse_pair(it),
            _b if _b == UnitBin::PairUint24Uint24 as u8 => Self::parse_pair(it),
            _b if _b == UnitBin::PairUint16Int32 as u8 => Self::parse_pair(it),
            _b if _b == UnitBin::List as u8 => Self::parse_list(it),
            _b if _b == UnitBin::List8 as u8 => Self::parse_list(it),
            _b if _b == UnitBin::List16 as u8 => Self::parse_list(it),
            _b if _b == UnitBin::List24 as u8 => Self::parse_list(it),
            _b if _b == UnitBin::Map as u8 => Self::parse_map(it),
            _b if _b == UnitBin::Map8 as u8 => Self::parse_map(it),
            _b if _b == UnitBin::Map16 as u8 => Self::parse_map(it),
            _b if _b == UnitBin::Map24 as u8 => Self::parse_map(it),
            _b if _b == UnitBin::Stream as u8 => Self::parse_stream(it),
            _ => Err(UnitParseErr::NotUnit)
        }
    }

    fn parse_none(mut it: Iter<'a, u8>) -> Result<(Unit, Iter<'a, u8>), UnitParseErr> {
        let b = *it.next().ok_or(UnitParseErr::UnexpectedEnd)?;
        if b != UnitBin::None as u8 {
            return Err(UnitParseErr::NotNone)
        }
        Ok((Unit::none(), it))
    }

    fn parse_bool(mut it: Iter<'a, u8>) -> Result<(Unit, Iter<'a, u8>), UnitParseErr> {
        let b = *it.next().ok_or(UnitParseErr::UnexpectedEnd)?;
        if b != UnitBin::Bool as u8 {
            return Err(UnitParseErr::NotBool)
        }

        match *it.next().ok_or(UnitParseErr::UnexpectedEnd)? {
            0 => Ok((Unit::bool(false), it)),
            1 => Ok((Unit::bool(true), it)),
            _ => Err(UnitParseErr::NotBool)
        }
    }

    fn parse_byte(mut it: Iter<'a, u8>) -> Result<(Unit, Iter<'a, u8>), UnitParseErr> {
        let b = *it.next().ok_or(UnitParseErr::UnexpectedEnd)?;
        if b != UnitBin::Byte as u8 {
            return Err(UnitParseErr::NotByte)
        }

        let v = *it.next().ok_or(UnitParseErr::UnexpectedEnd)?;
        Ok((Unit::byte(v), it))
    }

    fn parse_int(mut it: Iter<'a, u8>) -> Result<(Unit, Iter<'a, u8>), UnitParseErr> {
        match *it.next().ok_or(UnitParseErr::UnexpectedEnd)? {
            _b if _b == UnitBin::Zero as u8 => Ok((Unit::int(0), it)),
            _b if _b == UnitBin::Int as u8 => {
                let bytes = [
                    *it.next().ok_or(UnitParseErr::UnexpectedEnd)?,
                    *it.next().ok_or(UnitParseErr::UnexpectedEnd)?,
                    *it.next().ok_or(UnitParseErr::UnexpectedEnd)?,
                    *it.next().ok_or(UnitParseErr::UnexpectedEnd)?
                ];
                let v = <i32>::from_le_bytes(bytes);
                Ok((Unit::int(v), it))
            },
            _b if _b == UnitBin::Int8 as u8 => {
                let bytes = [
                    *it.next().ok_or(UnitParseErr::UnexpectedEnd)?,
                    0,
                    0,
                    0
                ];
                let v = <i32>::from_le_bytes(bytes);
                Ok((Unit::int(v), it))
            },
            _b if _b == UnitBin::Int16 as u8 => {
                let bytes = [
                    *it.next().ok_or(UnitParseErr::UnexpectedEnd)?,
                    *it.next().ok_or(UnitParseErr::UnexpectedEnd)?,
                    0,
                    0
                ];
                let v = <i32>::from_le_bytes(bytes);
                Ok((Unit::int(v), it))
            },
            _b if _b == UnitBin::Int24 as u8 => {
                let bytes = [
                    *it.next().ok_or(UnitParseErr::UnexpectedEnd)?,
                    *it.next().ok_or(UnitParseErr::UnexpectedEnd)?,
                    *it.next().ok_or(UnitParseErr::UnexpectedEnd)?,
                    0
                ];
                let v = <i32>::from_le_bytes(bytes);
                Ok((Unit::int(v), it))
            },
            _b if _b == UnitBin::IntNat as u8 => {
                let bytes = [
                    *it.next().ok_or(UnitParseErr::UnexpectedEnd)?,
                    *it.next().ok_or(UnitParseErr::UnexpectedEnd)?,
                    *it.next().ok_or(UnitParseErr::UnexpectedEnd)?,
                    *it.next().ok_or(UnitParseErr::UnexpectedEnd)?
                ];
                let v = <u32>::from_le_bytes(bytes);
                Ok((Unit::uint(v), it))
            },
            _b if _b == UnitBin::Uint8 as u8 => {
                let bytes = [
                    *it.next().ok_or(UnitParseErr::UnexpectedEnd)?,
                    0,
                    0,
                    0
                ];
                let v = <u32>::from_le_bytes(bytes);
                Ok((Unit::uint(v), it))
            },
            _b if _b == UnitBin::Uint16 as u8 => {
                let bytes = [
                    *it.next().ok_or(UnitParseErr::UnexpectedEnd)?,
                    *it.next().ok_or(UnitParseErr::UnexpectedEnd)?,
                    0,
                    0
                ];
                let v = <u32>::from_le_bytes(bytes);
                Ok((Unit::uint(v), it))
            },
            _b if _b == UnitBin::Uint24 as u8 => {
                let bytes = [
                    *it.next().ok_or(UnitParseErr::UnexpectedEnd)?,
                    *it.next().ok_or(UnitParseErr::UnexpectedEnd)?,
                    *it.next().ok_or(UnitParseErr::UnexpectedEnd)?,
                    0
                ];
                let v = <u32>::from_le_bytes(bytes);
                Ok((Unit::uint(v), it))
            },
            _b if _b == UnitBin::IntBig as u8 => {
                let sign = match *it.next().ok_or(UnitParseErr::UnexpectedEnd)? {
                    0 => Sign::Plus,
                    1 => Sign::Minus,
                    _ => return Err(UnitParseErr::InvalidSign)
                };

                let bytes = [
                    *it.next().ok_or(UnitParseErr::UnexpectedEnd)?,
                    *it.next().ok_or(UnitParseErr::UnexpectedEnd)?,
                    *it.next().ok_or(UnitParseErr::UnexpectedEnd)?,
                    *it.next().ok_or(UnitParseErr::UnexpectedEnd)?
                ];
                let len = <u32>::from_le_bytes(bytes);

                let bytes = (0..len).map(|_| it.next().map(|v| *v)).try_collect::<Vec<_>>().ok_or(UnitParseErr::UnexpectedEnd)?;
                let big = BigInt::from_bytes_le(sign, &bytes);

                Ok((Unit::int_big(big), it))

            },
            _ => Err(UnitParseErr::NotInt)
        }
    }

    fn parse_dec(mut it: Iter<'a, u8>) -> Result<(Unit, Iter<'a, u8>), UnitParseErr> {
        match *it.next().ok_or(UnitParseErr::UnexpectedEnd)? {
            _b if _b == UnitBin::Dec as u8 => {
                let bytes = [
                    *it.next().ok_or(UnitParseErr::UnexpectedEnd)?,
                    *it.next().ok_or(UnitParseErr::UnexpectedEnd)?,
                    *it.next().ok_or(UnitParseErr::UnexpectedEnd)?,
                    *it.next().ok_or(UnitParseErr::UnexpectedEnd)?
                ];
                let v = <f32>::from_le_bytes(bytes);
                Ok((Unit::dec(v), it))
            },
            _b if _b == UnitBin::DecBig as u8 => {
                let sign = match *it.next().ok_or(UnitParseErr::UnexpectedEnd)? {
                    0 => Sign::Plus,
                    1 => Sign::Minus,
                    _ => return Err(UnitParseErr::InvalidSign)
                };

                // numer
                let bytes = [
                    *it.next().ok_or(UnitParseErr::UnexpectedEnd)?,
                    *it.next().ok_or(UnitParseErr::UnexpectedEnd)?,
                    *it.next().ok_or(UnitParseErr::UnexpectedEnd)?,
                    *it.next().ok_or(UnitParseErr::UnexpectedEnd)?
                ];
                let len = <u32>::from_le_bytes(bytes);

                let bytes = (0..len).map(|_| it.next().map(|v| *v)).try_collect::<Vec<_>>().ok_or(UnitParseErr::UnexpectedEnd)?;
                let numer = BigInt::from_bytes_le(sign, &bytes);

                // denom
                let bytes = [
                    *it.next().ok_or(UnitParseErr::UnexpectedEnd)?,
                    *it.next().ok_or(UnitParseErr::UnexpectedEnd)?,
                    *it.next().ok_or(UnitParseErr::UnexpectedEnd)?,
                    *it.next().ok_or(UnitParseErr::UnexpectedEnd)?
                ];
                let len = <u32>::from_le_bytes(bytes);

                let bytes = (0..len).map(|_| it.next().map(|v| *v)).try_collect::<Vec<_>>().ok_or(UnitParseErr::UnexpectedEnd)?;
                let denom = BigInt::from_bytes_le(sign, &bytes);

                let big = BigRational::new(numer, denom);
                Ok((Unit::dec_big(big), it))
            },
            _ => Err(UnitParseErr::NotDec)
        }
    }

    fn parse_str(mut it: Iter<'a, u8>) -> Result<(Unit, Iter<'a, u8>), UnitParseErr> {
        let b = *it.next().ok_or(UnitParseErr::UnexpectedEnd)?;

        let bytes = match b {
            _b if _b == UnitBin::Str as u8 =>
                [
                    *it.next().ok_or(UnitParseErr::UnexpectedEnd)?,
                    *it.next().ok_or(UnitParseErr::UnexpectedEnd)?,
                    *it.next().ok_or(UnitParseErr::UnexpectedEnd)?,
                    *it.next().ok_or(UnitParseErr::UnexpectedEnd)?
                ],
            _b if _b == UnitBin::Str8 as u8 =>
                [
                    *it.next().ok_or(UnitParseErr::UnexpectedEnd)?,
                    0,
                    0,
                    0
                ],
            _b if _b == UnitBin::Str16 as u8 =>
                [
                    *it.next().ok_or(UnitParseErr::UnexpectedEnd)?,
                    *it.next().ok_or(UnitParseErr::UnexpectedEnd)?,
                    0,
                    0
                ],
            _b if _b == UnitBin::Str24 as u8 =>
                [
                    *it.next().ok_or(UnitParseErr::UnexpectedEnd)?,
                    *it.next().ok_or(UnitParseErr::UnexpectedEnd)?,
                    *it.next().ok_or(UnitParseErr::UnexpectedEnd)?,
                    0
                ],
            _ => return Err(UnitParseErr::NotStr)
        };
        let len = <u32>::from_le_bytes(bytes);

        let bytes = (0..len).map(|_| it.next().map(|v| *v)).try_collect::<Vec<_>>().ok_or(UnitParseErr::UnexpectedEnd)?;
        let s = String::from_utf8(bytes).map_err(|_| UnitParseErr::NotStr)?;
    
        Ok((Unit::str(&s), it))
    }

    fn parse_ref(mut it: Iter<'a, u8>) -> Result<(Unit, Iter<'a, u8>), UnitParseErr> {
        let b = *it.next().ok_or(UnitParseErr::UnexpectedEnd)?;
        if b != UnitBin::Ref as u8 {
            return Err(UnitParseErr::NotRef)
        }

        let bytes = [
            *it.next().ok_or(UnitParseErr::UnexpectedEnd)?,
            *it.next().ok_or(UnitParseErr::UnexpectedEnd)?,
            *it.next().ok_or(UnitParseErr::UnexpectedEnd)?,
            *it.next().ok_or(UnitParseErr::UnexpectedEnd)?
        ];
        let len = <u32>::from_le_bytes(bytes);

        let bytes = (0..len).map(|_| it.next().map(|v| *v)).try_collect::<Vec<_>>().ok_or(UnitParseErr::UnexpectedEnd)?;
        let s = String::from_utf8(bytes).map_err(|_| UnitParseErr::NotStr)?;
        
        if !s.chars().all(char_no_quoted) {
            return Err(UnitParseErr::RefInvalidPath);
        }

        let path = s.split(".").collect::<Vec<_>>();
        Ok((Unit::path(&path), it))
    }

    fn parse_stream(mut it: Iter<'a, u8>) -> Result<(Unit, Iter<'a, u8>), UnitParseErr> {
        let b = *it.next().ok_or(UnitParseErr::UnexpectedEnd)?;
        if b != UnitBin::Stream as u8 {
            return Err(UnitParseErr::NotStream)
        }

        // msg
        let (msg, mut it) = Unit::parse(it)?;

        // serv
        let bytes = [
            *it.next().ok_or(UnitParseErr::UnexpectedEnd)?,
            *it.next().ok_or(UnitParseErr::UnexpectedEnd)?,
            *it.next().ok_or(UnitParseErr::UnexpectedEnd)?,
            *it.next().ok_or(UnitParseErr::UnexpectedEnd)?
        ];
        let len = <u32>::from_le_bytes(bytes);

        let bytes = (0..len).map(|_| it.next().map(|v| *v)).try_collect::<Vec<_>>().ok_or(UnitParseErr::UnexpectedEnd)?;
        let serv = String::from_utf8(bytes).map_err(|_| UnitParseErr::NotStr)?;

        // addr
        let addr = match *it.next().ok_or(UnitParseErr::UnexpectedEnd)? {
            _b if _b == UnitBin::AddrLoc as u8 => Addr::Local,
            _b if _b == UnitBin::AddrRemote as u8 => {
                let addr = (0..8).map(|_| {
                    let bytes = [
                        *it.next()?,
                        *it.next()?
                    ];
                    Some(<u16>::from_le_bytes(bytes))
                }).try_collect::<Vec<_>>()
                    .ok_or(UnitParseErr::UnexpectedEnd)?
                    .try_into()
                    .map_err(|_| UnitParseErr::UnexpectedEnd)?;

                Addr::Remote(addr)
            },
            _ => return Err(UnitParseErr::InvalidAddr)
        };

        Ok((Unit::stream(msg, &serv, addr), it))
    }

    fn parse_pair(mut it: Iter<'a, u8>) -> Result<(Unit, Iter<'a, u8>), UnitParseErr> {
        let b = *it.next().ok_or(UnitParseErr::UnexpectedEnd)?;
        match b {
            _b if _b == UnitBin::Pair as u8 => {
                let (u0, it) = Unit::parse(it)?;
                let (u1, it) = Unit::parse(it)?;

                Ok((Unit::pair(u0, u1), it))
            },
            _b if _b == UnitBin::PairUint8Uint24 as u8 => {
                let bytes = [
                    *it.next().ok_or(UnitParseErr::UnexpectedEnd)?,
                    0,
                    0,
                    0
                ];
                let u0 = <u32>::from_le_bytes(bytes);

                let bytes = [
                    *it.next().ok_or(UnitParseErr::UnexpectedEnd)?,
                    *it.next().ok_or(UnitParseErr::UnexpectedEnd)?,
                    *it.next().ok_or(UnitParseErr::UnexpectedEnd)?,
                    0
                ];
                let u1 = <u32>::from_le_bytes(bytes);
                Ok((Unit::pair(Unit::uint(u0), Unit::uint(u1)), it))
            },
            _b if _b == UnitBin::PairUint16Uint24 as u8 => {
                let bytes = [
                    *it.next().ok_or(UnitParseErr::UnexpectedEnd)?,
                    *it.next().ok_or(UnitParseErr::UnexpectedEnd)?,
                    0,
                    0
                ];
                let u0 = <u32>::from_le_bytes(bytes);

                let bytes = [
                    *it.next().ok_or(UnitParseErr::UnexpectedEnd)?,
                    *it.next().ok_or(UnitParseErr::UnexpectedEnd)?,
                    *it.next().ok_or(UnitParseErr::UnexpectedEnd)?,
                    0
                ];
                let u1 = <u32>::from_le_bytes(bytes);
                Ok((Unit::pair(Unit::uint(u0), Unit::uint(u1)), it))
            },
            _b if _b == UnitBin::PairUint24Uint24 as u8 => {
                let bytes = [
                    *it.next().ok_or(UnitParseErr::UnexpectedEnd)?,
                    *it.next().ok_or(UnitParseErr::UnexpectedEnd)?,
                    *it.next().ok_or(UnitParseErr::UnexpectedEnd)?,
                    0
                ];
                let u0 = <u32>::from_le_bytes(bytes);

                let bytes = [
                    *it.next().ok_or(UnitParseErr::UnexpectedEnd)?,
                    *it.next().ok_or(UnitParseErr::UnexpectedEnd)?,
                    *it.next().ok_or(UnitParseErr::UnexpectedEnd)?,
                    0
                ];
                let u1 = <u32>::from_le_bytes(bytes);
                Ok((Unit::pair(Unit::uint(u0), Unit::uint(u1)), it))
            },
            _b if _b == UnitBin::PairUint16Int32 as u8 => {
                let bytes = [
                    *it.next().ok_or(UnitParseErr::UnexpectedEnd)?,
                    *it.next().ok_or(UnitParseErr::UnexpectedEnd)?,
                    0,
                    0
                ];
                let u0 = <u32>::from_le_bytes(bytes);

                let bytes = [
                    *it.next().ok_or(UnitParseErr::UnexpectedEnd)?,
                    *it.next().ok_or(UnitParseErr::UnexpectedEnd)?,
                    *it.next().ok_or(UnitParseErr::UnexpectedEnd)?,
                    *it.next().ok_or(UnitParseErr::UnexpectedEnd)?,
                ];
                let u1 = <i32>::from_le_bytes(bytes);
                Ok((Unit::pair(Unit::uint(u0), Unit::int(u1)), it))
            },
            _ => Err(UnitParseErr::NotPair)
        }
    }

    fn parse_list_partial(mut it: Iter<'a, u8>) -> Result<(usize, Iter<'a, u8>), UnitParseErr> {
        let b = *it.next().ok_or(UnitParseErr::UnexpectedEnd)?;

        let bytes = match b {
            _b if _b == UnitBin::List as u8 =>
                [
                    *it.next().ok_or(UnitParseErr::UnexpectedEnd)?,
                    *it.next().ok_or(UnitParseErr::UnexpectedEnd)?,
                    *it.next().ok_or(UnitParseErr::UnexpectedEnd)?,
                    *it.next().ok_or(UnitParseErr::UnexpectedEnd)?
                ],
            _b if _b == UnitBin::List8 as u8 =>
                [
                    *it.next().ok_or(UnitParseErr::UnexpectedEnd)?,
                    0,
                    0,
                    0
                ],
            _b if _b == UnitBin::List16 as u8 =>
                [
                    *it.next().ok_or(UnitParseErr::UnexpectedEnd)?,
                    *it.next().ok_or(UnitParseErr::UnexpectedEnd)?,
                    0,
                    0
                ],
            _b if _b == UnitBin::List24 as u8 =>
                [
                    *it.next().ok_or(UnitParseErr::UnexpectedEnd)?,
                    *it.next().ok_or(UnitParseErr::UnexpectedEnd)?,
                    *it.next().ok_or(UnitParseErr::UnexpectedEnd)?,
                    0
                ],
            _ => return Err(UnitParseErr::NotList)
        };

        let len = <u32>::from_le_bytes(bytes);
        Ok((len as usize, it))
    }

    fn parse_list(it: Iter<'a, u8>) -> Result<(Unit, Iter<'a, u8>), UnitParseErr> {
        let (len, mut it) = Unit::parse_list_partial(it)?;

        let mut lst = Vec::with_capacity(len);
        for _ in 0..len {
            let (u, next) = Unit::parse(it)?;
            lst.push(u);
            it = next;
        }
        Ok((Unit::list(&lst), it))
    }

    fn parse_map(mut it: Iter<'a, u8>) -> Result<(Unit, Iter<'a, u8>), UnitParseErr> {
        let b = *it.next().ok_or(UnitParseErr::UnexpectedEnd)?;

        let bytes = match b {
            _b if _b == UnitBin::Map as u8 =>
                [
                    *it.next().ok_or(UnitParseErr::UnexpectedEnd)?,
                    *it.next().ok_or(UnitParseErr::UnexpectedEnd)?,
                    *it.next().ok_or(UnitParseErr::UnexpectedEnd)?,
                    *it.next().ok_or(UnitParseErr::UnexpectedEnd)?
                ],
            _b if _b == UnitBin::Map8 as u8 =>
                [
                    *it.next().ok_or(UnitParseErr::UnexpectedEnd)?,
                    0,
                    0,
                    0
                ],
            _b if _b == UnitBin::Map16 as u8 =>
                [
                    *it.next().ok_or(UnitParseErr::UnexpectedEnd)?,
                    *it.next().ok_or(UnitParseErr::UnexpectedEnd)?,
                    0,
                    0
                ],
            _b if _b == UnitBin::Map24 as u8 =>
                [
                    *it.next().ok_or(UnitParseErr::UnexpectedEnd)?,
                    *it.next().ok_or(UnitParseErr::UnexpectedEnd)?,
                    *it.next().ok_or(UnitParseErr::UnexpectedEnd)?,
                    0
                ],
            _ => return Err(UnitParseErr::NotMap)
        };

        let len = <u32>::from_le_bytes(bytes);

        let mut map = Vec::with_capacity(len as usize);
        for _ in 0..len {
            let (u0, next) = Unit::parse(it)?;
            let (u1, next) = Unit::parse(next)?;
            map.push((u0, u1));
            it = next;
        }
        Ok((Unit::map(&map), it))
    }
}

impl<I> UnitParse<'_, char, I> for Unit where I: Iterator<Item = char> + Clone {
    fn parse(it: I) -> Result<(Unit, I), UnitParseErr> {
        if let Ok((u, it)) = Self::parse_stream(it.clone()) {
            return Ok((u, it))
        }

        Err(UnitParseErr::NotUnit)
    }

    fn parse_ch(expect: char, mut it: I) -> Result<I, UnitParseErr> {
        let ch = it.next().ok_or(UnitParseErr::UnexpectedEnd)?;
        if ch != expect {
            return Err(UnitParseErr::UnexpectedChar);
        }
        Ok(it)
    }

    fn parse_ws(mut it: I) -> Result<(usize, I), UnitParseErr> {
        let mut tmp = it.clone();
        let cnt = (0..).map_while(|_| {
            if tmp.next()?.is_whitespace() {
                it = tmp.clone();
                return Some(())
            }
            None
        }).count();

        Ok((cnt, it))
    }

    fn parse_none(it: I) -> Result<(Unit, I), UnitParseErr> {
        let it = Unit::parse_ch('-', it)?;
        Ok((Unit::none(), it))
    }

    fn parse_bool(mut it: I) -> Result<(Unit, I), UnitParseErr> {
        let ch = it.next().ok_or(UnitParseErr::UnexpectedEnd)?;
        let v = match ch {
            't' => true,
            'f' => false,
            _ => return Err(UnitParseErr::NotBool)
        };

        if let Some(ch) = it.clone().next() {
            if char_no_quoted(ch) {
                return Err(UnitParseErr::UnexpectedChar)
            }
        }
        Ok((Unit::bool(v), it))
    }

    fn parse_byte(mut it: I) -> Result<(Unit, I), UnitParseErr> {
        if let Some(s) = (0..4).map(|_| it.next()).try_collect::<String>() {
            let v = u8::from_str_radix(s.trim_start_matches("0x"), 16).map_err(|_| UnitParseErr::NotByte)?;
            return Ok((Unit::byte(v), it))
        }
        Err(UnitParseErr::UnexpectedEnd)
    }

    fn parse_int(mut it: I) -> Result<(Unit, I), UnitParseErr> {
        let mut tmp = it.clone();
        let s = (0..).map_while(|_| {
            let c = tmp.next()?;
            if c.is_numeric() || c == '-' {
                it = tmp.clone();
                return Some(c)
            }
            None
        }).collect::<String>();

        if s.is_empty() {
            return Err(UnitParseErr::UnexpectedEnd);
        }

        let big = BigInt::parse_bytes(s.as_bytes(), 10).ok_or(UnitParseErr::NotInt)?;
        Ok((Unit::int_big(big), it))
    }

    fn parse_dec(mut it: I) -> Result<(Unit, I), UnitParseErr> { 
        let mut tmp = it.clone();
        let s = (0..).map_while(|_| {
            let c = tmp.next()?;
            if c.is_numeric() || c == '-' || c == '.' {
                it = tmp.clone();
                return Some(c)
            }
            None
        }).collect::<String>();

        if s.is_empty() {
            return Err(UnitParseErr::UnexpectedEnd);
        }

        // get ratio
        let (fst, scd) = s.split_once(".").ok_or(UnitParseErr::NotDec)?;

        let fst = BigInt::parse_bytes(fst.as_bytes(), 10).ok_or(UnitParseErr::NotDec)?;
        let scd = BigInt::parse_bytes(scd.as_bytes(), 10).ok_or(UnitParseErr::NotDec)?;

        let len = format!("{scd}").len(); 

        let denom = BigInt::from(10).pow(len as u32);
        let numer = fst * &denom + scd;

        let big = BigRational::new(numer, denom);
        Ok((Unit::dec_big(big), it))
    }

    fn parse_str(mut it: I) -> Result<(Unit, I), UnitParseErr> {
        let sep = it.clone().next().ok_or(UnitParseErr::UnexpectedEnd)?;
        if !(sep == '`' || sep == '\'' || sep == '"' || char_no_quoted(sep)) {
            return Err(UnitParseErr::NotStr);
        }

        // #ab_c.123
        let mut tmp = it.clone();
        let mut tmp2 = tmp.clone();
        let mut s = String::new();

        while let Some(c) = tmp.next() {
            if !char_no_quoted(c) {
                break;
            }
            tmp2 = tmp.clone();
            s.push(c);
        }

        if !s.is_empty() {
            return Ok((Unit::str(&s), tmp2))
        }

        it = tmp2;

        // <sep>..<sep>
        it.next();
        let s = (0..).map_while(|_| {
            let c = it.next()?;
            if c != sep {
                return Some(c)
            }
            None
        }).collect::<String>();

        if !s.is_empty() {
            return Ok((Unit::str(&s), it))
        }

        return Err(UnitParseErr::NotStr);
    }

    fn parse_ref(it: I) -> Result<(Unit, I), UnitParseErr> {
        let it = Unit::parse_ch('@', it)?;
        let (path, it) = Unit::parse_str(it)?;

        let path = path.as_str().ok_or(UnitParseErr::RefInvalidPath)?;
        let path = path.split(".").map(|s| s).collect::<Vec<_>>();

        if !path.iter().all(|s| s.chars().all(char_no_quoted)) {
            return Err(UnitParseErr::RefInvalidPath)
        }

        Ok((Unit::path(&path), it))
    }

    fn parse_stream(it: I) -> Result<(Unit, I), UnitParseErr> {
        let (mut u, mut it) = if let Ok((u, it)) = Self::parse_bool(it.clone()) {
            (u, it)
        } else if let Ok((u, it)) = Self::parse_byte(it.clone()) {
            (u, it)
        } else if let Ok((u, it)) = Self::parse_dec(it.clone()) {
            (u, it)
        } else if let Ok((u, it)) = Self::parse_int(it.clone()) {
            (u, it)
        } else if let Ok((u, it)) = Self::parse_none(it.clone()) {
            (u, it)
        } else if let Ok((u, it)) = Self::parse_str(it.clone()) {
            (u, it)
        } else if let Ok((u, it)) = Self::parse_ref(it.clone()) {
            (u, it)
        } else if let Ok((u, it)) = Self::parse_pair(it.clone()) {
            (u, it)
        } else if let Ok((u, it)) = Self::parse_list(it.clone()) {
            (u, it)
        } else if let Ok((u, it)) = Self::parse_map(it.clone()) {
            (u, it)
        } else {
            return Err(UnitParseErr::NotStream)
        };

        while let Ok(tmp) = Unit::parse_ch('@', it.clone()) {
            let (serv, tmp) = Unit::parse_str(tmp)?;
            let serv = serv.as_str().ok_or(UnitParseErr::StreamInvalidServ)?;

            // FIXME: add `addr`
            (u, it) = (Unit::stream_loc(u, &serv), tmp);
        }
        Ok((u, it))
    }

    fn parse_pair(it: I) -> Result<(Unit, I), UnitParseErr> {
        let it = Unit::parse_ch('(', it)?;
        let (u0, it) = Unit::parse(it)?;

        let (cnt, it) = Unit::parse_ws(it)?;
        if cnt < 1 {
            return Err(UnitParseErr::UnexpectedChar);
        }

        let (u1, it) = Unit::parse(it)?;
        let it = Unit::parse_ch(')', it)?;

        Ok((Unit::pair(u0, u1), it))
    }

    fn parse_list(it: I) -> Result<(Unit, I), UnitParseErr> {
        let mut it = Unit::parse_ch('[', it)?;
        let mut lst = Vec::new();

        loop {
            if let Ok(it) = Unit::parse_ch(']', it.clone()) {
                return Ok((Unit::list(&lst), it));
            }

            let (_, tmp) = Unit::parse_ws(it)?;
            let (u, tmp) = Unit::parse(tmp)?;
            let (_, tmp) = Unit::parse_ws(tmp)?;

            lst.push(u);            
            it = tmp;
        }
    }

    fn parse_map(it: I) -> Result<(Unit, I), UnitParseErr> {
        let mut it = Unit::parse_ch('{', it)?;
        let mut map = Vec::new();

        loop {
            if let Ok(it) = Unit::parse_ch('}', it.clone()) {
                return Ok((Unit::map(&map), it))
            }

            let (_, tmp) = Unit::parse_ws(it)?;
            let (u0, tmp) = Unit::parse(tmp)?;
            let (_, tmp) = Unit::parse_ws(tmp)?;

            let tmp = Unit::parse_ch(':', tmp)?;

            let (_, tmp) = Unit::parse_ws(tmp)?;
            let (u1, tmp) = Unit::parse(tmp)?;
            let (_, tmp) = Unit::parse_ws(tmp)?;

            map.push((u0, u1));
            it = tmp;
        }
    }
}

impl UnitModify for Unit {
    fn find<'a, I>(&self, mut path: I) -> Option<Unit> where I: Iterator<Item = &'a str> + Clone {
        let step = if let Some(step) = path.next() {
            step
        } else {
            return Some(self.clone());
        };

        match self.0.as_ref() {
            UnitType::Pair(u0, u1) =>
                match step.parse::<usize>().ok()? {
                    0 => u0.find(path),
                    1 => u1.find(path),
                    _ => None
                },
            UnitType::List(lst) => {
                let idx = step.parse::<usize>().ok()?;
                lst.get(idx).map(|u| u.find(path)).flatten()
            },
            UnitType::Map(map) => map.iter()
                .filter_map(|(u0, u1)| Some((u0.clone().as_str()?, u1.clone())))
                .find_map(|(s, u)| {
                    if s.as_str() == step {
                        return u.find(path.clone())
                    }
                    None
                }),
            _ => None
        }
    }

    fn replace<'a, I>(self, mut path: I, what: Unit) -> Option<Unit> where I: Iterator<Item = &'a str> + Clone {
        let step = if let Some(step) = path.next() {
            step
        } else {
            return Some(what);
        };

        match self.0.as_ref() {
            UnitType::Pair(u0, u1) =>
                match step.parse::<usize>().ok()? {
                    0 => Some(Unit::pair(u0.clone().replace(path, what)?, u1.clone())),
                    1 => Some(Unit::pair(u0.clone(), u1.clone().replace(path, what)?)),
                    _ => None
                },
            UnitType::List(lst) => {
                let idx = step.parse::<usize>().ok()?;
                if idx >= lst.len() {
                    return None;
                }

                let lst = lst.iter().cloned().enumerate().map(|(i, u)| {
                    if i == idx {
                        return u.clone().replace(path.clone(), what.clone())
                    }
                    Some(u)
                }).collect::<Option<Vec<_>>>()?;
                Some(Unit::list(&lst))
            },
            UnitType::Map(map) => {
                if let None = map.iter().filter_map(|(u0, _)| u0.clone().as_str()).find(|s| Rc::unwrap_or_clone(s.clone()) == step) {
                    return None
                }

                let map = map.iter().cloned().map(|(u0, u1)| {
                    if let Some(_) = u0.clone().as_str().filter(|s| Rc::unwrap_or_clone(s.clone()) == step) {
                        return Some((
                            u0.clone(),
                            u1.clone().replace(path.clone(), what.clone())?
                        ))
                    }
                    Some((u0, u1))
                }).collect::<Option<Vec<_>>>()?;
                Some(Unit::map(&map))
            }
            _ => None
        }
    }

    fn merge<'a, I>(self, path: I, what: Unit) -> Option<Unit> where I: Iterator<Item = &'a str> + Clone {
        // FIXME: make it work
        let what = if let Some(u) = self.find(path.clone()) {
            u.merge_with(what)
        } else {
            what
        };
        self.replace(path, what)
    }

    fn merge_with(self, what: Unit) -> Unit {
        if self == what {
            return self;
        }

        match self.0.as_ref() {
            UnitType::List(lst) => {
                let lst = match what.0.as_ref() {
                    UnitType::List(w_lst) => {
                        let mut lst = Rc::unwrap_or_clone(lst.clone());
                        lst.extend(Rc::unwrap_or_clone(w_lst.clone()));
                        lst
                    },
                    _ => {
                        let mut lst = Rc::unwrap_or_clone(lst.clone());
                        lst.push(what);
                        lst
                    }
                };
                Unit::list(&lst)
            },
            UnitType::Map(map) => {
                let map = match what.0.as_ref() {
                    UnitType::Pair(u0, u1) => {
                        let mut map = Rc::unwrap_or_clone(map.clone());
                        map.push((u0.clone(), u1.clone()));
                        map
                    },
                    UnitType::Map(w_map) => {
                        let mut w_map = Rc::unwrap_or_clone(w_map.clone());
                        let mut map = Rc::unwrap_or_clone(map.clone()).into_iter()
                            .map(|(u0, u1)| {
                                if let Some((_, u)) = w_map.drain_filter(|(u00, _)| u00.clone() == u0.clone()).next() {
                                    return (u0.clone(), u1.merge_with(u))
                                }
                                (u0, u1)
                            }).collect::<Vec<_>>();

                        map.append(&mut w_map);
                        map
                    },
                    _ => return what
                };
                Unit::map(&map)
            },
            _ => what
        }
    }
}

impl Iterator for UnitParseBytesIter {
    type Item = Unit;

    fn next(&mut self) -> Option<Self::Item> {
        let it = self.dat.get(self.pos..)?.iter();
        let (u, new_it) = Unit::parse(it.clone()).ok()?;

        self.pos += unsafe{
            new_it.as_slice().as_ptr().offset_from(it.as_slice().as_ptr()) as usize
        };
        return Some(u);
    }
}

impl UnitParseBytesIter {
    pub fn new(dat: Vec<u8>) -> Self {
        UnitParseBytesIter {dat, pos: 0}
    }
}

impl Int {
    pub fn to_small(&self) -> Option<i32> {
        self.0.to_i32()
    }

    pub fn to_nat(&self) -> Option<u32> {
        self.0.to_u32()
    }
}

impl Dec {
    pub fn to_small(&self) -> Option<f32> {
        self.0.to_f32()
    }
}

impl Unit {
    fn new(t: UnitType) -> Unit {
        Unit(Rc::new(t))
    }

    pub fn as_ptr(&self) -> *const UnitType {
        Rc::as_ptr(&self.0)
    }

    pub fn size(&self, units: MemSizeUnits) -> usize {
        let size = core::mem::size_of::<UnitType>() + match self.0.as_ref() {
            UnitType::None | UnitType::Bool(..) | UnitType::Byte(..) => 0,
            UnitType::Int(v) => v.0.to_bytes_le().1.len(),
            UnitType::Dec(v) => v.0.numer().to_bytes_le().1.len() + v.0.denom().to_bytes_le().1.len(),
            UnitType::Str(s) => s.len(),
            UnitType::Ref(path) => path.iter().fold(0, |prev, s| prev + s.len()),
            UnitType::Stream(msg, serv, _addr) => msg.size(MemSizeUnits::Bytes) + serv.len(),
            UnitType::Pair(u0, u1) => u0.size(MemSizeUnits::Bytes) + u1.size(MemSizeUnits::Bytes),
            UnitType::List(lst) => lst.iter().fold(0, |prev, u| prev + u.size(MemSizeUnits::Bytes)),
            UnitType::Map(map) => map.iter().fold(0, |prev, (u0, u1)| prev + u0.size(MemSizeUnits::Bytes) + u1.size(MemSizeUnits::Bytes))
        };

        match units {
            MemSizeUnits::Bytes => size,
            MemSizeUnits::Kilo => size / 1024,
            MemSizeUnits::Mega => size / (1024 * 1024),
            MemSizeUnits::Giga => size / (1024 * 1024 * 1024)
        }
    }
}

impl DisplayShort {
    fn shrt(&self, s: String) -> String {
        if s.len() > self.0 {
            format!("{}..", s.chars().take(self.0).collect::<String>())
        } else {
            s
        }
    }
}
