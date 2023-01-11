use core::ops::Deref;
use core::fmt::Write;

use heapless::String;

use crate::driver::{CLIErr, TermKey};

use crate::vnix::core::msg::Msg;
use crate::vnix::core::unit::Unit;

use crate::vnix::core::serv::Serv;
use crate::vnix::core::kern::{KernErr, Kern};


struct GFXMng {
    fill: Option<u32>
}

struct Inp {
    pmt: String<256>
}

pub struct Term {
    gfx: Option<GFXMng>,
    inp: Option<Inp>,
    nl: bool,
    msg: Option<String<256>>,
    trc: bool,
    prs: bool
}

impl Default for Term {
    fn default() -> Self {
        Term {
            gfx: None,
            inp: None,
            nl: true,
            msg: None,
            trc: false,
            prs: false
        }
    }
}

impl Term {
    fn gfx_hlr(&self, kern: &mut Kern) -> Result<Option<Msg>, KernErr> {
        if let Some(ref gfx) = self.gfx {
            if let Some(fill) = gfx.fill {
                kern.disp.fill(&|_, _| {
                    fill
                }).map_err(|e| KernErr::DispErr(e))?;
            }
        }

        Ok(None)
    }

    fn cli_hlr(&self, msg: Msg, kern: &mut Kern) -> Result<Option<Msg>, KernErr> {
        if let Some(ref s) = self.msg {
            if self.nl {
                writeln!(kern.cli, "{}", s).map_err(|_| KernErr::CLIErr(CLIErr::Write))?;
            } else {
                write!(kern.cli, "{}", s).map_err(|_| KernErr::CLIErr(CLIErr::Write))?;
            }
        } else if self.inp.is_none() {
            if self.nl {
                writeln!(kern.cli, "{}", msg.msg).map_err(|_| KernErr::CLIErr(CLIErr::Write))?;
            } else {
                write!(kern.cli, "{}", msg.msg).map_err(|_| KernErr::CLIErr(CLIErr::Write))?;
            }
        }

        if let Some(inp) = &self.inp {
            let mut out = String::<256>::new();

            write!(kern.cli, "\r{}", inp.pmt).map_err(|_| KernErr::CLIErr(CLIErr::Write))?;

            loop {
                if let Some(key) = kern.cli.get_key().map_err(|e| KernErr::CLIErr(e))? {
                    if let TermKey::Char(c) = key {
                        if c == '\r' || c == '\n' {
                            writeln!(kern.cli).map_err(|_| KernErr::CLIErr(CLIErr::Write))?;
                            break;
                        }

                        if c == '\u{8}' {
                            out.pop();
                        } else {
                            write!(out, "{}", c).map_err(|_| KernErr::CLIErr(CLIErr::Write))?;
                        }

                        write!(kern.cli, "\r{}{out} ", inp.pmt).map_err(|_| KernErr::CLIErr(CLIErr::Write))?;
                    }
                }
            }

            if !out.is_empty() {
                let u = if self.prs {
                    let tmp = Unit::parse(out.chars(), kern)?.0;
                    kern.unit(tmp)?
                } else {
                    kern.unit(Unit::Str(out))?
                };
    
                return Ok(Some(kern.msg(&msg.ath.name, u)?))
            }
        }

        Ok(None)
    }
}

impl Serv for Term {
    fn inst(msg: Msg, _kern: &mut Kern) -> Result<(Self, Msg), KernErr> {
        let mut inst = Term::default();

        // config instance
        if let Unit::Map(m) = msg.msg.deref() {
            let mut it = m.iter().filter_map(|p| Some((p.0.deref().as_str()?, p.1.deref())));
            let mut bool_it = it.clone().filter_map(|(s, u)| Some((s, u.as_bool()?)));

            bool_it.clone().find(|(s, _)| s == "trc").map(|(_, v)| inst.trc = v);
            bool_it.clone().find(|(s, _)| s == "nl").map(|(_, v)| inst.nl = v);
            bool_it.find(|(s, _)| s == "prs").map(|(_, v)| inst.prs = v);

            it.clone().filter_map(|(s, u)| Some((s, u.as_str()?))).find(|(s, _)| s == "inp").map(|(_, s)| {
                inst.inp = Some(Inp {
                    pmt: s
                })
            });

            it.clone().filter_map(|(s, u)| Some((s, u.as_int()?))).find(|(s, _)| s == "fill").map(|(_, v)| {
                inst.gfx = Some(GFXMng {
                    fill: Some(v as u32)
                });
            });

            let msg = it.find(|(s, _)| s == "msg").map(|(_, u)| u);

            if let Some(u) = msg {
                let mut s = String::<256>::new();
                write!(s, "{}", u).map_err(|_| KernErr::CLIErr(CLIErr::Write))?;

                inst.msg = Some(s);
            }
        }

        Ok((inst, msg))
    }

    fn handle(&self, msg: Msg, kern: &mut Kern) -> Result<Option<Msg>, KernErr> {
        if self.trc {
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
            if let Some(msg) = self.cli_hlr(msg, kern)? {
                return Ok(Some(msg));
            }
        }

        Ok(None)
    }
}
