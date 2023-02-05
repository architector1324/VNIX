use alloc::vec::Vec;

use crate::vnix::core::msg::Msg;
use crate::vnix::core::kern::{KernErr, Kern};
use crate::vnix::core::unit::{Unit, FromUnit, SchemaMapEntry, SchemaInt, SchemaStr, SchemaUnit, Schema, SchemaMapRequire, SchemaPair, SchemaOr, SchemaSeq, Or};

use crate::vnix::utils;

use super::{TermAct, Term};


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
pub enum Tex {
    Color(u32),
    Img(Img),
    Vid(Video)
}

#[derive(Debug, Clone)]
struct VidFrameDiff {
    diff: Vec<((usize, usize), i32)>
}

#[derive(Debug, Clone)]
pub struct Video {
    img: Img,
    frames: Vec<VidFrameDiff>
}

impl Img {
    pub fn draw(&self, pos: (i32, i32), src: u32, kern: &mut Kern) -> Result<(), KernErr> {
        kern.disp.blk(pos, self.size, src, &self.img).map_err(|e| KernErr::DispErr(e))
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
                    let img0 = utils::decompress_bytes(s.as_str()).ok()?;
                    let img_u = Unit::parse_bytes(img0.iter()).ok()?.0.as_vec()?;

                    img_u.into_iter().filter_map(|u| u.as_int()).map(|v| v as u32).collect()
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


impl FromUnit for VidFrameDiff {
    fn from_unit_loc(u: &Unit) -> Option<Self> {
        Self::from_unit(u, u)
    }

    fn from_unit(glob: &Unit, u: &Unit) -> Option<Self> {
        let schm = SchemaOr(
            SchemaStr,
            SchemaSeq(
                SchemaPair(
                    SchemaPair(SchemaInt, SchemaInt),
                    SchemaInt
                )
            )
        );

        schm.find_deep(glob, u).and_then(|or| {
            let diff = match or {
                Or::First(s) => {
                    let diff0 = utils::decompress_bytes(s.as_str()).ok()?;
                    let diff_u = Unit::parse_bytes(diff0.iter()).ok()?.0.as_vec()?;

                    diff_u.into_iter().filter_map(|u| u.as_pair())
                        .filter_map(|(p, diff)| Some((p.as_pair()?, diff.as_int()?)))
                        .filter_map(|((x, y), diff)| Some(((x.as_int()?, y.as_int()?), diff)))
                        .map(|((x, y), diff)| ((x as usize, y as usize), diff)).collect()
                },
                Or::Second(seq) => seq.into_iter().map(|((x, y), diff)| ((x as usize, y as usize), diff)).collect()
            };
            Some(VidFrameDiff {
                diff
            })
        })
    }
}

impl FromUnit for Video {
    fn from_unit_loc(u: &Unit) -> Option<Self> {
        Self::from_unit(u, u)
    }

    fn from_unit(glob: &Unit, u: &Unit) -> Option<Self> {
        let schm = SchemaMapRequire(
            SchemaMapEntry(Unit::Str("img".into()), SchemaUnit),
            SchemaMapEntry(
                Unit::Str("fms".into()),
                SchemaOr(
                    SchemaStr,
                    SchemaSeq(SchemaUnit)
                )
            )
        );

        schm.find_deep(glob, u).and_then(|(img, or)| {
            let img = Img::from_unit(glob, &img)?;

            let frames = match or {
                Or::First(s) => {
                    let fms0 = utils::decompress(s.as_str()).ok()?;
                    let fms_s = utils::decompress(fms0.as_str()).ok()?;
                    let fms_u = Unit::parse(fms_s.chars()).ok()?.0.as_vec()?;

                    fms_u.into_iter().filter_map(|u| VidFrameDiff::from_unit(glob, &u)).collect()
                },
                Or::Second(seq) => seq.into_iter().filter_map(|u| VidFrameDiff::from_unit(glob, &u)).collect()
            };

            Some(Video {
                img,
                frames
            })
        })
    }
}

impl FromUnit for Tex {
    fn from_unit_loc(u: &Unit) -> Option<Self> {
        Self::from_unit(u, u)
    }

    fn from_unit(glob: &Unit, u: &Unit) -> Option<Self> {
        let schm = SchemaOr(
            SchemaStr,
            SchemaUnit
        );

        schm.find_deep(glob, &u).and_then(|or| {
            match or {
                Or::First(s) => Some(Tex::Color(utils::hex_to_u32(s.as_str())?)),
                Or::Second(u) => {
                    if let Some(img) = Img::from_unit(glob, &u) {
                        return Some(Tex::Img(img));
                    }

                    if let Some(vid) = Video::from_unit(glob, &u) {
                        return Some(Tex::Vid(vid));
                    }

                    None
                }
            }
        })
    }
}

impl TermAct for Img {
    fn act(self, _term: &mut Term, _orig: &Msg, msg: Unit, kern: &mut Kern) -> Result<Unit, KernErr> {
        let res = kern.disp.res().map_err(|e| KernErr::DispErr(e))?;
        let pos = (
            (res.0 - self.size.0) as i32 / 2,
            (res.1 - self.size.1) as i32 / 2
        );
        
        self.draw(pos, 0x00ff00, kern)?;
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

impl TermAct for Video {
    fn act(self, _term: &mut Term, _orig: &Msg, msg: Unit, kern: &mut Kern) -> Result<Unit, KernErr> {
        let res = kern.disp.res().map_err(|e| KernErr::DispErr(e))?;
        let pos = (
            (res.0 - self.img.size.0) as i32 / 2,
            (res.1 - self.img.size.1) as i32 / 2
        );

        // render first frame
        self.img.draw(pos, 0x00ff00, kern)?;
        kern.disp.flush_blk(pos, self.img.size).map_err(|e| KernErr::DispErr(e))?;

        // render next frames        
        let mut img = self.img.clone();

        for diff in self.frames {
            for ((x, y), diff) in diff.diff {
                if let Some(px) = img.img.get_mut(x + y * img.size.0) {
                    *px = (*px as i32 + diff) as u32;
                    // kern.disp.px(*px, (pos.0 + x as i32) as usize, (pos.1 + y as i32) as usize).map_err(|e| KernErr::DispErr(e))?;
                }
            }
            img.draw(pos, 0x00ff00, kern)?;
            kern.disp.flush_blk(pos, self.img.size).map_err(|e| KernErr::DispErr(e))?;

            kern.time.wait(30000).map_err(|e| KernErr::TimeErr(e))?;
        }

        Ok(msg)
    }
}
