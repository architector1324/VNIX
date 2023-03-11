use alloc::format;
use alloc::rc::Rc;
use alloc::vec::Vec;
use alloc::string::String;

use num::bigint::BigInt;
use num::rational::BigRational;

use crate::driver::MemSizeUnits;

use super::kern::Addr;


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
}

impl Unit {
    fn new(t: UnitType) -> Unit {
        Unit(Rc::new(t))
    }

    pub fn size(&self, units: MemSizeUnits) -> usize {
        match self.0.as_ref() {
            UnitType::None | UnitType::Bool(..) | UnitType::Byte(..) => core::mem::size_of::<UnitType>(),
            _ => todo!()
        }
    }
}
