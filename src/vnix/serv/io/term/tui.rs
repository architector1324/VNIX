use core::pin::Pin;
use core::ops::{Generator, GeneratorState};

use spin::Mutex;
use alloc::rc::Rc;

use alloc::vec::Vec;
use alloc::boxed::Box;
use alloc::string::String;

use crate::driver::{CLIErr, TermKey};
use crate::vnix::core::msg::Msg;
use crate::vnix::core::unit::{Unit, FromUnit, SchemaStr, Schema, SchemaMapEntry, SchemaUnit, SchemaOr, Or, SchemaMapSecondRequire};
use crate::vnix::core::kern::{Kern, KernErr};


use super::{TermAct, ActMode};


trait TUIBorder {
    fn draw(&self, pos: (usize, usize), size: (usize, usize), set: [char; 6], mode: ActMode, term: Rc<super::Term>, kern: &mut Kern) -> Result<(), CLIErr> {
        // corners
        term.print_glyth(set[0], (pos.0, pos.1), 0, &mode, kern)?;
        term.print_glyth(set[1], (pos.0 + size.0 - 8, pos.1), 0, &mode, kern)?;
        term.print_glyth(set[2], (pos.0, size.1 - 16 + pos.1), 0, &mode, kern)?;

        if let ActMode::Gfx = mode {
            term.print_glyth(set[3], (pos.0 + size.0 - 8, pos.1 + size.1 - 16), 0, &mode, kern)?;
        }

        // borders
        for x in (8..(size.0 - 8)).step_by(8) {
            term.print_glyth(set[4], (pos.0 + x, pos.1), 0, &mode, kern)?;
            term.print_glyth(set[4], (pos.0 + x, pos.1 + size.1 - 16), 0, &mode, kern)?;
        }

        for y in (16..(size.1 - 16)).step_by(16) {
            term.print_glyth(set[5], (pos.0, pos.1 + y), 0, &mode, kern)?;
            term.print_glyth(set[5], (pos.0 + size.0 - 8, pos.1 + y), 0, &mode, kern)?;
        }

        Ok(())
    }
}

type TUIActAsync<'a> = Box<dyn Generator<Yield = (), Return = Result<(), KernErr>> + 'a>;

trait TUIAct {
    fn tui_act<'a>(self, pos: (usize, usize), size:(usize, usize), term: Rc<super::Term>, kern: &'a Mutex<Kern>) -> TUIActAsync<'a>;
}

#[derive(Debug, Clone)]
pub struct Win {
    title: Option<String>,
    pub mode: ActMode,
    border: [char; 6],
    content: Option<TUI>
}

#[derive(Debug, Clone)]
enum TUI {
    VStack(Vec<TUI>),
    HStack(Vec<TUI>),
    Win(Box<Win>),
}


impl FromUnit for TUI {
    fn from_unit_loc(u: &Unit) -> Option<Self> {
        Self::from_unit(u, u)
    }

    fn from_unit(_glob: &Unit, _u: &Unit) -> Option<Self> {
        None
    }
}

impl FromUnit for Win {
    fn from_unit_loc(u: &Unit) -> Option<Self> {
        Self::from_unit(u, u)
    }

    fn from_unit(glob: &Unit, u: &Unit) -> Option<Self> {
        let schm = SchemaMapSecondRequire(
            SchemaMapEntry(Unit::Str("name".into()), SchemaStr),
            SchemaMapSecondRequire(
                SchemaMapEntry(Unit::Str("brd".into()), SchemaStr),
                SchemaOr(
                    SchemaMapEntry(Unit::Str("win.cli".into()), SchemaUnit),
                    SchemaMapEntry(Unit::Str("win.cli.gfx".into()), SchemaUnit),
                )
            )
        );

        schm.find_deep(glob, u).and_then(|(name, (brd, or))| {
            let (content, mode) = match or {
                Or::First(content) => (FromUnit::from_unit(glob, &content), ActMode::Cli),
                Or::Second(content) => (FromUnit::from_unit(glob, &content), ActMode::Gfx)
            };

            let set_default = match mode {
                ActMode::Cli => "┌┐└┘─│",
                ActMode::Gfx => "╭╮╰╯─│"
            };

            let border = brd.unwrap_or(set_default.into()).chars().collect::<Vec<_>>().try_into().ok()?;

            Some(Win {
                title: name,
                mode,
                border,
                content
            })
        })
    }
}

impl TUIBorder for Win {}

impl TUIAct for Win {
    fn tui_act<'a>(self, pos: (usize, usize), size:(usize, usize), term: Rc<super::Term>, kern: &'a Mutex<Kern>) -> TUIActAsync<'a> {
        let hlr = move || {
            let mut redraw = true;
            loop {
                // render
                if redraw {
                    self.draw(pos, size, self.border, self.mode.clone(), term.clone(), &mut kern.lock()).map_err(|e| KernErr::CLIErr(e))?;

                    // title
                    if let Some(title) = &self.title {
                        for (i, ch) in title.chars().enumerate() {
                            term.print_glyth(ch, (pos.0 + (size.0 - title.len() * 8) / 2 + i * 8, pos.1), 0, &self.mode, &mut kern.lock()).map_err(|e| KernErr::CLIErr(e))?;
                        }
                    }
                    yield;

                    if let ActMode::Gfx = self.mode {
                        kern.lock().drv.disp.flush_blk((pos.0 as i32, pos.1 as i32), size).map_err(|e| KernErr::DispErr(e))?;
                    }
                    yield;

                    redraw = false;
                }
                yield;
            }
        };
        Box::new(hlr)
    }
}

impl TermAct for Win {
    fn act<'a>(self, _orig: Rc<Msg>, msg: Unit, term: Rc<super::Term>, kern: &'a Mutex<Kern>) -> super::TermActAsync<'a> {
        let hlr = move || {
            // clear screen
            term.clear(&self.mode, &mut kern.lock()).map_err(|e| KernErr::CLIErr(e))?;
            yield;

            let size = match self.mode {
                ActMode::Cli => {
                    let res = kern.lock().drv.cli.res().map_err(|e| KernErr::CLIErr(e))?;
                    (res.0 * 8, res.1 * 16)
                },
                ActMode::Gfx => kern.lock().drv.disp.res().map_err(|e| KernErr::DispErr(e))?,
            };

            let mut gen = Box::into_pin(self.tui_act((0, 0), size, term.clone(), kern));

            loop {
                if let GeneratorState::Complete(res) = Pin::new(&mut gen).resume(()) {
                    res?;
                    break;
                }

                // wait for key
                if let Some(key) = term.get_key(&mut kern.lock()).map_err(|e| KernErr::CLIErr(e))? {
                    if TermKey::Esc == key {
                        break;
                    }
                }
                yield;
            }

            Ok(msg)
        };
        Box::new(hlr)
    }
}
