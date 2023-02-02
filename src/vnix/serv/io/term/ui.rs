use alloc::boxed::Box;
use alloc::{vec, format};
use alloc::string::String;
use alloc::vec::Vec;


use crate::driver::TermKey;
use crate::vnix::core::msg::Msg;
use crate::vnix::core::kern::{KernErr, Kern};
use crate::vnix::core::unit::{Unit, FromUnit, SchemaMapSecondRequire, SchemaMapEntry, SchemaBool, SchemaInt, SchemaStr, SchemaUnit, Schema, SchemaMap, SchemaMapRequire, SchemaRef, SchemaPair, SchemaOr, SchemaSeq, Or, DisplayShort};

use crate::vnix::utils;

use super::{TermAct, Mode, Term, content};


pub struct Skin {
    cursor: Img
}

trait UIAct {
    fn ui_act(&self, pos: (i32, i32), size:(usize, usize), term: &mut Term, kern: &mut Kern) -> Result<(), KernErr>;
    fn ui_gfx_act(&self, pos: (i32, i32), size:(usize, usize), term: &mut Term, kern: &mut Kern) -> Result<(), KernErr>;
}

#[derive(Debug, Clone)]
pub struct Inp {
    pub pmt: String,
    pub prs: bool,
    pub sct: bool,
    pub out: Vec<String>
}

#[derive(Debug, Clone)]
pub struct Say {
    pub msg: Unit,
    pub shrt: Option<usize>,
    pub nl: bool
}

#[derive(Debug, Clone)]
pub struct Put {
    pub pos: (i32, i32),
    pub str: String
}

#[derive(Debug, Clone)]
pub struct Img {
    pub size: (usize, usize),
    pub img: Vec<u32>
}

#[derive(Debug, Clone)]
pub struct Sprite {
    pub pos: (i32, i32),
    pub img: Img
}


#[derive(Debug, Clone)]
pub struct Win {
    title: Option<String>,
    border: bool,
    mode: Mode,

    pos: Option<(i32, i32)>,
    size: Option<(usize, usize)>,

    content: Option<Box<UI>>,
}

#[derive(Debug, Clone)]
pub enum UI {
    VStack(Vec<UI>),
    HStack(Vec<UI>),
    Win(Win)
}


impl Img {
    fn draw(&self, pos: (i32, i32), src: u32, kern: &mut Kern) -> Result<(), KernErr> {
        kern.disp.blk(pos, self.size, src, &self.img).map_err(|e| KernErr::DispErr(e))
    }
}

impl FromUnit for Inp {
    fn from_unit_loc(u: &Unit) -> Option<Self> {
        Self::from_unit(u, u)
    }

    fn from_unit(glob: &Unit, u: &Unit) -> Option<Self> {
        let schm = SchemaMapRequire(
            SchemaMapEntry(Unit::Str("pmt".into()), SchemaStr),
            SchemaMapSecondRequire(
                SchemaMapEntry(Unit::Str("prs".into()), SchemaBool),
                SchemaMap(
                    SchemaMapEntry(Unit::Str("sct".into()), SchemaBool),
                    SchemaMapEntry(Unit::Str("out".into()), SchemaRef),

                )
            )
        );

        schm.find_deep(glob, u).map(|(pmt, (prs, (sct, out)))| {
            Inp {
                pmt,
                prs: prs.unwrap_or(false),
                sct: sct.unwrap_or(false),
                out: out.unwrap_or(vec!["msg".into()])
            }
        })
    }
}

impl FromUnit for Say {
    fn from_unit_loc(u: &Unit) -> Option<Self> {
        Self::from_unit(u, u)
    }

    fn from_unit(glob: &Unit, u: &Unit) -> Option<Self> {
        let schm = SchemaMapSecondRequire(
            SchemaMapEntry(Unit::Str("shrt".into()), SchemaInt),
            SchemaMap(
                SchemaMapEntry(Unit::Str("nl".into()), SchemaBool),
                SchemaOr(
                    SchemaMapEntry(Unit::Str("say".into()), SchemaRef),
                    SchemaMapEntry(Unit::Str("say".into()), SchemaUnit),
                )
            )
        );

        schm.find_deep(glob, u).and_then(|(shrt, (nl, or))| {
            let msg = if let Some(or) = or {
                match or {
                    Or::First(path) => Unit::Ref(path),
                    Or::Second(u) => u
                }
            } else {
                Unit::Ref(vec!["msg".into()])
            };

            Some(Say {
                msg,
                shrt: shrt.map(|shrt| shrt as usize),
                nl: nl.unwrap_or(false)
            })
        })
    }
}

impl FromUnit for Put {
    fn from_unit_loc(u: &Unit) -> Option<Self> {
        Self::from_unit(u, u)
    }

    fn from_unit(glob: &Unit, u: &Unit) -> Option<Self> {
        let schm = SchemaMapRequire(
            SchemaMapEntry(
                Unit::Str("pos".into()),
                SchemaPair(SchemaInt, SchemaInt)
            ),
            SchemaMapEntry(
                Unit::Str("put".into()),
                SchemaStr
            )
        );

        schm.find_deep(glob, u).map(|(pos, str)| {
            Put {pos, str}
        })
    }
}

impl FromUnit for Img {
    fn from_unit_loc(u: &Unit) -> Option<Self> {
        Self::from_unit(u, u)
    }

    fn from_unit(glob: &Unit, u: &Unit) -> Option<Self> {
        let schm = SchemaMapRequire(
            SchemaMapEntry(
                Unit::Str("size".into()),
                SchemaPair(SchemaInt, SchemaInt)
            ),
            SchemaMapEntry(
                Unit::Str("img".into()),
                SchemaOr(
                    SchemaStr,
                    SchemaSeq(SchemaInt)
                )
            )
        );

        schm.find(glob, u).and_then(|(size, or)|{
            let img = match or {
                Or::First(s) => {
                    let img0 = utils::decompress(s.as_str()).ok()?;
                    let img_s = utils::decompress(img0.as_str()).ok()?;
                    let img_u = Unit::parse(img_s.chars()).ok()?.0.as_vec()?;

                    img_u.iter().filter_map(|u| u.as_int()).map(|v| v as u32).collect()
                },
                Or::Second(seq) => seq.into_iter().map(|e| e as u32).collect()
            };

            Some(Img {
                size: (size.0 as usize, size.1 as usize),
                img
            })
        })
    }
}

impl FromUnit for Sprite {
    fn from_unit_loc(u: &Unit) -> Option<Self> {
        Self::from_unit(u, u)
    }

    fn from_unit(glob: &Unit, u: &Unit) -> Option<Self> {
        let schm = SchemaMapRequire(
            SchemaMapEntry(
                Unit::Str("pos".into()),
                SchemaPair(SchemaInt, SchemaInt)
            ),
            SchemaMapEntry(
                Unit::Str("img".into()),
                SchemaUnit
            )
        );

        schm.find_deep(glob, u).and_then(|(pos, img)| {
            let img = Img::from_unit(glob, &img)?;

            Some(Sprite {
                pos,
                img
            })
        })
    }
}

impl FromUnit for Win {
    fn from_unit_loc(u: &Unit) -> Option<Self> {
        Self::from_unit(u, u)
    }

    fn from_unit(glob: &Unit, u: &Unit) -> Option<Self> {
        let schm = SchemaMapSecondRequire(
            SchemaMapEntry(Unit::Str("brd".into()), SchemaBool),
            SchemaMapSecondRequire(
                SchemaMapEntry(Unit::Str("title".into()), SchemaStr),
                SchemaOr(
                    SchemaMapEntry(Unit::Str("win".into()), SchemaUnit),
                    SchemaMapEntry(Unit::Str("win.gfx".into()), SchemaUnit)
                )
            )
        );

        schm.find_deep(glob, u).map(|(brd, (title, or))| {
            let (mode, content) = match or {
                Or::First(u) => (Mode::Cli, u),
                Or::Second(u) => (Mode::Gfx, u)
            };

            Win {
                title,
                border: brd.unwrap_or(false),
                mode,

                pos: None,
                size: None,
                content: UI::from_unit(glob, &content).map(|ui| Box::new(ui))
            }
        })
    }
}

impl FromUnit for UI {
    fn from_unit_loc(u: &Unit) -> Option<Self> {
        Self::from_unit(u, u)
    }

    fn from_unit(glob: &Unit, u: &Unit) -> Option<Self> {
        let schm = SchemaOr(
            SchemaOr(
                SchemaMapEntry(
                    Unit::Str("hstack".into()),
                    SchemaSeq(SchemaUnit)
                ),
                SchemaMapEntry(
                    Unit::Str("vstack".into()),
                    SchemaSeq(SchemaUnit)
                ),
            ),
            SchemaUnit
        );

        schm.find_deep(glob, u).and_then(|or| {
            match or {
                Or::First(or) =>
                    match or {
                        Or::First(hstack) => Some(UI::HStack(hstack.into_iter().filter_map(|u| UI::from_unit(glob, &u)).collect())),
                        Or::Second(vstack) => Some(UI::VStack(vstack.into_iter().filter_map(|u| UI::from_unit(glob, &u)).collect())),
                    },
                Or::Second(u) => {
                    if let Some(win) = Win::from_unit(glob, &u) {
                        return Some(UI::Win(win));
                    }
                    None
                }
            }
        })
    }
}

impl TermAct for Say {
    fn act(mut self, term: &mut Term, orig: &Msg, msg: Unit, kern: &mut Kern) -> Result<Unit, KernErr> {
        match self.msg {
            Unit::Str(s) => term.print(format!("{}", s.replace("\\n", "\n").replace("\\r", "\r")).as_str(), kern)?,
            Unit::Ref(path) => {
                if let Some(_msg) = Unit::find_ref(path.into_iter(), &msg) {
                    self.msg = _msg;
                    return self.act(term, orig, msg, kern);
                }
            },
            _ => {
                if let Some(shrt) = self.shrt {
                    term.print(format!("{}", DisplayShort(&self.msg, shrt)).as_str(), kern)?;
                } else {
                    term.print(format!("{}", self.msg).as_str(), kern)?;
                }
            }
        }

        if self.nl {
            term.print(format!("\n").as_str(), kern)?;
        }

        Ok(msg)
    }
}

impl TermAct for Inp {
    fn act(self, term: &mut Term, _orig: &Msg, msg: Unit, kern: &mut Kern) -> Result<Unit, KernErr> {
        term.print(self.pmt.as_str(), kern)?;
        let out = term.input(self.sct, kern)?;

        if out.is_empty() {
            return Ok(msg);
        }

        let out = if self.prs {
            Unit::parse(out.chars()).map_err(|e| KernErr::ParseErr(e))?.0
        } else {
            Unit::Str(out)
        };

        if let Some(u) = Unit::merge_ref(self.out.into_iter(), out, msg.clone()) {
            return Ok(u);
        }
        Ok(msg)
    }
}

impl TermAct for Put {
    fn act(self, term: &mut Term, _orig: &Msg, msg: Unit, kern: &mut Kern) -> Result<Unit, KernErr> {
        match term.mode {
            Mode::Cli => {
                let (w, h) = kern.cli.res().map_err(|e| KernErr::CLIErr(e))?;

                if self.pos.0 < w as i32 && self.pos.1 < h as i32 {
                    if let Some(ch) = self.str.chars().next() {
                        term.print_glyth(ch, ((self.pos.0 * 8) as usize, (self.pos.1 * 16) as usize), 0x00ff00, kern)?;
                    }
                }
            },
            Mode::Gfx => {
                let (w, h) = kern.disp.res().map_err(|e| KernErr::DispErr(e))?;
                let (w, h) = (w / 8, h / 16);

                if self.pos.0 < w as i32 && self.pos.1 < h as i32 {
                    if let Some(ch) = self.str.chars().next() {
                        term.print_glyth(ch, ((self.pos.0 * 8) as usize, (self.pos.1 * 16) as usize), 0x00ff00, kern)?;
                    }
                }
            }
        }

        Ok(msg)
    }
}

impl TermAct for Img {
    fn act(self, _term: &mut Term, _orig: &Msg, msg: Unit, kern: &mut Kern) -> Result<Unit, KernErr> {
        self.draw((0, 0), 0x00ff00, kern)?;
        Ok(msg)
    }
}

impl TermAct for Sprite {
    fn act(self, _term: &mut Term, _orig: &Msg, msg: Unit, kern: &mut Kern) -> Result<Unit, KernErr> {
        let w = self.img.size.0;
        let h = self.img.size.1;

        let x_offs = self.pos.0 - (w as i32 / 2);
        let y_offs = self.pos.1 - (h as i32 / 2);

        self.img.draw((x_offs, y_offs), 0x00ff00, kern)?;
        Ok(msg)
    }
}

impl TermAct for Win {
    fn act(self, term: &mut Term, _orig: &Msg, msg: Unit, kern: &mut Kern) -> Result<Unit, KernErr> {
        // render
        if self.border {
            match self.mode {
                Mode::Cli => {
                    let res = match term.mode {
                        Mode::Cli => kern.cli.res().map_err(|e| KernErr::CLIErr(e))?,
                        Mode::Gfx => {
                            let res = kern.disp.res().map_err(|e| KernErr::DispErr(e))?;
                            (res.0 / 8, res.1 / 16)
                        }
                    };

                    let size = self.size.unwrap_or(res);
                    let pos = self.pos.unwrap_or((0, 0));

                    self.ui_act(pos, size, term, kern)?;

                    if let Mode::Gfx = term.mode {
                        kern.disp.flush().map_err(|e| KernErr::DispErr(e))?;
                    }
                },
                Mode::Gfx => {
                    let res = kern.disp.res().map_err(|e| KernErr::DispErr(e))?;

                    let size = self.size.unwrap_or(res);
                    let pos = self.pos.unwrap_or((0, 0));

                    self.ui_gfx_act(pos, size, term, kern)?;

                    kern.disp.flush().map_err(|e| KernErr::DispErr(e))?;
                }
            }
        } else {
            term.clear(kern)?;
        }

        let res = kern.disp.res().map_err(|e| KernErr::DispErr(e))?;
        let mut mouse_pos = ((res.0 / 2) as i32, (res.1 / 2) as i32);

        let skin = Skin {
            cursor: Img {
                size: (32, 32),
                img: Vec::from(content::CURSOR)
            }
        };

        if let Mode::Gfx = self.mode {
            skin.cursor.draw(mouse_pos, 0, kern)?;
            kern.disp.flush_blk(mouse_pos, skin.cursor.size).map_err(|e| KernErr::DispErr(e))?;
        }

        loop {
            if let Mode::Gfx = self.mode {
                // render
                if self.border {
                    match self.mode {
                        Mode::Cli => {
                            let res = match term.mode {
                                Mode::Cli => kern.cli.res().map_err(|e| KernErr::CLIErr(e))?,
                                Mode::Gfx => {
                                    let res = kern.disp.res().map_err(|e| KernErr::DispErr(e))?;
                                    (res.0 / 8, res.1 / 16)
                                }
                            };

                            let size = self.size.unwrap_or(res);
                            let pos = self.pos.unwrap_or((0, 0));
        
                            self.ui_act(pos, size, term, kern)?;
                        },
                        Mode::Gfx => {
                            let res = kern.disp.res().map_err(|e| KernErr::DispErr(e))?;
        
                            let size = self.size.unwrap_or(res);
                            let pos = self.pos.unwrap_or((0, 0));
        
                            self.ui_gfx_act(pos, size, term, kern)?;
                        }
                    }
                } else {
                    term.clear(kern)?;
                }

                // mouse
                let mouse = kern.disp.mouse(false).map_err(|e| KernErr::DispErr(e))?;
                if let Some(mouse) = mouse {
                    kern.disp.flush_blk(mouse_pos, skin.cursor.size).map_err(|e| KernErr::DispErr(e))?;

                    mouse_pos.0 += mouse.dpos.0 / 4096;
                    mouse_pos.1 += mouse.dpos.1 / 4096;

                    skin.cursor.draw(mouse_pos, 0, kern)?;
                    kern.disp.flush_blk(mouse_pos, skin.cursor.size).map_err(|e| KernErr::DispErr(e))?;
                }
            }

            // wait for esc key
            if let Some(key) = term.get_key(false, kern)? {
                if let TermKey::Esc = key{
                    break;
                }
            }
        }

        term.clear(kern)?;
        Ok(msg)
    }
}

impl UIAct for Win {
    fn ui_act(&self, pos: (i32, i32), size:(usize, usize), term: &mut Term, kern: &mut Kern) -> Result<(), KernErr> {
        if self.border {
            for x in 0..size.0 {
                for y in 0..size.1 {
                    let ch = if x == 0 && y == 0 {
                        '┌'
                    } else if x == 0 && y == size.1 - 1 {
                        '└'
                    } else if x == size.0 - 1 && y == 0 {
                        '┐'
                    } else if x == size.0 - 1 && y == size.1 - 1 {
                        '┘'
                    } else if y == 0 || y == size.1 - 1 {
                        '─'
                    } else if x == 0 || x == size.0 - 1 {
                        '│'
                    } else {
                        ' '
                    };

                    term.print_glyth(ch, ((pos.0 as usize + x) * 8, (pos.1 as usize + y) * 16), 0x00ff00, kern)?;
                }
            }

            if let Some(title) = &self.title {
                for (i, ch) in title.chars().enumerate() {
                    let offs = pos.0 as usize + (size.0 - title.len()) / 2;
                    term.print_glyth(ch, ((offs + i) * 8, (pos.1 as usize) * 16), 0x00ff00, kern)?;
                }
            }
        }

        if let Some(ui) = &self.content {
            ui.ui_act((pos.0 + 1, pos.1 + 1), (size.0 - 2, size.1 - 2), term, kern)?;
        }

        Ok(())
    }

    fn ui_gfx_act(&self, pos: (i32, i32), size:(usize, usize), term: &mut Term, kern: &mut Kern) -> Result<(), KernErr> {
        {
            let mut tmp = Vec::with_capacity(size.0 * size.1);
    
            for y in 0..size.1 {
                for x in 0..size.0 {
                    let col = if x < 4 || x >= size.0 - 4 || y < 16 || y >= size.1 - 4 {
                        0x2a2e32
                    } else {
                        0x18191d
                    };
                    tmp.push(col);
                }
            }

            kern.disp.blk(pos, size, 0, tmp.as_slice()).map_err(|e| KernErr::DispErr(e))?;
        }

        if self.border {
            {
                let mut tmp = Vec::with_capacity(size.0 * 16);

                for _ in 0..16 {
                    for _ in 0..size.0 {
                        tmp.push(0x2a2e32);
                    }
                }

                kern.disp.blk(pos, (size.0, 16), 0, tmp.as_slice()).map_err(|e| KernErr::DispErr(e))?;
            }

            if let Some(title) = &self.title {
                for (i, ch) in title.chars().enumerate() {
                    let offs = pos.0 as usize + (size.0 - title.len() * 8) / 2;
                    term.print_glyth(ch, (offs + i * 8, pos.1 as usize), 0, kern)?;
                }
            }
        }

        if let Some(ui) = &self.content {
            ui.ui_gfx_act((pos.0 + 4, pos.1 + 16), (size.0 - 8, size.1 - 16), term, kern)?;
        }

        Ok(())
    }
}

impl UIAct for UI {
    fn ui_act(&self, pos: (i32, i32), size:(usize, usize), term: &mut Term, kern: &mut Kern) -> Result<(), KernErr> {
        match self {
            UI::HStack(hstack) => {
                for (i, ui) in hstack.iter().enumerate() {
                    let size = (size.0 / hstack.len(), size.1);
                    let pos = (pos.0 + (i * size.0) as i32, pos.1);

                    ui.ui_act(pos, size, term, kern)?;
                }
            },
            UI::VStack(vstack) => {
                for (i, ui) in vstack.iter().enumerate() {
                    let size = (size.0, size.1 / vstack.len());
                    let pos = (pos.0, pos.1 + (i * size.1) as i32);

                    ui.ui_act(pos, size, term, kern)?;
                }
            },
            UI::Win(win) => return win.ui_act(pos, size, term, kern)
        }
        Ok(())
    }

    fn ui_gfx_act(&self, pos: (i32, i32), size:(usize, usize), term: &mut Term, kern: &mut Kern) -> Result<(), KernErr> {
        match self {
            UI::HStack(hstack) => {
                for (i, ui) in hstack.iter().enumerate() {
                    let size = (size.0 / hstack.len(), size.1);
                    let pos = (pos.0 + (i * size.0) as i32, pos.1);

                    ui.ui_gfx_act(pos, size, term, kern)?;
                }
            },
            UI::VStack(vstack) => {
                for (i, ui) in vstack.iter().enumerate() {
                    let size = (size.0, size.1 / vstack.len());
                    let pos = (pos.0, pos.1 + (i * size.1) as i32);

                    ui.ui_gfx_act(pos, size, term, kern)?;
                }
            },
            UI::Win(win) => return win.ui_gfx_act(pos, size, term, kern)
        }
        Ok(())
    }
}
