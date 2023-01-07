use heapless::pool::{Pool, Box};
use heapless::{Vec, LinearMap};
use core::ops::Deref;

use super::{super::driver::CLI, super::driver::CLIErr, unit::Unit};

#[derive(Debug)]
pub enum KernErr {
    MemoryOut,
    CLIErr(CLIErr)
}

pub struct Kern<'a> {
    // drivers
    pub cli: &'a mut dyn CLI,

    // vnix
    units: Pool<Unit>
}


impl<'a> Kern<'a> {
    pub fn new(cli: &'a mut dyn CLI) -> Self {
        let kern = Kern {
            cli,
            units: Pool::new()
        };

        static mut UNITS_MEM: [u8; 256 * core::mem::size_of::<Unit>()] = [0; 256 * core::mem::size_of::<Unit>()];

        unsafe {
            kern.units.grow(&mut UNITS_MEM);
        }

        kern
    }

    pub fn unit(&mut self, u: Unit) -> Result<Box<Unit>, KernErr> {
        if let Some(b) = self.units.alloc() {
           return Ok(b.init(u));
        }
        Err(KernErr::MemoryOut)
    }

    pub fn dup(&mut self, u: &Box<Unit>) -> Result<Box<Unit>, KernErr> {
        if let Some(b) = self.units.alloc() {
            let n_u = match u.deref() {
                Unit::None => Unit::None,
                Unit::Bool(v) => Unit::Bool(*v),
                Unit::Byte(v) => Unit::Byte(*v),
                Unit::Int(v) => Unit::Int(*v),
                Unit::Dec(v) => Unit::Dec(*v),
                Unit::Str(s) => Unit::Str(s.as_str().into()),
                Unit::Pair(p) => Unit::Pair((self.dup(&p.0)?, self.dup(&p.1)?)),
                Unit::Lst(lst) => {
                    let mut n_lst = Vec::new();

                    for u in lst {
                        n_lst.push(self.dup(u)?).map_err(|_| KernErr::MemoryOut)?;
                    }

                    Unit::Lst(n_lst)
                },
                Unit::Map(m) => {
                    let mut n_map = LinearMap::new();

                    for (u0, u1) in m {
                        n_map.insert(self.dup(u0)?, self.dup(u1)?).map_err(|_| KernErr::MemoryOut)?;
                    }
                    Unit::Map(n_map)
                }
            };
            return Ok(b.init(n_u));
        }
        Err(KernErr::MemoryOut)
    }
}
