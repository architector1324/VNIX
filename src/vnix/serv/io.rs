use core::fmt::Write;

use alloc::boxed::Box;
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


#[derive(Debug)]
struct Inp {
    pmt: String
}

#[derive(Debug)]
struct Img {
    img: Vec<u32>
}

#[derive(Debug)]
struct PutChar {
    pos: (usize, usize),
    ch: String
}

#[derive(Debug)]
enum Get {
    CliRes,
    GfxRes
}

#[derive(Debug)]
pub struct Term {
    inp: Option<Inp>,
    img: Option<Img>,
    put: Option<Vec<PutChar>>,
    get: Option<Get>,
    msg: Option<String>,

    nl: bool,
    cls: bool,
    trc: bool,
    prs: bool
}

impl Default for Term {
    fn default() -> Self {
        Term {
            inp: None,
            img: None,
            put: None,
            get: None,
            msg: None,

            nl: true,
            cls: false,
            trc: false,
            prs: false
        }
    }
}

impl Inp {
    fn msg(prs:bool, s: String, msg: &Msg, serv: &mut Serv) -> Result<Option<Msg>, KernErr> {
        if !s.is_empty() {
            if !prs {
                let _msg = Unit::Map(vec![
                    (Unit::Str("msg".into()), Unit::Str(s))
                ]);

                return Ok(Some(serv.kern.msg(&msg.ath, _msg)?))
            };
        }

        if prs {
            let u = if !s.is_empty() {
                Unit::parse(s.chars(), serv.kern)?.0
            } else {
                Unit::None
            };

            let _msg = Unit::Map(vec![
                (Unit::Str("msg".into()), u)
            ]);

            return Ok(Some(serv.kern.msg(&msg.ath, _msg)?))
        }

        return Ok(None)
    }

    fn handle(&self, prs:bool, msg: &Msg, serv: &mut Serv) -> Result<Option<Msg>, KernErr> {
        let mut out = String::new();

        match self.pmt.as_str() {
            "key" => {
                if let Some(key) = serv.kern.cli.get_key(true).map_err(|e| KernErr::CLIErr(e))? {
                    write!(out, "{}", key).map_err(|_| KernErr::CLIErr(CLIErr::Write))?;
                }
                return Inp::msg(prs, out, msg, serv);
            },
            "key#async" => {
                if let Some(key) = serv.kern.cli.get_key(false).map_err(|e| KernErr::CLIErr(e))? {
                    write!(out, "{}", key).map_err(|_| KernErr::CLIErr(CLIErr::Write))?;
                }
                return Inp::msg(prs, out, msg, serv);
            }
            _ => ()
        }

        // input str
        write!(serv.kern.cli, "\r{}", self.pmt).map_err(|_| KernErr::CLIErr(CLIErr::Write))?;

        while let Some(key) = serv.kern.cli.get_key(true).map_err(|e| KernErr::CLIErr(e))? {
            if let TermKey::Char(c) = key {
                if c == '\r' || c == '\n' {
                    writeln!(serv.kern.cli).map_err(|_| KernErr::CLIErr(CLIErr::Write))?;
                    break;
                }

                if c == '\u{8}' {
                    out.pop();
                } else if c == '\u{3}' {
                    writeln!(serv.kern.cli, "\r{}{out} ", self.pmt).map_err(|_| KernErr::CLIErr(CLIErr::Write))?;
                    return Ok(None);
                } else {
                    write!(out, "{}", c).map_err(|_| KernErr::CLIErr(CLIErr::Write))?;
                }

                write!(serv.kern.cli, "\r{}{out} ", self.pmt).map_err(|_| KernErr::CLIErr(CLIErr::Write))?;
            }
        }

        // create msg
        return Inp::msg(prs, out, msg, serv);
    }
}

impl Get {
    fn handle(&self, msg: Msg, serv: &mut Serv) -> Result<Option<Msg>, KernErr> {        
        let res = match self {
            Get::CliRes => serv.kern.cli.res().map_err(|e| KernErr::CLIErr(e))?,
            Get::GfxRes => serv.kern.disp.res().map_err(|e| KernErr::DispErr(e))? 
        };

        let _msg = Unit::Map(vec![
            (
                Unit::Str("msg".into()),
                Unit::Pair((
                    Box::new(Unit::Int(res.0 as i32)),
                    Box::new(Unit::Int(res.1 as i32))
                ))
            )
        ]);

        return Ok(Some(serv.kern.msg(&msg.ath, _msg)?));
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

    fn cls(&self, serv: &mut Serv) -> Result<(), KernErr> {
        if self.cls {
            serv.kern.cli.clear().map_err(|_| KernErr::CLIErr(CLIErr::Clear))?;
        }
        Ok(())
    }

    fn print_msg(&self, msg: &Msg, serv: &mut Serv) -> Result<(), KernErr> {
        if let Some(ref s) = self.msg {
            if self.nl {
                writeln!(serv.kern.cli, "{}", s).map_err(|_| KernErr::CLIErr(CLIErr::Write))?;
            } else {
                write!(serv.kern.cli, "{}", s).map_err(|_| KernErr::CLIErr(CLIErr::Write))?;
            }
        } else if self.inp.is_none() && self.get.is_none() && !self.cls {
            if self.nl {
                writeln!(serv.kern.cli, "{}", msg.msg).map_err(|_| KernErr::CLIErr(CLIErr::Write))?;
            } else {
                write!(serv.kern.cli, "{}", msg.msg).map_err(|_| KernErr::CLIErr(CLIErr::Write))?;
            }
        }

        Ok(())
    }

    fn put_char(&self, serv: &mut Serv) -> Result<bool, KernErr> {
        if let Some(put) = &self.put {
            let res = serv.kern.cli.res().map_err(|e| KernErr::CLIErr(e))?;

            let mut out = ".".repeat(res.0 * res.1);
            serv.kern.cli.clear().map_err(|_| KernErr::CLIErr(CLIErr::Clear))?;

            for ch in put {
                let offs = ch.pos.0 + res.0 * (ch.pos.1 + 1);
                out.replace_range(offs..offs + 1, &ch.ch);

                write!(serv.kern.cli, "{}", out).map_err(|_| KernErr::CLIErr(CLIErr::Write))?;
            }

            // wait for key
            serv.kern.cli.get_key(true).map_err(|e| KernErr::CLIErr(e))?;
            serv.kern.cli.clear().map_err(|_| KernErr::CLIErr(CLIErr::Clear))?;

            return Ok(true);
        }

        Ok(false)
    }

    fn cli_hlr(&self, msg: Msg, serv: &mut Serv) -> Result<Option<Msg>, KernErr> {
        self.cls(serv)?;
        self.print_msg(&msg, serv)?;

        if self.put_char(serv)? {
            return Ok(Some(msg));
        }

        if let Some(get) = &self.get {
            return get.handle(msg, serv);
        }

        if let Some(inp) = &self.inp {
            return inp.handle(self.prs, &msg, serv);
        }

        return Ok(Some(msg));
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

        msg.msg.find_pair(&mut vec!["put".into()].iter()).iter()
            .filter_map(|(u0, u1)| Some((u0.as_str()?, u1.as_pair()?)))
            .filter_map(|(ch, (x, y))| Some((ch, (x.as_int()?, y.as_int()?))))
            .map(|(ch, (x, y))| {
                let ch = PutChar {
                    pos: (x as usize, y as usize),
                    ch
                };
                inst.put.replace(vec![ch]);
            }).for_each(drop);

        msg.msg.find_list(&mut vec!["put".into()].iter()).map(|lst| {
            inst.put = lst.iter().filter_map(|u| u.as_pair())
                .filter_map(|(u0, u1)| Some((u0.as_str()?, u1.as_pair()?)))
                .filter_map(|(ch, (x, y))| Some((ch, (x.as_int()?, y.as_int()?))))
                .map(|(ch, (x, y))| {
                    Some(PutChar {
                        pos: (x as usize, y as usize),
                        ch
                    })
            }).collect::<Option<Vec<_>>>();
        });

        msg.msg.find_str(&mut vec!["inp".into()].iter()).map(|s| {
            inst.inp.replace(Inp {
                pmt: s
            })
        });

        msg.msg.find_list(&mut vec!["img".into()].iter()).map(|lst| {
            let img = lst.iter().filter_map(|u| u.as_int()).map(|v| v as u32).collect();

            inst.img.replace(Img {
                img
            })
        });

        msg.msg.find_str(&mut vec!["get".into()].iter()).map(|s| {
            match s.as_ref() {
                "cli.res" => inst.get.replace(Get::CliRes),
                "gfx.res" => inst.get.replace(Get::GfxRes),
                _ => None
            }
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
                });
            } else {
                return Err(KernErr::ParseErr(UnitParseErr::NotList));
            }

            Ok(())
        });

        if let Some(e) = e {
            e?;
        }

        msg.msg.find_unit(&mut vec!["msg".into()].iter()).map(|u| {
            inst.msg.replace(format!("{}", u))
        });

        Ok((inst, msg))
    }

    fn handle(&self, msg: Msg, serv: &mut Serv) -> Result<Option<Msg>, KernErr> {
        if self.trc {
            writeln!(serv.kern.cli, "INFO vnix:io.term: {}", msg).map_err(|_| KernErr::CLIErr(CLIErr::Write))?;
            return Ok(Some(msg))
        }

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

        Ok(None)
    }
}
