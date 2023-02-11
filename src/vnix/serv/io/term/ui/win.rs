use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec::Vec;

use super::media;
use super::UIAct;

use crate::driver::TermKey;
use crate::vnix::core::msg::Msg;
use crate::vnix::core::kern::{KernErr, Kern};
use crate::vnix::core::unit::SchemaInt;
use crate::vnix::core::unit::SchemaMapRequire;
use crate::vnix::core::unit::SchemaPair;
use crate::vnix::core::unit::{Unit, FromUnit, SchemaMapSecondRequire, SchemaMapEntry, SchemaBool, SchemaStr, SchemaUnit, Schema, SchemaOr, Or};

use crate::vnix::utils;

use super::{TermAct, Mode, Term};


#[derive(Debug, Clone)]
pub struct WinFloating {
    pos: (i32, i32),
    size: (usize, usize),
}

#[derive(Debug, Clone)]
pub struct Win {
    title: Option<String>,
    border: bool,
    mode: Mode,

    border_col: u32,
    back_tex: media::Tex,

    floating: Option<WinFloating>,

    content: Option<Box<super::UI>>,
}


impl FromUnit for Win {
    fn from_unit_loc(u: &Unit) -> Option<Self> {
        Self::from_unit(u, u)
    }

    fn from_unit(glob: &Unit, u: &Unit) -> Option<Self> {
        let schm = SchemaMapSecondRequire(
            SchemaMapEntry(Unit::Str("brd".into()), SchemaBool),
            SchemaMapSecondRequire(
                SchemaMapEntry(Unit::Str("brd.col".into()), SchemaStr),
                SchemaMapSecondRequire(
                    SchemaMapEntry(Unit::Str("title".into()), SchemaStr),
                    SchemaMapSecondRequire(
                        SchemaMapEntry(Unit::Str("tex".into()), SchemaUnit),
                        SchemaMapSecondRequire(
                            SchemaMapEntry(
                                Unit::Str("flt".into()),
                                SchemaMapRequire(
                                    SchemaMapEntry(
                                        Unit::Str("pos".into()),
                                        SchemaPair(SchemaInt, SchemaInt)
                                    ),
                                    SchemaMapEntry(
                                        Unit::Str("size".into()),
                                        SchemaPair(SchemaInt, SchemaInt)
                                    )
                                )
                            ),
                            SchemaOr(
                                SchemaMapEntry(Unit::Str("win".into()), SchemaUnit),
                                SchemaMapEntry(Unit::Str("win.gfx".into()), SchemaUnit)
                            )
                        )
                    )
                )
            )
        );

        schm.find_deep(glob, u).map(|(brd, (brd_col, (title, (tex, (flt, or)))))| {
            let (mode, content) = match or {
                Or::First(u) => (Mode::Cli, u),
                Or::Second(u) => (Mode::Gfx, u)
            };

            let tex = tex.and_then(|u| media::Tex::from_unit(glob, &u));

            let flt = flt.map(|(pos, size)| {
                WinFloating {
                    pos: pos,
                    size: (size.0 as usize, size.1 as usize)
                }
            });

            Win {
                title,
                border: brd.unwrap_or(false),
                mode,

                border_col: brd_col.and_then(|col| utils::hex_to_u32(col.as_str())).unwrap_or(0x2a2e32),
                back_tex: tex.unwrap_or(media::Tex::Color(0x18191d)),

                floating: flt,
                
                content: super::UI::from_unit(glob, &content).map(|ui| Box::new(ui))
            }
        })
    }
}


impl TermAct for Win {
    fn act(mut self, term: &mut Term, _orig: &Msg, msg: Unit, kern: &mut Kern) -> Result<Unit, KernErr> {
        // render
        match self.mode {
            Mode::Cli => {
                let res = match term.mode {
                    Mode::Cli => kern.cli.res().map_err(|e| KernErr::CLIErr(e))?,
                    Mode::Gfx => {
                        let res = kern.disp.res().map_err(|e| KernErr::DispErr(e))?;
                        (res.0 / 8, res.1 / 16)
                    }
                };

                let (pos, size) = self.floating.as_ref().map(|flt| (flt.pos, flt.size)).unwrap_or(((0, 0), res));

                self.ui_act(pos, size, term, kern)?;

                if let Mode::Gfx = term.mode {
                    kern.disp.flush().map_err(|e| KernErr::DispErr(e))?;
                }
            },
            Mode::Gfx => {
                let res = kern.disp.res().map_err(|e| KernErr::DispErr(e))?;

                let (pos, size) = self.floating.as_ref().map(|flt| (flt.pos, flt.size)).unwrap_or(((0, 0), res));

                self.ui_gfx_act(pos, size, None, term, kern)?;

                kern.disp.flush().map_err(|e| KernErr::DispErr(e))?;
            }
        }

        let res = kern.disp.res().map_err(|e| KernErr::DispErr(e))?;
        let mut mouse_pos = ((res.0 / 2) as i32, (res.1 / 2) as i32);
        let mut mouse_click = (false, false);

        if let Mode::Gfx = self.mode {
            term.res.cur.draw(mouse_pos, 0, kern)?;
            kern.disp.flush_blk(mouse_pos, term.res.cur.size).map_err(|e| KernErr::DispErr(e))?;
        }

        loop {
            if let Mode::Gfx = self.mode {
                // render
                match self.mode {
                    Mode::Cli => {
                        let res = match term.mode {
                            Mode::Cli => kern.cli.res().map_err(|e| KernErr::CLIErr(e))?,
                            Mode::Gfx => {
                                let res = kern.disp.res().map_err(|e| KernErr::DispErr(e))?;
                                (res.0 / 8, res.1 / 16)
                            }
                        };

                        let (pos, size) = self.floating.as_ref().map(|flt| (flt.pos, flt.size)).unwrap_or(((0, 0), res));
    
                        self.ui_act(pos, size, term, kern)?;
                    },
                    Mode::Gfx => {
                        let res = kern.disp.res().map_err(|e| KernErr::DispErr(e))?;

                        let (pos, size) = self.floating.as_ref().map(|flt| (flt.pos, flt.size)).unwrap_or(((0, 0), res));
    
                        if self.ui_gfx_act(pos, size, Some((mouse_pos, mouse_click)), term, kern)? {
                            term.res.cur.draw(mouse_pos, 0, kern)?;
                            kern.disp.flush_blk(pos, size).map_err(|e| KernErr::DispErr(e))?;
                        }
                    }
                }

                // mouse
                let mouse = kern.disp.mouse(false).map_err(|e| KernErr::DispErr(e))?;
                if let Some(mouse) = mouse {
                    kern.disp.flush_blk(mouse_pos, term.res.cur.size).map_err(|e| KernErr::DispErr(e))?;

                    mouse_pos.0 += mouse.dpos.0 / 4096;
                    mouse_pos.1 += mouse.dpos.1 / 4096;

                    mouse_click = mouse.click;

                    term.res.cur.draw(mouse_pos, 0, kern)?;
                    kern.disp.flush_blk(mouse_pos, term.res.cur.size).map_err(|e| KernErr::DispErr(e))?;
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
    fn ui_act(&mut self, pos: (i32, i32), size:(usize, usize), term: &mut Term, kern: &mut Kern) -> Result<(), KernErr> {
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

        if let Some(ui) = &mut self.content {
            ui.ui_act((pos.0 + 1, pos.1 + 1), (size.0 - 2, size.1 - 2), term, kern)?;
        }

        Ok(())
    }

    fn ui_gfx_act(&mut self, mut pos: (i32, i32), mut size:(usize, usize), mouse: Option<((i32, i32), (bool, bool))>, term: &mut Term, kern: &mut Kern) -> Result<bool, KernErr> {
        let mut flush = false;

        if let Some(flt) = &mut self.floating {
            if let Some((_pos, click)) = mouse {
                if click.0 && _pos.0 >= flt.pos.0 && _pos.0 < flt.pos.0 + flt.size.0 as i32 && _pos.1 >= flt.pos.1 && _pos.1 < flt.pos.1 + 16 as i32 {
                    flt.pos.0 = _pos.0 - flt.size.0 as i32 / 2;
                    flt.pos.1 = _pos.1 - 8;
                    flush = true;
                }
            }

            pos = flt.pos;
            size = flt.size;
        }

        let mut tmp = Vec::with_capacity(size.0 * size.1);

        let img = if let media::Tex::Vid(vid) = &mut self.back_tex {
            flush = true;
            vid.next()
        } else {
            None
        };

        for y in 0..size.1 {
            for x in 0..size.0 {
                let col = if self.border && (x < 4 || x >= size.0 - 4 || y < 16 || y >= size.1 - 4) {
                    self.border_col
                } else {
                    match &mut self.back_tex {
                        media::Tex::Color(col) => *col,
                        media::Tex::Img(img) => *img.img.get(x * img.size.0 / size.0 + y * img.size.1 / size.1 * img.size.0).unwrap_or(&0x18191d),
                        media::Tex::Vid(..) =>
                            match &img {
                                Some(img) => *img.img.get(x * img.size.0 / size.0 + y * img.size.1 / size.1 * img.size.0).unwrap_or(&0x18191d),
                                None => 0x18191d
                            }
                    }
                };
                tmp.push(col);
            }
        }

        kern.disp.blk(pos, size, 0, tmp.as_slice()).map_err(|e| KernErr::DispErr(e))?;

        if self.border {
            if let Some(title) = &self.title {
                for (i, ch) in title.chars().enumerate() {
                    let offs = pos.0 as usize + (size.0 - title.len() * 8) / 2;
                    term.print_glyth(ch, (offs + i * 8, pos.1 as usize), 0, kern)?;
                }
            }
        }

        if let Some(ui) = &mut self.content {
            ui.ui_gfx_act((pos.0 + 4, pos.1 + 16), (size.0 - 8, size.1 - 16), mouse, term, kern)?;
        }

        Ok(flush)
    }
}
