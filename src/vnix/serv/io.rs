use core::ops::Deref;

use crate::driver::CLIErr;

use crate::vnix::core::msg::Msg;
use crate::vnix::core::unit::Unit;

use crate::vnix::core::serv::Serv;
use crate::vnix::core::kern::{KernErr, Kern};


pub struct Term {
    full: bool,
    nl: bool
}

impl Default for Term {
    fn default() -> Self {
        Term {
            full: false,
            nl: true
        }
    }
}

impl Serv for Term {
    fn inst(msg: Msg, _kern: &mut Kern) -> Result<(Self, Msg), KernErr> {
        // config instance
        if let Unit::Map(m) = msg.msg.deref() {
            let mut inst = Term::default();

            m.iter().for_each(|(u0, u1)| {
                if let Unit::Str(s) = u0.deref() {
                    if s == "full" {
                        if let Unit::Bool(v) = u1.deref() {
                            inst.full = *v;
                        }
                    }

                    if s == "nl" {
                        if let Unit::Bool(v) = u1.deref() {
                            inst.nl = *v;
                        }
                    }
                }
            });

            return Ok((inst, msg));
        }

        // default
        Ok((Term::default(), msg))
    }

    fn handle(&self, msg: Msg, kern: &mut Kern) -> Result<Option<Msg>, KernErr> {
        if self.full {
            writeln!(kern.cli, "INFO vnix:io.term: {}", msg).map_err(|_| KernErr::CLIErr(CLIErr::Write))?;
        } else {
            if self.nl {
                writeln!(kern.cli, "{}", msg.msg).map_err(|_| KernErr::CLIErr(CLIErr::Write))?;
            } else {
                write!(kern.cli, "{}", msg.msg).map_err(|_| KernErr::CLIErr(CLIErr::Write))?;
            }
        }

        Ok(None)
    }
}
