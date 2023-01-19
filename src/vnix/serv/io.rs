use core::fmt::Write;

use alloc::{format, vec};
use alloc::string::String;
use alloc::vec::Vec;

use base64ct::{Base64, Encoding};
use compression::prelude::{GZipDecoder, DecodeExt};

use crate::driver::{CLIErr, TermKey};

use crate::vnix::core::msg::Msg;
use crate::vnix::core::unit::{Unit, UnitParseErr};

use crate::vnix::core::serv::{Serv, ServHlr};
use crate::vnix::core::kern::KernErr;


struct Inp {
    pmt: String
}

struct Img {
    img: Vec<u32>
}

pub struct Term {
    inp: Option<Inp>,
    img: Option<Img>,
    nl: bool,
    cls: bool,
    msg: Option<String>,
    trc: bool,
    prs: bool
}

impl Default for Term {
    fn default() -> Self {
        Term {
            inp: None,
            img: None,
            nl: true,
            cls: false,
            msg: None,
            trc: false,
            prs: false
        }
    }
}

impl Term {
    fn img_hlr(&self, serv: &mut Serv) -> Result<(), KernErr> {
        if let Some(ref img) = self.img {
            let (w, _) = serv.kern.disp.res().map_err(|e| KernErr::DispErr(e))?;

            serv.kern.disp.fill(&|x, y| {
                if let Some(px) = img.img.get(x + w * y) {
                    *px
                } else {
                    0
                }
            }).map_err(|e| KernErr::DispErr(e))?;
        }

        Ok(())
    }

    fn cli_hlr(&self, msg: Msg, serv: &mut Serv) -> Result<Option<Msg>, KernErr> {
        if self.cls {
            serv.kern.cli.clear().map_err(|_| KernErr::CLIErr(CLIErr::Clear))?;
        }

        if let Some(ref s) = self.msg {
            if self.nl {
                writeln!(serv.kern.cli, "{}", s).map_err(|_| KernErr::CLIErr(CLIErr::Write))?;
            } else {
                write!(serv.kern.cli, "{}", s).map_err(|_| KernErr::CLIErr(CLIErr::Write))?;
            }
        } else if self.inp.is_none() && !self.cls {
            if self.nl {
                writeln!(serv.kern.cli, "{}", msg.msg).map_err(|_| KernErr::CLIErr(CLIErr::Write))?;
            } else {
                write!(serv.kern.cli, "{}", msg.msg).map_err(|_| KernErr::CLIErr(CLIErr::Write))?;
            }
        }

        if let Some(inp) = &self.inp {
            let mut out = String::new();

            if inp.pmt == "key" {
                if let Some(key) = serv.kern.cli.get_key(true).map_err(|e| KernErr::CLIErr(e))? {
                    write!(out, "{}", key).map_err(|_| KernErr::CLIErr(CLIErr::Write))?;
                }
            } else if inp.pmt == "key#async" {
                if let Some(key) = serv.kern.cli.get_key(false).map_err(|e| KernErr::CLIErr(e))? {
                    write!(out, "{}", key).map_err(|_| KernErr::CLIErr(CLIErr::Write))?;
                }
            } else {
                write!(serv.kern.cli, "\r{}", inp.pmt).map_err(|_| KernErr::CLIErr(CLIErr::Write))?;
    
                loop {
                    if let Some(key) = serv.kern.cli.get_key(false).map_err(|e| KernErr::CLIErr(e))? {
                        if let TermKey::Char(c) = key {
                            if c == '\r' || c == '\n' {
                                writeln!(serv.kern.cli).map_err(|_| KernErr::CLIErr(CLIErr::Write))?;
                                break;
                            }
    
                            if c == '\u{8}' {
                                out.pop();
                            } else {
                                write!(out, "{}", c).map_err(|_| KernErr::CLIErr(CLIErr::Write))?;
                            }
    
                            write!(serv.kern.cli, "\r{}{out} ", inp.pmt).map_err(|_| KernErr::CLIErr(CLIErr::Write))?;
                        }
                    }
                }
            }

            if !out.is_empty() {
                if self.prs {
                    let u = Unit::parse(out.chars(), serv.kern)?.0;
                    return Ok(Some(serv.kern.msg(&msg.ath, u)?))
                } else {
                    let _msg = vec![
                        (Unit::Str("msg".into()), Unit::Str(out))
                    ];
    
                    return Ok(Some(serv.kern.msg(&msg.ath, Unit::Map(_msg))?))
                };
            }
        } else {
            return Ok(Some(msg));
        }

        Ok(None)
    }
}

impl ServHlr for Term {
    fn inst(msg: Msg, serv: &mut Serv) -> Result<(Self, Msg), KernErr> {
        let mut inst = Term::default();

        // config instance
        msg.msg.find_bool(&mut vec!["trc".into()].iter()).map(|v| inst.trc = v);
        msg.msg.find_bool(&mut vec!["cls".into()].iter()).map(|v| inst.cls = v);
        msg.msg.find_bool(&mut vec!["nl".into()].iter()).map(|v| inst.nl = v);
        msg.msg.find_bool(&mut vec!["prs".into()].iter()).map(|v| inst.prs = v);

        msg.msg.find_str(&mut vec!["inp".into()].iter()).map(|s| {
            inst.inp = Some(Inp {
                pmt: s
            })
        });

        msg.msg.find_list(&mut vec!["img".into()].iter()).map(|lst| {
            let img = lst.iter().filter_map(|u| u.as_int()).map(|v| v as u32).collect();

            inst.img = Some(Img {
                img
            })
        });

        let e = msg.msg.find_str(&mut vec!["img".into()].iter()).map(|s| {
            let mut dec = GZipDecoder::new();

            let img_v = Base64::decode_vec(s.as_str()).map_err(|_| KernErr::DecodeFault)?;
            let decompressed = img_v.iter().cloned().decode(&mut dec).collect::<Result<Vec<_>, _>>().map_err(|_| KernErr::DecompressionFault)?;

            let img_s = String::from_utf8(decompressed).map_err(|_| KernErr::DecodeFault)?;
            let img_u = Unit::parse(img_s.chars(), serv.kern)?.0;

            if let Unit::Lst(lst) = img_u {
                let img = lst.iter().filter_map(|u| u.as_int()).map(|v| v as u32).collect();

                inst.img = Some(Img {
                    img
                })
            } else {
                return Err(KernErr::ParseErr(UnitParseErr::NotList));
            }

            Ok(())
        });

        if let Some(e) = e {
            e?;
        }

        msg.msg.find_unit(&mut vec!["msg".into()].iter()).map(|u| {
            inst.msg = Some(format!("{}", u));
        });

        Ok((inst, msg))
    }

    fn handle(&self, msg: Msg, serv: &mut Serv) -> Result<Option<Msg>, KernErr> {
        if self.trc {
            writeln!(serv.kern.cli, "INFO vnix:io.term: {}", msg).map_err(|_| KernErr::CLIErr(CLIErr::Write))?;
            return Ok(Some(msg))
        } else {
            // gfx
            if self.img.is_some() {
                self.img_hlr(serv)?;
 
                // wait for key
                serv.kern.cli.get_key(true).map_err(|e| KernErr::CLIErr(e))?;
                serv.kern.cli.clear().map_err(|_| KernErr::CLIErr(CLIErr::Clear))?;

                return Ok(Some(msg));
            }

            // cli
            if let Some(msg) = self.cli_hlr(msg, serv)? {
                return Ok(Some(msg));
            }
        }

        Ok(None)
    }
}
