use core::ops::Deref;
use core::fmt::Write;

use heapless::String;

use crate::driver::{CLIErr, DispErr, TermKey};

use crate::vnix::core::msg::Msg;
use crate::vnix::core::unit::Unit;

use crate::vnix::core::serv::Serv;
use crate::vnix::core::kern::{KernErr, Kern};

struct GFXMng {
    fill: Option<u32>
}

pub struct Term {
    gfx: Option<GFXMng>,
    full: bool,
    nl: bool,
    msg: String<256>
}

impl Default for Term {
    fn default() -> Self {
        Term {
            gfx: None,
            full: false,
            nl: true,
            msg: String::new()
        }
    }
}

impl Term {
    fn gfx_hlr(&self, kern: &mut Kern) -> Result<Option<Msg>, KernErr> {
        if let Some(ref gfx) = self.gfx {
            if let Some(fill) = gfx.fill {
                let res = kern.cli.res().map_err(|e| KernErr::DispErr(e))?;
    
                for x in 0..res.0 {
                    for y in 0..res.1 {
                        kern.cli.px(fill, x, y).map_err(|e| KernErr::DispErr(e))?;
                    }
                }
            }
        }

        Ok(None)
    }

    fn cli_hlr(&self, kern: &mut Kern) -> Result<Option<Msg>, KernErr> {
        if self.nl {
            writeln!(kern.cli, "{}", self.msg).map_err(|_| KernErr::CLIErr(CLIErr::Write))?;
        } else {
            write!(kern.cli, "{}", self.msg).map_err(|_| KernErr::CLIErr(CLIErr::Write))?;
        }

        Ok(None)
    }
}

impl Serv for Term {
    fn inst(msg: Msg, _kern: &mut Kern) -> Result<(Self, Msg), KernErr> {
        let mut inst = Term::default();
    
        // config instance
        if let Unit::Map(m) = msg.msg.deref() {

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

                    if s == "fill" {
                        if let Unit::Int(v) = u1.deref() {
                            inst.gfx = Some(GFXMng {
                                fill: Some(*v as u32)
                            });
                        }
                    }
                }
            });
        }

        write!(inst.msg, "{}", msg.msg).map_err(|_| KernErr::CLIErr(CLIErr::Write))?;

        // default
        Ok((inst, msg))
    }

    fn handle(&self, msg: Msg, kern: &mut Kern) -> Result<Option<Msg>, KernErr> {
        if self.full {
            writeln!(kern.cli, "INFO vnix:io.term: {}", msg).map_err(|_| KernErr::CLIErr(CLIErr::Write))?;
        } else {
            // gfx
            if self.gfx.is_some() {
                if let Some(msg) = self.gfx_hlr(kern)? {
                    return Ok(Some(msg));
                }

                // wait for key
                kern.cli.get_key().map_err(|e| KernErr::CLIErr(e))?;
                kern.cli.clear().map_err(|_| KernErr::CLIErr(CLIErr::Clear))?;
            }

            // cli
            if let Some(msg) = self.cli_hlr(kern)? {
                return Ok(Some(msg));
            }
        }

        Ok(None)
    }
}
