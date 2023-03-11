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
    Big(BigInt)
}

#[derive(Debug, PartialEq, Clone)]
pub enum Dec {
    Small(f32),
    Big(BigRational)
}

pub type Path = Vec<String>;

#[derive(Debug, PartialEq, Clone)]
pub enum UnitType {
    None,
    Bool(bool),
    Byte(u8),
    Int(Int),
    Dec(Dec),
    Str(String),
    Ref(Path),
    Stream(Unit, String, Addr),
    Pair(Unit, Unit),
    List(Vec<Unit>),
    Map(Vec<(Unit, Unit)>)
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
        Unit::new(UnitType::Int(Int::Big(v)))
    }

    fn dec(v: f32) -> Unit {
        Unit::new(UnitType::Dec(Dec::Small(v)))
    }

    fn dec_big(v: BigRational) -> Unit {
        Unit::new(UnitType::Dec(Dec::Big(v)))
    }

    fn str(s: &str) -> Unit {
        Unit::new(UnitType::Str(s.into()))
    }

    fn path(path: &[&str]) -> Unit {
        Unit::new(UnitType::Ref(path.into_iter().cloned().map(|s| format!("{s}")).collect()))
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
        Unit::new(UnitType::List(lst.to_vec()))
    }

    fn map(map: &[(Unit, Unit)]) -> Unit {
        Unit::new(UnitType::Map(map.to_vec()))
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
