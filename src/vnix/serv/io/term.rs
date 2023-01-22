use core::fmt::Write;

use alloc::boxed::Box;
use alloc::{format, vec};
use alloc::string::String;
use alloc::vec::Vec;

use crate::driver::{CLIErr, TermKey};

use crate::vnix::core::msg::Msg;
use crate::vnix::core::unit::{Unit, UnitParseErr};

use crate::vnix::core::serv::{Serv, ServHlr};
use crate::vnix::core::kern::KernErr;
use crate::vnix::utils;


#[derive(Debug)]
struct Inp {
    pmt: String
}

#[derive(Debug)]
struct Img {
    size: (usize, usize),
    img: Vec<u32>
}

#[derive(Debug)]
struct Sprite {
    pos: (i32, i32),
    img: Img
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
    spr: Option<Sprite>,
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
            spr: None,
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
    fn msg(prs:bool, s: String, msg: Msg, serv: &mut Serv) -> Result<Option<Msg>, KernErr> {
        let u = if !s.is_empty() {
            if prs {
                Unit::parse(s.chars(), serv.kern)?.0
            } else {
                Unit::Str(s)
            }
        } else {
            Unit::None
        };

        let _msg = Unit::Map(vec![
            (Unit::Str("msg".into()), u)
        ]);

        return Ok(Some(serv.kern.msg(&msg.ath, _msg)?));
    }

    fn handle(&self, prs:bool, msg: Msg, serv: &mut Serv) -> Result<Option<Msg>, KernErr> {
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

        loop {
            if let Some(key) = serv.kern.cli.get_key(true).map_err(|e| KernErr::CLIErr(e))? {
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
                    } else if !c.is_ascii_control() {
                        write!(out, "{}", c).map_err(|_| KernErr::CLIErr(CLIErr::Write))?;
                    }
    
                    write!(serv.kern.cli, "\r{}{out} ", self.pmt).map_err(|_| KernErr::CLIErr(CLIErr::Write))?;
                    // write!(serv.kern.cli, "{c}").map_err(|_| KernErr::CLIErr(CLIErr::Write))?;
                }
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

impl Img {
    fn find_img(u: &Unit, serv: &mut Serv) -> Result<Option<Self>, KernErr> {
        let mut _img = None;
        
        u.find_pair(&mut vec!["img".into()].iter()).iter()
            .filter_map(|(u0, u1)| Some((u0.as_pair()?, u1.as_vec()?)))
            .filter_map(|((w, h), lst)| Some(((w.as_int()?, h.as_int()?), lst)))
            .map(|((w, h), lst)| {
                let img = lst.iter().filter_map(|u| u.as_int()).map(|v| v as u32).collect();

                _img = Some(Img {
                    size: (w as usize, h as usize),
                    img
                });
            }).for_each(drop);

        if _img.is_some() {
            return Ok(_img);
        }

        u.find_pair(&mut vec!["img".into()].iter()).iter()
            .filter_map(|(u0, u1)| Some((u0.as_pair()?, u1.as_str()?)))
            .filter_map(|((w, h), s)| Some(((w.as_int()?, h.as_int()?), s)))
            .map(|((w, h), s)| {
                let img0 = utils::decompress(s.as_str())?;
                let img_s = utils::decompress(img0.as_str())?;

                let img_u = Unit::parse(img_s.chars(), serv.kern)?.0;

                if let Unit::Lst(lst) = img_u {
                    let img = lst.iter().filter_map(|u| u.as_int()).map(|v| v as u32).collect();

                    _img = Some(Img {
                        size: (w as usize, h as usize),
                        img
                    });
                } else {
                    return Err(KernErr::ParseErr(UnitParseErr::NotList));
                }
                Ok(())
            }).collect::<Result<(), KernErr>>()?;

        Ok(_img)
    }
}

impl PutChar {
    fn find_put(u: &Unit, _serv: &mut Serv) -> Result<Option<Vec<Self>>, KernErr> {
        let mut put = None;
        
        u.find_pair(&mut vec!["put".into()].iter()).iter()
            .filter_map(|(u0, u1)| Some((u0.as_str()?, u1.as_pair()?)))
            .filter_map(|(ch, (x, y))| Some((ch, (x.as_int()?, y.as_int()?))))
            .map(|(ch, (x, y))| {
                let ch = PutChar {
                    pos: (x as usize, y as usize),
                    ch
                };
                put.replace(vec![ch]);
            }).for_each(drop);

        if put.is_some() {
            return Ok(put);
        }

        u.find_list(&mut vec!["put".into()].iter()).map(|lst| {
            put = lst.iter().filter_map(|u| u.as_pair())
                .filter_map(|(u0, u1)| Some((u0.as_str()?, u1.as_pair()?)))
                .filter_map(|(ch, (x, y))| Some((ch, (x.as_int()?, y.as_int()?))))
                .map(|(ch, (x, y))| {
                    Some(PutChar {
                        pos: (x as usize, y as usize),
                        ch
                    })
            }).collect::<Option<Vec<_>>>();
        });

        Ok(put)
    }
}

impl Sprite {
    fn find_spr(u: &Unit, serv: &mut Serv) -> Result<Option<Self>, KernErr> {
        let mut spr = None;

        u.find_map(&mut vec!["spr".into()].iter()).iter()
            .filter_map(|m| {
                let m = Unit::Map(m.clone());
                Some((
                    (
                        m.find_int(&mut vec!["x".into()].iter())?,
                        m.find_int(&mut vec!["y".into()].iter())?
                    ),
                    Img::find_img(&m, serv).ok()??
                ))
            })
            .map(|((x, y), img)| {
                spr.replace(Sprite {
                    pos: (x, y),
                    img
                })
            }).for_each(drop);

        Ok(spr)
    }
}

impl Term {
    fn img_hlr(&self, serv: &mut Serv) -> Result<(), KernErr> {
        if let Some(ref img) = self.img {
            for x in 0..img.size.0 {
                for y in 0..img.size.1 {
                    if let Some(px) = img.img.get(x + img.size.0 * y) {
                        serv.kern.disp.px(*px, x, y).map_err(|e| KernErr::DispErr(e))?;
                    }
                }
            }
        }

        if let Some(ref spr) = self.spr {
            let w = spr.img.size.0;
            let h = spr.img.size.1;

            for x in 0..w {
                for y in 0..h {
                    if let Some(px) = spr.img.img.get(x + w * y) {
                        let x_offs = (spr.pos.0 - (w as i32 / 2)) as usize;
                        let y_offs = (spr.pos.1 - (h as i32 / 2)) as usize;

                        serv.kern.disp.px(*px, x + x_offs, y + y_offs).map_err(|e| KernErr::DispErr(e))?;
                    }
                }
            }
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
            }
            write!(serv.kern.cli, "{}", out).map_err(|_| KernErr::CLIErr(CLIErr::Write))?;

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
            return inp.handle(self.prs, msg, serv);
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

        if let Some(put) = PutChar::find_put(&msg.msg, serv)? {
            inst.put.replace(put);
        }

        msg.msg.find_str(&mut vec!["inp".into()].iter()).map(|s| {
            inst.inp.replace(Inp {
                pmt: s
            })
        });

        msg.msg.find_str(&mut vec!["get".into()].iter()).map(|s| {
            match s.as_ref() {
                "cli.res" => inst.get.replace(Get::CliRes),
                "gfx.res" => inst.get.replace(Get::GfxRes),
                _ => None
            }
        });

        if let Some(img) = Img::find_img(&msg.msg, serv)? {
            inst.img.replace(img);
        }

        if let Some(spr) = Sprite::find_spr(&msg.msg, serv)? {
            inst.spr.replace(spr);
        }

        msg.msg.find_unit(&mut vec!["msg".into()].iter()).filter(|u| u.as_none().is_none()).map(|u| {
            match u {
                Unit::Str(s) => inst.msg.replace(format!("{}", s)),
                _ => inst.msg.replace(format!("{}", u))
            }
        });

        Ok((inst, msg))
    }

    fn handle(&self, msg: Msg, serv: &mut Serv) -> Result<Option<Msg>, KernErr> {
        if self.trc {
            writeln!(serv.kern.cli, "INFO vnix:io.term: {}", msg).map_err(|_| KernErr::CLIErr(CLIErr::Write))?;
            return Ok(Some(msg))
        }

        // gfx
        if self.img.is_some() || self.spr.is_some() {
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
