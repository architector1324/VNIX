use alloc::rc::Rc;
use alloc::vec::Vec;
use alloc::string::String;

use num::bigint::BigInt;
use num::rational::BigRational;

use crate::driver::MemSizeUnits;

use super::kern::Addr;


pub enum Int {
    Small(i32),
    Nat(u32),
    Big(BigInt)
}

pub enum Dec {
    Small(f32),
    Big(BigRational)
}

pub type Path = Vec<String>;

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

pub struct Unit(Rc<UnitType>);


impl Unit {
    pub fn new(t: UnitType) -> Unit {
        Unit(Rc::new(t))
    }

    pub fn new_none() -> Unit {
        Unit::new(UnitType::None)
    }

    pub fn size(&self, units: MemSizeUnits) -> usize {
        match self.0.as_ref() {
            UnitType::None | UnitType::Bool(..) | UnitType::Byte(..) => core::mem::size_of::<UnitType>(),
            _ => todo!()
        }
    }
}
