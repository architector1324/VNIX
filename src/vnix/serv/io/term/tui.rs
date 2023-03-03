use core::pin::Pin;
use core::fmt::Write;
use core::ops::{Generator, GeneratorState};

use spin::Mutex;
use alloc::rc::Rc;

use alloc::{vec, format};
use alloc::vec::Vec;
use alloc::boxed::Box;
use alloc::string::{String, ToString};

use crate::driver::{CLIErr, DispErr, TermKey, Duration};
use crate::vnix::core::msg::Msg;
use crate::vnix::core::task::TaskLoop;
use crate::vnix::core::unit::{Unit, FromUnit, SchemaStr, Schema, SchemaMapEntry, SchemaUnit, SchemaOr, SchemaSeq, Or, SchemaPair, SchemaRef, SchemaInt, SchemaMapRequire, SchemaMapFirstRequire, SchemaMapSecondRequire};
use crate::vnix::core::kern::{Kern, KernErr};


use super::{TermAct, ActMode};


#[derive(Debug, Clone)]
pub struct Win {
    title: Option<String>,
    pub mode: ActMode,
    border: [char; 6]
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
                Or::First(content) => (content, ActMode::Cli),
                Or::Second(content) => (content, ActMode::Gfx)
            };

            let set_default = match mode {
                ActMode::Cli => "┌┐└┘─│",
                ActMode::Gfx => "╭╮╰╯─│"
            };

            let border = brd.unwrap_or(set_default.into()).chars().collect::<Vec<_>>().try_into().ok()?;

            Some(Win {
                title: name,
                mode,
                border
            })
        })
    }
}

impl Win {
    fn draw(&self, pos: (usize, usize), size: (usize, usize), term: Rc<super::Term>, kern: &mut Kern) -> Result<(), CLIErr> {
        // corners
        term.print_glyth(self.border[0], (pos.0, pos.1), 0, &self.mode, kern)?;
        term.print_glyth(self.border[1], (pos.0 + size.0 - 8, pos.1), 0, &self.mode, kern)?;
        term.print_glyth(self.border[2], (pos.0, size.1 - 16 + pos.1), 0, &self.mode, kern)?;

        if let ActMode::Gfx = self.mode {
            term.print_glyth(self.border[3], (pos.0 + size.0 - 8, pos.1 + size.1 - 16), 0, &self.mode, kern)?;
        }

        // borders
        for x in (8..(size.0 - 8)).step_by(8) {
            term.print_glyth(self.border[4], (pos.0 + x, pos.1), 0, &self.mode, kern)?;
            term.print_glyth(self.border[4], (pos.0 + x, pos.1 + size.1 - 16), 0, &self.mode, kern)?;
        }

        for y in (16..(size.1 - 16)).step_by(16) {
            term.print_glyth(self.border[5], (pos.0, pos.1 + y), 0, &self.mode, kern)?;
            term.print_glyth(self.border[5], (pos.0 + size.0 - 8, pos.1 + y), 0, &self.mode, kern)?;
        }

        // title
        if let Some(title) = &self.title {
            for (i, ch) in title.chars().enumerate() {
                term.print_glyth(ch, (pos.0 + (size.0 - title.len() * 8) / 2 + i * 8, pos.1), 0, &self.mode, kern)?;
            }
        }

        Ok(())
    }
}

impl TermAct for Win {
    fn act<'a>(self, orig: Rc<Msg>, msg: Unit, term: Rc<super::Term>, kern: &'a Mutex<Kern>) -> super::TermActAsync<'a> {
        let hlr = move || {
            // clear screen
            term.clear(&self.mode, &mut kern.lock()).map_err(|e| KernErr::CLIErr(e))?;
            yield;

            let mut redraw = true;

            loop {
                // render
                let size = match self.mode {
                    ActMode::Cli => {
                        let res = kern.lock().drv.cli.res().map_err(|e| KernErr::CLIErr(e))?;
                        (res.0 * 8, res.1 * 16)
                    },
                    ActMode::Gfx => kern.lock().drv.disp.res().map_err(|e| KernErr::DispErr(e))?,
                };

                if redraw {
                    self.draw((0, 0), size, term.clone(), &mut kern.lock()).map_err(|e| KernErr::CLIErr(e))?;
                    yield;
                    
                    term.flush(&self.mode, &mut kern.lock()).map_err(|e| KernErr::DispErr(e))?;
                    yield;

                    redraw = false;
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
