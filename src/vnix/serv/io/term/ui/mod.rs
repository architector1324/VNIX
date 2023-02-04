pub mod text;
pub mod media;
pub mod win;

use alloc::vec::Vec;

use crate::vnix::core::kern::{KernErr, Kern};
use crate::vnix::core::unit::{Unit, FromUnit, SchemaMapEntry, SchemaUnit, Schema, SchemaOr, SchemaSeq, Or};

use super::{TermAct, Mode, Term};


trait UIAct {
    fn ui_act(&self, pos: (i32, i32), size:(usize, usize), term: &mut Term, kern: &mut Kern) -> Result<(), KernErr>;
    fn ui_gfx_act(&self, pos: (i32, i32), size:(usize, usize), term: &mut Term, kern: &mut Kern) -> Result<(), KernErr>;
}

#[derive(Debug, Clone)]
pub enum UI {
    VStack(Vec<UI>),
    HStack(Vec<UI>),
    Win(win::Win)
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
                    if let Some(win) = win::Win::from_unit(glob, &u) {
                        return Some(UI::Win(win));
                    }
                    None
                }
            }
        })
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
