use core::fmt::Write;

use alloc::boxed::Box;
use alloc::{format, vec};
use alloc::string::String;
use alloc::vec::Vec;

use crate::driver::{CLIErr, TermKey};

use crate::vnix::core::msg::Msg;
use crate::vnix::core::unit::{Unit, Schema, SchemaUnit, FromUnit};

use crate::vnix::core::serv::{Serv, ServHlr};
use crate::vnix::core::kern::{KernErr, Kern};
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
    fn msg(prs:bool, s: String, msg: Msg, kern: &mut Kern) -> Result<Option<Msg>, KernErr> {
        let u = if !s.is_empty() {
            if prs {
                Unit::parse(s.chars()).map_err(|e| KernErr::ParseErr(e))?.0
            } else {
                Unit::Str(s)
            }
        } else {
            Unit::None
        };

        let _msg = Unit::Map(vec![
            (Unit::Str("msg".into()), u)
        ]);

        return Ok(Some(kern.msg(&msg.ath, _msg)?));
    }

    fn handle(&self, prs:bool, msg: Msg, kern: &mut Kern) -> Result<Option<Msg>, KernErr> {
        let mut out = String::new();

        match self.pmt.as_str() {
            "key" => {
                if let Some(key) = kern.cli.get_key(true).map_err(|e| KernErr::CLIErr(e))? {
                    write!(out, "{}", key).map_err(|_| KernErr::CLIErr(CLIErr::Write))?;
                }
                return Inp::msg(prs, out, msg, kern);
            },
            "key#async" => {
                if let Some(key) = kern.cli.get_key(false).map_err(|e| KernErr::CLIErr(e))? {
                    write!(out, "{}", key).map_err(|_| KernErr::CLIErr(CLIErr::Write))?;
                }
                return Inp::msg(prs, out, msg, kern);
            }
            _ => ()
        }

        // input str
        write!(kern.cli, "\r{}", self.pmt).map_err(|_| KernErr::CLIErr(CLIErr::Write))?;

        loop {
            if let Some(key) = kern.cli.get_key(true).map_err(|e| KernErr::CLIErr(e))? {
                if let TermKey::Char(c) = key {
                    if c == '\r' || c == '\n' {
                        writeln!(kern.cli).map_err(|_| KernErr::CLIErr(CLIErr::Write))?;
                        break;
                    }
    
                    if c == '\u{8}' {
                        out.pop();
                    } else if c == '\u{3}' {
                        writeln!(kern.cli, "\r{}{out} ", self.pmt).map_err(|_| KernErr::CLIErr(CLIErr::Write))?;
                        return Ok(None);
                    } else if !c.is_ascii_control() {
                        write!(out, "{}", c).map_err(|_| KernErr::CLIErr(CLIErr::Write))?;
                    }
    
                    write!(kern.cli, "\r{}{out} ", self.pmt).map_err(|_| KernErr::CLIErr(CLIErr::Write))?;
                    // write!(kern.cli, "{c}").map_err(|_| KernErr::CLIErr(CLIErr::Write))?;
                }
            }
        }

        // create msg
        return Inp::msg(prs, out, msg, kern);
    }
}

impl Get {
    fn handle(&self, msg: Msg, kern: &mut Kern) -> Result<Option<Msg>, KernErr> {        
        let res = match self {
            Get::CliRes => kern.cli.res().map_err(|e| KernErr::CLIErr(e))?,
            Get::GfxRes => kern.disp.res().map_err(|e| KernErr::DispErr(e))? 
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

        return Ok(Some(kern.msg(&msg.ath, _msg)?));
    }
}

impl FromUnit for Img {
    fn from_unit(u: &Unit) -> Option<Self> {
        let tmp = u.find_pair(&mut vec!["img".into()].iter()).iter()
            .filter_map(|(u0, u1)| Some((u0.as_pair()?, u1.as_vec()?)))
            .filter_map(|((w, h), lst)| Some(((w.as_int()?, h.as_int()?), lst)))
            .map(|((w, h), lst)| {
                let img = lst.iter().filter_map(|u| u.as_int()).map(|v| v as u32).collect::<Vec<_>>();
                ((w as usize, h as usize), img)
            }).next();

        if tmp.is_some() {
            let tmp = tmp?;
            return Some(Img {
                size: tmp.0,
                img: tmp.1
            })
        }

        let tmp = u.find_pair(&mut vec!["img".into()].iter()).iter()
            .filter_map(|(u0, u1)| Some((u0.as_pair()?, u1.as_str()?)))
            .filter_map(|((w, h), s)| Some(((w.as_int()?, h.as_int()?), s)))
            .map(|((w, h), s)| {
                let img0 = utils::decompress(s.as_str()).ok()?;
                let img_s = utils::decompress(img0.as_str()).ok()?;

                let img_u = Unit::parse(img_s.chars()).ok()?.0;

                if let Unit::Lst(lst) = img_u {
                    let img = lst.iter().filter_map(|u| u.as_int()).map(|v| v as u32).collect();

                    return Some(((w as usize, h as usize), img))
                }
                None
            }).next().flatten();

        let tmp = tmp?;

        return Some(Img {
            size: tmp.0,
            img: tmp.1
        })
    }
}

impl FromUnit for Vec<PutChar> {
    fn from_unit(u: &Unit) -> Option<Self> {
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
            return put;
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

        put
    }
}

impl FromUnit for Sprite {
    fn from_unit(u: &Unit) -> Option<Self> {
        let mut spr = None;

        u.find_map(&mut vec!["spr".into()].iter()).iter()
            .filter_map(|m| {
                let m = Unit::Map(m.clone());
                Some((
                    (
                        m.find_int(&mut vec!["x".into()].iter())?,
                        m.find_int(&mut vec!["y".into()].iter())?
                    ),
                    Img::from_unit(&m)?
                ))
            })
            .map(|((x, y), img)| {
                spr.replace(Sprite {
                    pos: (x, y),
                    img
                })
            }).for_each(drop);

        spr
    }
}

impl Term {
    fn img_hlr(&self, kern: &mut Kern) -> Result<(), KernErr> {
        if let Some(ref img) = self.img {
            for x in 0..img.size.0 {
                for y in 0..img.size.1 {
                    if let Some(px) = img.img.get(x + img.size.0 * y) {
                        kern.disp.px(*px, x, y).map_err(|e| KernErr::DispErr(e))?;
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

                        kern.disp.px(*px, x + x_offs, y + y_offs).map_err(|e| KernErr::DispErr(e))?;
                    }
                }
            }
        }

        Ok(())
    }

    fn cls(&self, kern: &mut Kern) -> Result<(), KernErr> {
        if self.cls {
            kern.cli.clear().map_err(|_| KernErr::CLIErr(CLIErr::Clear))?;
        }
        Ok(())
    }

    fn print_msg(&self, msg: &Msg, kern: &mut Kern) -> Result<(), KernErr> {
        if let Some(ref s) = self.msg {
            if self.nl {
                writeln!(kern.cli, "{}", s).map_err(|_| KernErr::CLIErr(CLIErr::Write))?;
            } else {
                write!(kern.cli, "{}", s).map_err(|_| KernErr::CLIErr(CLIErr::Write))?;
            }
        } else if self.inp.is_none() && self.get.is_none() && !self.cls {
            if self.nl {
                writeln!(kern.cli, "{}", msg.msg).map_err(|_| KernErr::CLIErr(CLIErr::Write))?;
            } else {
                write!(kern.cli, "{}", msg.msg).map_err(|_| KernErr::CLIErr(CLIErr::Write))?;
            }
        }

        Ok(())
    }

    fn put_char(&self, kern: &mut Kern) -> Result<bool, KernErr> {
        if let Some(put) = &self.put {
            let res = kern.cli.res().map_err(|e| KernErr::CLIErr(e))?;

            let mut out = ".".repeat(res.0 * res.1);
            kern.cli.clear().map_err(|_| KernErr::CLIErr(CLIErr::Clear))?;

            for ch in put {
                let offs = ch.pos.0 + res.0 * (ch.pos.1 + 1);
                out.replace_range(offs..offs + 1, &ch.ch);
            }
            write!(kern.cli, "{}", out).map_err(|_| KernErr::CLIErr(CLIErr::Write))?;

            // wait for key
            kern.cli.get_key(true).map_err(|e| KernErr::CLIErr(e))?;
            kern.cli.clear().map_err(|_| KernErr::CLIErr(CLIErr::Clear))?;

            return Ok(true);
        }

        Ok(false)
    }

    fn cli_hlr(&self, msg: Msg, kern: &mut Kern) -> Result<Option<Msg>, KernErr> {
        self.cls(kern)?;
        self.print_msg(&msg, kern)?;

        if self.put_char(kern)? {
            return Ok(Some(msg));
        }

        if let Some(get) = &self.get {
            return get.handle(msg, kern);
        }

        if let Some(inp) = &self.inp {
            return inp.handle(self.prs, msg, kern);
        }

        return Ok(Some(msg));
    }
}

impl FromUnit for Term {
    fn from_unit(u: &Unit) -> Option<Self> {
        let mut inst = Term::default();

        // config instance
        let mut trc = None;
        let mut cls = None;
        let mut nl = None;
        let mut prs = None;
        let mut inp = None;
        let mut get = None;
        let mut _msg = None;

        let mut schm = Schema::Unit(SchemaUnit::Map(vec![
            (
                Schema::Value(Unit::Str("trc".into())),
                Schema::Unit(SchemaUnit::Bool(&mut trc))
            ),
            (
                Schema::Value(Unit::Str("cls".into())),
                Schema::Unit(SchemaUnit::Bool(&mut cls))
            ),
            (
                Schema::Value(Unit::Str("nl".into())),
                Schema::Unit(SchemaUnit::Bool(&mut nl))
            ),
            (
                Schema::Value(Unit::Str("prs".into())),
                Schema::Unit(SchemaUnit::Bool(&mut prs))
            ),
            (
                Schema::Value(Unit::Str("inp".into())),
                Schema::Unit(SchemaUnit::Str(&mut inp))
            ),
            (
                Schema::Value(Unit::Str("get".into())),
                Schema::Unit(SchemaUnit::Str(&mut get))
            ),
            (
                Schema::Value(Unit::Str("msg".into())),
                Schema::Unit(SchemaUnit::Unit(&mut _msg))
            ),
        ]));

        schm.find(u);

        trc.map(|v| inst.trc = v);
        cls.map(|v| inst.cls = v);
        nl.map(|v| inst.nl = v);
        prs.map(|v| inst.prs = v);

        inp.map(|s| inst.inp.replace(Inp{pmt: s}));

        get.map(|s| {
            match s.as_ref() {
                "cli.res" => inst.get.replace(Get::CliRes),
                "gfx.res" => inst.get.replace(Get::GfxRes),
                _ => None
            }
        });

        _msg.map(|u| {
            match u {
                Unit::Str(s) => inst.msg.replace(format!("{}", s)),
                _ => inst.msg.replace(format!("{}", u))
            }
        });

        if let Some(put) = Vec::<PutChar>::from_unit(u) {
            inst.put.replace(put);
        }

        if let Some(img) = Img::from_unit(u) {
            inst.img.replace(img);
        }

        if let Some(spr) = Sprite::from_unit(u) {
            inst.spr.replace(spr);
        }

        Some(inst)
    }
}

impl ServHlr for Term {
    fn handle(&self, msg: Msg, _serv: &mut Serv, kern: &mut Kern) -> Result<Option<Msg>, KernErr> {
        if self.trc {
            writeln!(kern.cli, "INFO vnix:io.term: {}", msg).map_err(|_| KernErr::CLIErr(CLIErr::Write))?;
            return Ok(Some(msg))
        }

        // gfx
        if self.img.is_some() || self.spr.is_some() {
            self.img_hlr(kern)?;

            // wait for key
            kern.cli.get_key(true).map_err(|e| KernErr::CLIErr(e))?;
            kern.cli.clear().map_err(|_| KernErr::CLIErr(CLIErr::Clear))?;

            return Ok(Some(msg));
        }

        // cli
        if let Some(msg) = self.cli_hlr(msg, kern)? {
            return Ok(Some(msg));
        }

        Ok(None)
    }
}
