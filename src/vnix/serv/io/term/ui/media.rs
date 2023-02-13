use core::iter::Cycle;

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
    Vid(Cycle<VideoIter>)
}

#[derive(Debug, Clone)]
struct VidFrameDiffRle16x16 {
    rle: Vec<(usize, i32)>
}

#[derive(Debug, Clone)]
struct VidFrameDiffRle16x16Iter {
    block: VidFrameDiffRle16x16,
    last: (usize, i32),
    idx: usize
}

#[derive(Debug, Clone)]
struct VidFrameDiff {
    diff: Vec<((usize, usize), VidFrameDiffRle16x16)>
}

#[derive(Debug, Clone)]
pub struct Video {
    img: Img,
    frames: Vec<VidFrameDiff>
}

#[derive(Debug, Clone)]
pub struct VideoIter {
    vid: Video,
    img: Img,

    idx: usize
}

impl Img {
    pub fn draw(&self, pos: (i32, i32), src: u32, kern: &mut Kern) -> Result<(), KernErr> {
        kern.disp.blk(pos, self.size, src, &self.img).map_err(|e| KernErr::DispErr(e))
    }
}

impl IntoIterator for Video {
    type Item = Img;
    type IntoIter = VideoIter;

    fn into_iter(self) -> Self::IntoIter {
        VideoIter {
            img: self.img.clone(),
            vid: self,
            idx: 0
        }
    }
}

impl IntoIterator for VidFrameDiffRle16x16 {
    type Item = i32;
    type IntoIter = VidFrameDiffRle16x16Iter;

    fn into_iter(self) -> Self::IntoIter {
        VidFrameDiffRle16x16Iter {
            block: self,
            last: (0, 0),
            idx: 0
        }
    }
}

impl Iterator for VidFrameDiffRle16x16Iter {
    type Item = i32;

    fn next(&mut self) -> Option<Self::Item> {
        if self.last.0 == 0 {
            self.last = self.block.rle.get(self.idx)?.clone();
            self.idx += 1;
        }

        self.last.0 -= 1;
        Some(self.last.1)
    }
}

impl Iterator for VideoIter {
    type Item = Img;

    fn next(&mut self) -> Option<Self::Item> {
        if self.idx >= self.vid.frames.len() {
            return None
        }

        let diff = self.vid.frames.get(self.idx)?;
        self.idx += 1;

        for ((block_x, block_y), diff) in diff.diff.clone() {
            let mut it = diff.into_iter();

            for y in 0..16 {
                for x in 0..16 {
                    if let Some(diff) = it.next() {
                        let idx = (x + block_x * 16) + (y + block_y * 16) * self.img.size.0;
                        if let Some(px) = self.img.img.get_mut(idx) {
                            *px = (*px as i64 + diff as i64) as u32;
                        }
                    }
                }
            }
        }

        Some(self.img.clone())
    }
}

impl VidFrameDiffRle16x16 {
    pub fn parse_bytes<I>(mut it: I) -> Option<(Self, I)> where I: Iterator<Item = u8> {
        let bytes = [
            it.next()?,
            it.next()?,
        ];

        let len = u16::from_le_bytes(bytes);

        let mut rle = Vec::new();

        for _ in 0..len {
            let bytes = [
                it.next()?,
                it.next()?,
            ];

            let cnt = u16::from_le_bytes(bytes);

            let bytes = [
                it.next()?,
                it.next()?,
                it.next()?,
                it.next()?,
            ];

            let diff = i32::from_le_bytes(bytes);

            rle.push((cnt as usize, diff));
        }

        Some((VidFrameDiffRle16x16{rle}, it))
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
                    let mut it = utils::decompress_bytes(s.as_str()).ok()?.into_iter();

                    let bytes = [
                        it.next()?,
                        it.next()?,
                    ];

                    let len = u16::from_le_bytes(bytes);
                    let mut tmp = Vec::with_capacity(len as usize);

                    for _ in 0..len {
                        let bytes = [
                            it.next()?,
                            it.next()?,
                        ];

                        let x = u16::from_le_bytes(bytes);

                        let bytes = [
                            it.next()?,
                            it.next()?,
                        ];

                        let y = u16::from_le_bytes(bytes);

                        let (diff, tmp_it) = VidFrameDiffRle16x16::parse_bytes(it)?;
                        it = tmp_it;

                        tmp.push(((x as usize, y as usize), diff));
                    }

                    tmp
                },
                Or::Second(seq) => todo!() //seq.into_iter().map(|((x, y), diff)| ((x as usize, y as usize), diff)).collect()
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
                        return Some(Tex::Vid(vid.into_iter().cycle()));
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
        let mut it = self.into_iter();

        while let Some(img) = it.next() {
            img.draw(pos, 0x00ff00, kern)?;
            kern.disp.flush_blk(pos, img.size).map_err(|e| KernErr::DispErr(e))?;

            kern.time.wait(30000).map_err(|e| KernErr::TimeErr(e))?;
        }

        Ok(msg)
    }
}
