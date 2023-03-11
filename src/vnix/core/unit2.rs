use alloc::{format, vec};
use alloc::rc::Rc;
use alloc::vec::Vec;
use alloc::boxed::Box;
use alloc::string::String;

use core::pin::Pin;
use core::slice::Iter;
use core::fmt::Display;
use core::ops::{Generator, GeneratorState};

use num::bigint::{BigInt, Sign};
use num::rational::BigRational;
use spin::Mutex;

use crate::driver::MemSizeUnits;
use crate::{thread, thread_await};

use super::kern::{Addr, KernErr, Kern};
use super::task::ThreadAsync;


#[derive(Debug, PartialEq, Clone)]
pub enum Int {
    Small(i32),
    Nat(u32),
    Big(Rc<BigInt>)
}

#[derive(Debug, PartialEq, Clone)]
pub enum Dec {
    Small(f32),
    Big(Rc<BigRational>)
}

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
    Stream(Unit, String, Addr),
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
    RefInvalidPath,
    InvalidSign,
    InvalidAddr,
}

pub trait UnitNew {
    fn none() -> Unit;
    fn bool(v: bool) -> Unit;
    fn byte(v: u8) -> Unit;
    fn int(v: i32) -> Unit;
    fn uint(v: u32) -> Unit;
    fn int_big(v: BigInt) -> Unit;
    fn dec(v: f32) -> Unit;
    fn dec_big(v: BigRational) -> Unit;
    fn str(s: &str) -> Unit;
    fn path(path: &[&str]) -> Unit;
    fn stream_loc(u: Unit, serv: &str) -> Unit;
    fn stream(u: Unit, serv: &str, addr: Addr) -> Unit;
    fn pair(u0: Unit, u1: Unit) -> Unit;
    fn list(lst: &[Unit]) -> Unit;
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

pub trait UnitParse<'a, T: 'a, I> where I: Iterator<Item = &'a T> + Clone {
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

}

pub type UnitReadAsync<'a> = ThreadAsync<'a, Result<Option<(Unit, Rc<String>)>, KernErr>>;

pub trait UnitReadAsyncI {
    fn read_async<'a>(self, ath: Rc<String>, orig: Unit, kern: &'a Mutex<Kern>) -> UnitReadAsync<'a>;
    fn as_map_find_async<'a>(self, sch: String, ath: Rc<String>, orig: Unit, kern: &'a Mutex<Kern>) -> UnitReadAsync<'a>;
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
        Unit::new(UnitType::Int(Int::Small(v)))
    }

    fn uint(v: u32) -> Unit {
        Unit::new(UnitType::Int(Int::Nat(v)))
    }

    fn int_big(v: BigInt) -> Unit {
        Unit::new(UnitType::Int(Int::Big(Rc::new(v))))
    }

    fn dec(v: f32) -> Unit {
        Unit::new(UnitType::Dec(Dec::Small(v)))
    }

    fn dec_big(v: BigRational) -> Unit {
        Unit::new(UnitType::Dec(Dec::Big(Rc::new(v))))
    }

    fn str(s: &str) -> Unit {
        Unit::new(UnitType::Str(Rc::new(s.into())))
    }

    fn path(path: &[&str]) -> Unit {
        Unit::new(UnitType::Ref(Rc::new(path.into_iter().cloned().map(|s| format!("{s}")).collect())))
    }

    fn stream_loc(u: Unit, serv: &str) -> Unit {
        Unit::new(UnitType::Stream(u, serv.into(), Addr::Local))
    }

    fn stream(u: Unit, serv: &str, addr: Addr) -> Unit {
        Unit::new(UnitType::Stream(u, serv.into(), addr))
    }

    fn pair(u0: Unit, u1: Unit) -> Unit {
        Unit::new(UnitType::Pair(u0, u1))
    }

    fn list(lst: &[Unit]) -> Unit {
        Unit::new(UnitType::List(Rc::new(lst.to_vec())))
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
            if let Int::Small(v) = v {
                return Some(*v)
            }
        }
        None
    }

    fn as_uint(self) -> Option<u32> {
        if let UnitType::Int(v) = self.0.as_ref() {
            if let Int::Nat(v) = v {
                return Some(*v)
            }
        }
        None
    }

    fn as_int_big(self) -> Option<Rc<BigInt>> {
        if let UnitType::Int(v) = self.0.as_ref() {
            if let Int::Big(v) = v {
                return Some(v.clone())
            }
        }
        None
    }

    fn as_dec(self) -> Option<f32> {
        if let UnitType::Dec(v) = self.0.as_ref() {
            if let Dec::Small(v) = v {
                return Some(*v)
            }
        }
        None
    }

    fn as_dec_big(self) -> Option<Rc<BigRational>> {
        if let UnitType::Dec(v) = self.0.as_ref() {
            if let Dec::Big(v) = v {
                return Some(v.clone())
            }
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
            return Some((u.clone(), serv.clone(), addr.clone()))
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
                    if Rc::unwrap_or_clone(s) == sch {
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
                UnitType::Ref(path) => {
                    yield;
                    todo!()
                },
                UnitType::Stream(msg, serv, _addr) => {
                    todo!()
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

impl Display for Unit {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self.0.as_ref() {
            UnitType::None => write!(f, "-"),
            UnitType::Bool(v) => write!(f, "{}", if *v {"t"} else {"f"}),
            UnitType::Byte(v) => write!(f, "{:#02x}", *v),
            UnitType::Int(v) =>
                match v {
                    Int::Small(v) => write!(f, "{v}"),
                    Int::Nat(v) => write!(f, "{v}"),
                    Int::Big(v) => write!(f, "{v}")
                }
            UnitType::Dec(v) =>
                match v {
                    Dec::Small(v) => write!(f, "{v}"),
                    Dec::Big(v) => write!(f, "{v}") // FIXME: use `<i>.<i>` format
                }
            UnitType::Str(s) => {
                if s.as_str().chars().all(|c| c.is_alphanumeric() || c == '.' || c == '#' || c == '_') {
                    write!(f, "{s}")
                } else {
                    write!(f, "`{s}`")
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

impl UnitAsBytes for Unit {
    fn as_bytes(self) -> Vec<u8> {
        match self.0.as_ref() {
            UnitType::None => vec![UnitBin::None as u8],
            UnitType::Bool(v) => vec![UnitBin::Bool as u8, if *v {1} else {0}],
            UnitType::Byte(v) => vec![UnitBin::Byte as u8, *v],
            UnitType::Int(v) =>
                match v {
                    Int::Small(v) => [UnitBin::Int as u8].into_iter().chain(v.to_le_bytes()).collect(),
                    Int::Nat(v) => [UnitBin::IntNat as u8].into_iter().chain(v.to_le_bytes()).collect(),
                    Int::Big(v) => {
                        let (s, b) = v.to_bytes_le();
                        let len = (b.len() as u32).to_le_bytes();
                        [UnitBin::IntBig as u8].into_iter()
                            .chain([if let Sign::Minus = s {1} else {0}])
                            .chain(len)
                            .chain(b)
                            .collect()
                    }
                },
            UnitType::Dec(v) =>
                match v {
                    Dec::Small(v) => [UnitBin::Dec as u8].into_iter().chain(v.to_le_bytes()).collect(),
                    Dec::Big(v) => {
                        let (s, b0) = v.numer().to_bytes_le();
                        let len0 = (b0.len() as u32).to_le_bytes();

                        let (_, b1) = v.denom().to_bytes_le();
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
            UnitType::Str(s) => [UnitBin::Str as u8].into_iter()
                .chain((s.len() as u32).to_le_bytes())
                .chain(s.as_bytes().into_iter().cloned())
                .collect(),
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
                .chain(match addr {
                    Addr::Local => vec![UnitBin::AddrLoc as u8],
                    Addr::Remote(addr) => [UnitBin::AddrRemote as u8].into_iter().chain(addr.into_iter().flat_map(|e| e.to_le_bytes())).collect::<Vec<u8>>()
                }).collect(),
            UnitType::Pair(u0, u1) => [UnitBin::Pair as u8].into_iter()
                .chain(u0.clone().as_bytes())
                .chain(u1.clone().as_bytes())
                .collect(),
            UnitType::List(lst) => [UnitBin::List as u8].into_iter()
                .chain((lst.len() as u32).to_le_bytes())
                .chain(lst.iter().flat_map(|u| u.clone().as_bytes()))
                .collect(),
            UnitType::Map(map) => [UnitBin::Map as u8].into_iter()
                .chain((map.len() as u32).to_le_bytes())
                .chain(
                    map.iter().flat_map(|(u0, u1)| u0.clone().as_bytes().into_iter().chain(u1.clone().as_bytes()).collect::<Vec<u8>>())
                )
                .collect()
        }
    }
}

impl<'a> UnitParse<'a, u8, Iter<'a, u8>> for Unit {
    fn parse(it: Iter<'a, u8>) -> Result<(Unit, Iter<'a, u8>), UnitParseErr> {
        match *it.clone().next().ok_or(UnitParseErr::UnexpectedEnd)? {
            _b if _b == UnitBin::None as u8 => Self::parse_none(it),
            _b if _b == UnitBin::Bool as u8 => Self::parse_bool(it),
            _b if _b == UnitBin::Byte as u8 => Self::parse_byte(it),
            _b if _b == UnitBin::Int as u8 => Self::parse_int(it),
            _b if _b == UnitBin::IntNat as u8 => Self::parse_int(it),
            _b if _b == UnitBin::IntBig as u8 => Self::parse_int(it),
            _b if _b == UnitBin::Dec as u8 => Self::parse_dec(it),
            _b if _b == UnitBin::DecBig as u8 => Self::parse_dec(it),
            _b if _b == UnitBin::Str as u8 => Self::parse_str(it),
            _b if _b == UnitBin::Ref as u8 => Self::parse_ref(it),
            _b if _b == UnitBin::Pair as u8 => Self::parse_pair(it),
            _b if _b == UnitBin::List as u8 => Self::parse_list(it),
            _b if _b == UnitBin::Map as u8 => Self::parse_map(it),
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
        if b != UnitBin::Str as u8 {
            return Err(UnitParseErr::NotStr)
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
        
        if !s.chars().all(|c| c.is_alphanumeric() || c == '#' || c == '_' || c == '.') {
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
        if b != UnitBin::Pair as u8 {
            return Err(UnitParseErr::NotPair)
        }

        let (u0, it) = Unit::parse(it)?;
        let (u1, it) = Unit::parse(it)?;

        Ok((Unit::pair(u0, u1), it))
    }

    fn parse_list(mut it: Iter<'a, u8>) -> Result<(Unit, Iter<'a, u8>), UnitParseErr> {
        let b = *it.next().ok_or(UnitParseErr::UnexpectedEnd)?;
        if b != UnitBin::List as u8 {
            return Err(UnitParseErr::NotList);
        }

        let bytes = [
            *it.next().ok_or(UnitParseErr::UnexpectedEnd)?,
            *it.next().ok_or(UnitParseErr::UnexpectedEnd)?,
            *it.next().ok_or(UnitParseErr::UnexpectedEnd)?,
            *it.next().ok_or(UnitParseErr::UnexpectedEnd)?
        ];
        let len = <u32>::from_le_bytes(bytes);

        let mut lst = Vec::with_capacity(len as usize);
        for _ in 0..len {
            let (u, next) = Unit::parse(it)?;
            lst.push(u);
            it = next;
        }
        Ok((Unit::list(&lst), it))
    }

    fn parse_map(mut it: Iter<'a, u8>) -> Result<(Unit, Iter<'a, u8>), UnitParseErr> {
        let b = *it.next().ok_or(UnitParseErr::UnexpectedEnd)?;
        if b != UnitBin::Map as u8 {
            return Err(UnitParseErr::NotMap);
        }

        let bytes = [
            *it.next().ok_or(UnitParseErr::UnexpectedEnd)?,
            *it.next().ok_or(UnitParseErr::UnexpectedEnd)?,
            *it.next().ok_or(UnitParseErr::UnexpectedEnd)?,
            *it.next().ok_or(UnitParseErr::UnexpectedEnd)?
        ];
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

// impl<'a> UnitParse<'a, char, Iter<'a, char>> for Unit {
//     fn parse(it: Iter<'a, char>) -> Result<(Unit, Iter<'a, char>), UnitParseErr> {
//         todo!()
//     }

//     fn parse_none(it: Iter<'a, char>) -> Result<(Unit, Iter<'a, char>), UnitParseErr> {
//         todo!()
//     }

//     fn parse_bool(it: Iter<'a, char>) -> Result<(Unit, Iter<'a, char>), UnitParseErr> {
//         todo!()
//     }

//     fn parse_byte(it: Iter<'a, char>) -> Result<(Unit, Iter<'a, char>), UnitParseErr> {
//         todo!()
//     }

//     fn parse_int(it: Iter<'a, char>) -> Result<(Unit, Iter<'a, char>), UnitParseErr> {
//         todo!()
//     }

//     fn parse_dec(it: Iter<'a, char>) -> Result<(Unit, Iter<'a, char>), UnitParseErr> {
//         todo!()
//     }

//     fn parse_str(it: Iter<'a, char>) -> Result<(Unit, Iter<'a, char>), UnitParseErr> {
//         todo!()
//     }

//     fn parse_ref(it: Iter<'a, char>) -> Result<(Unit, Iter<'a, char>), UnitParseErr> {
//         todo!()
//     }

//     fn parse_stream(it: Iter<'a, char>) -> Result<(Unit, Iter<'a, char>), UnitParseErr> {
//         todo!()
//     }

//     fn parse_pair(it: Iter<'a, char>) -> Result<(Unit, Iter<'a, char>), UnitParseErr> {
//         todo!()
//     }

//     fn parse_list(it: Iter<'a, char>) -> Result<(Unit, Iter<'a, char>), UnitParseErr> {
//         todo!()
//     }

//     fn parse_map(it: Iter<'a, char>) -> Result<(Unit, Iter<'a, char>), UnitParseErr> {
//         todo!()
//     }
// }

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
            UnitType::Int(v) =>
                match v {
                    Int::Small(..) | Int::Nat(..) => 0,
                    Int::Big(v) => v.to_bytes_le().1.len(),
                },
            UnitType::Dec(v) =>
                match v {
                    Dec::Small(..) => 0,
                    Dec::Big(v) => v.numer().to_bytes_le().1.len() + v.denom().to_bytes_le().1.len()
                },
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
