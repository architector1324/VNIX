use alloc::rc::Rc;
use core::cell::RefCell;

use alloc::vec::Vec;
use alloc::boxed::Box;

use crate::driver::DispErr;
use crate::vnix::core::msg::Msg;
use crate::vnix::core::kern::{KernErr, Kern};
use crate::vnix::core::unit::{Unit, FromUnit, SchemaMapEntry, SchemaInt, SchemaStr, Schema, SchemaMapRequire, SchemaPair, SchemaOr, SchemaSeq, Or, SchemaUnit, SchemaMapSeq};

use crate::vnix::utils;

use super::{TermAct, Term, TermActAsync, ActMode};


trait ParseBytes: Sized {
    fn parse_bytes<'a, I>(it: I) -> Option<(Self, I)> where I: Iterator<Item = &'a u8>;
}

#[derive(Debug, Clone)]
enum ImgFmt {
    Rgb,
    Rgba,
    RgbRLE,
    RgbaRLE
}

#[derive(Debug, Clone)]
enum Pixels {
    Rgb(Vec<(u8, u8, u8)>),
    Rgba(Vec<u32>),
    RgbRLE(Vec<(usize, (u8, u8, u8))>),
    RgbaRLE(Vec<(usize, u32)>)
}

#[derive(Debug, Clone)]
pub struct Img {
    size: (usize, usize),
    img: Pixels,
    _cache: Option<Vec<u32>>
}

#[derive(Debug, Clone)]
pub struct Sprite {
    pub pos: (i32, i32),
    pub img: Img
}

#[derive(Debug, Clone)]
struct VidFrameDiffRle16x16 {
    rle: Vec<(u16, u32)>
}

#[derive(Debug, Clone)]
struct VidFrameDiffRle16x16Iter<'a> {
    block: &'a VidFrameDiffRle16x16,
    pallete: &'a VidDiffPallete,
    last: (u16, i32),
    idx: usize
}

#[derive(Debug, Clone)]
struct VidFrameDiff {
    diff: Vec<((u16, u16), u32)>
}

#[derive(Debug, Clone)]
struct VidDiffPallete {
    pal: Vec<i32>
}

#[derive(Debug, Clone)]
pub struct Video {
    img: Img,
    frames: Vec<VidFrameDiff>,
    blocks: Vec<VidFrameDiffRle16x16>,
    pallete: VidDiffPallete
}

#[derive(Debug, Clone)]
pub struct VideoIter {
    vid: Video,
    img: Rc<RefCell<Img>>,

    idx: usize
}


impl Img {
    fn draw(&mut self, pos: (i32, i32), src: u32, kern: &mut Kern) -> Result<(), DispErr> {
        let size = self.size.clone();
        kern.drv.disp.blk(pos, size, src, &self.get_pixels()?)?;

        Ok(())
    }

    fn get_pixels(&mut self) -> Result<&mut Vec<u32>, DispErr> {
        self.cache();
        self._cache.as_mut().ok_or(DispErr::SetPixel)
    }

    fn cache(&mut self) {
        if self._cache.is_none() {
            let img = match &self.img {
                Pixels::Rgb(dat) => dat.iter().cloned().map(|(r, g, b)| u32::from_le_bytes([r, g, b, 0])).collect(),
                Pixels::Rgba(dat) => dat.clone(),
                Pixels::RgbRLE(rle) => rle.iter().cloned().flat_map(|(cnt, (r, g, b))| (0..cnt).map(|_| u32::from_le_bytes([r, g, b, 0])).collect::<Vec<_>>()).collect(),
                Pixels::RgbaRLE(rle) => rle.iter().cloned().flat_map(|(cnt, px)| (0..cnt).map(|_| px).collect::<Vec<_>>()).collect()
            };

            self._cache = Some(img);
        }
    }
}

impl IntoIterator for Video {
    type Item = Rc<RefCell<Img>>;
    type IntoIter = VideoIter;

    fn into_iter(self) -> Self::IntoIter {
        VideoIter {
            img: Rc::new(RefCell::new(self.img.clone())),
            vid: self,
            idx: 0
        }
    }
}

impl VidFrameDiffRle16x16 {
    fn into_iter<'a>(&'a self, pal: &'a VidDiffPallete) -> VidFrameDiffRle16x16Iter<'a> {
        VidFrameDiffRle16x16Iter {
            block: &self,
            pallete: pal,
            last: (0, 0),
            idx: 0
        }
    }
}

impl<'a> Iterator for VidFrameDiffRle16x16Iter<'a> {
    type Item = i32;

    fn next(&mut self) -> Option<Self::Item> {
        if self.last.0 == 0 {
            let (cnt, id) = self.block.rle.get(self.idx)?.clone();
            self.last = (cnt, *self.pallete.pal.get(id as usize)?);
            self.idx += 1;
        }

        self.last.0 -= 1;
        Some(self.last.1)
    }
}

impl Iterator for VideoIter {
    type Item = Rc<RefCell<Img>>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.idx >= self.vid.frames.len() + 1 {
            return None
        }

        if self.idx == 0 {
            self.idx += 1;
            return Some(self.img.clone())
        }

        let diff = self.vid.frames.get(self.idx - 1)?;
        self.idx += 1;

        let size = self.img.borrow().size.clone();

        let mut tmp = self.img.borrow_mut();
        let img = tmp.get_pixels().ok()?;

        for ((block_x, block_y), diff) in &diff.diff {
            let mut it = self.vid.blocks.get(*diff as usize)?.into_iter(&self.vid.pallete);

            for y in 0..16 {
                for x in 0..16 {
                    if let Some(diff) = it.next() {
                        let idx = (x + (*block_x as usize) * 16) + (y + (*block_y as usize) * 16) * size.0;
                        if let Some(px) = img.get_mut(idx) {
                            *px = (*px as i64 + diff as i64) as u32;
                        }
                    }
                }
            }
        }

        Some(self.img.clone())
    }
}

impl ParseBytes for VidFrameDiff {
    fn parse_bytes<'a, I>(mut it: I) -> Option<(Self, I)> where I: Iterator<Item = &'a u8> {
        let len = u16::from_le_bytes([*it.next()?, *it.next()?]);
        let mut diff = Vec::with_capacity(len as usize);

        for _ in 0..len {
            let x = u16::from_le_bytes([*it.next()?, *it.next()?]);
            let y = u16::from_le_bytes([*it.next()?, *it.next()?]);
            let id = u32::from_le_bytes([*it.next()?, *it.next()?, *it.next()?, 0]);

            diff.push(((x, y), id));
        }
        diff.shrink_to_fit();

        Some((VidFrameDiff{diff}, it))
    }
}

impl ParseBytes for VidFrameDiffRle16x16 {
    fn parse_bytes<'a, I>(mut it: I) -> Option<(Self, I)> where I: Iterator<Item = &'a u8> {
        let len = u16::from_le_bytes([*it.next()?, *it.next()?]);
        let mut rle = Vec::with_capacity(len as usize);

        for _ in 0..len {
            let cnt = u16::from_le_bytes([*it.next()?, *it.next()?]);
            let id = u32::from_le_bytes([*it.next()?, *it.next()?,*it.next()?, 0]);

            rle.push((cnt, id));
        }
        rle.shrink_to_fit();

        Some((VidFrameDiffRle16x16{rle}, it))
    }
}

impl ParseBytes for VidDiffPallete {
    fn parse_bytes<'a, I>(mut it: I) -> Option<(Self, I)> where I: Iterator<Item = &'a u8> {
        let len = u32::from_le_bytes([*it.next()?, *it.next()?, *it.next()?, 0]);
        let mut pal = Vec::with_capacity(len as usize);

        for _ in 0..len {
            let dpx = i32::from_le_bytes([*it.next()?, *it.next()?, *it.next()?, *it.next()?]);
            pal.push(dpx);
        }
        pal.shrink_to_fit();

        Some((VidDiffPallete{pal}, it))
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
            SchemaMapRequire(
                SchemaMapEntry(Unit::Str("fmt".into()), SchemaStr),
                SchemaMapEntry(
                    Unit::Str("img".into()),
                    SchemaOr(
                        SchemaStr,
                        SchemaOr(
                            SchemaSeq(
                                SchemaPair(SchemaInt, SchemaInt)
                            ),
                            SchemaSeq(SchemaInt)
                        )
                    )
                )
            )
        );

        schm.find_deep(glob, u).and_then(|((w, h), (fmt, or))| {
            let fmt = match fmt.as_str() {
                "rgb" => ImgFmt::Rgb,
                "rgba" => ImgFmt::Rgba,
                "rgb.rle" => ImgFmt::RgbRLE,
                "rgba.rle" => ImgFmt::RgbaRLE,
                _ => return None
            };

            let img = match or {
                Or::First(s) => {
                    let img0 = utils::decompress_bytes(s.as_str()).ok()?;

                    match fmt {
                        ImgFmt::Rgb => Pixels::Rgb(img0.array_chunks::<3>().map(|ch| (ch[0], ch[1], ch[2])).collect()),
                        ImgFmt::Rgba => Pixels::Rgba(img0.array_chunks::<4>().map(|ch| u32::from_le_bytes([ch[0], ch[1], ch[2], ch[3]])).collect()),
                        ImgFmt::RgbRLE => Pixels::RgbRLE(img0.array_chunks::<6>().map(|ch| {
                            let cnt = u32::from_le_bytes([ch[0], ch[1], ch[2], 0]);
                            (cnt as usize, (ch[3], ch[4], ch[5]))
                        }).collect()),
                        ImgFmt::RgbaRLE => Pixels::RgbaRLE(img0.array_chunks::<7>().map(|ch| {
                            let cnt = u32::from_le_bytes([ch[0], ch[1], ch[2], 0]);
                            let px = u32::from_le_bytes([ch[3], ch[4], ch[5], ch[6]]);
                            (cnt as usize, px)
                        }).collect())
                    }
                },
                Or::Second(or) =>
                    match or {
                        Or::First(rle) =>
                            match fmt {
                                ImgFmt::RgbRLE => Pixels::RgbRLE(rle.into_iter().map(|(cnt, px)| {
                                    let b = px.to_le_bytes();
                                    (cnt as usize, (b[0], b[1], b[2]))
                                }).collect()),
                                ImgFmt::RgbaRLE => Pixels::RgbaRLE(rle.into_iter().map(|(cnt, px)| (cnt as usize, px as u32)).collect()),
                                _ => return None
                            }
                        Or::Second(img) =>
                            match fmt {
                                ImgFmt::Rgb => Pixels::Rgb(img.into_iter().map(|px| {
                                    let b = px.to_le_bytes();
                                    (b[0], b[1], b[2])
                                }).collect()),
                                ImgFmt::Rgba => Pixels::Rgba(img.into_iter().map(|px| px as u32).collect()),
                                _ => return None
                            }
                    }
            };

            Some(Img {
                size: (w as usize, h as usize),
                img,
                _cache: None
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
                Unit::Str("spr".into()),
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

impl FromUnit for Video {
    fn from_unit_loc(u: &Unit) -> Option<Self> {
        Self::from_unit(u, u)
    }

    fn from_unit(glob: &Unit, u: &Unit) -> Option<Self> {
        let schm = SchemaMapRequire(
            SchemaMapEntry(Unit::Str("img".into()), SchemaUnit),
            SchemaMapRequire(
                SchemaMapEntry(
                    Unit::Str("fms".into()),
                    SchemaOr(
                        SchemaSeq(SchemaStr),
                        SchemaSeq(
                            SchemaMapSeq(
                                SchemaPair(SchemaInt, SchemaInt),
                                SchemaInt
                            )
                        )
                    )
                ),
                SchemaMapRequire(
                    SchemaMapEntry(
                        Unit::Str("pal".into()),
                        SchemaOr(
                            SchemaStr,
                            SchemaSeq(SchemaInt)
                        )
                    ),
                    SchemaMapEntry(
                        Unit::Str("blk".into()),
                        SchemaOr(
                            SchemaStr,
                            SchemaSeq(
                                SchemaSeq(
                                    SchemaPair(SchemaInt, SchemaInt)
                                )
                            )
                        )
                    )
                )
            )
        );

        schm.find_deep(glob, u).and_then(|(img, (fms, (pal, blk)))| {
            let img = Img::from_unit(glob, &img)?;

            let frames = match fms {
                Or::First(seq) => {
                    seq.into_iter().filter_map(|s| {
                        let fms_b = utils::decompress_bytes(s.as_str()).ok()?;
                        Some(VidFrameDiff::parse_bytes(fms_b.iter())?.0)
                    }).collect::<Vec<_>>()
                },
                Or::Second(seq) =>
                    seq.into_iter().map(|v| {
                        let diff = v.into_iter().map(|((x, y), id)| ((x as u16, y as u16), id as u32)).collect();
                        VidFrameDiff{diff}
                    }).collect()
            };

            let blocks = match blk {
                Or::First(s) => {
                    let blk_b = utils::decompress_bytes(s.as_str()).ok()?;
                    let mut it = blk_b.iter();

                    let len = u32::from_le_bytes([*it.next()?, *it.next()?, *it.next()?, 0]);

                    let mut out = Vec::with_capacity(len as usize);

                    for _ in 0..len {
                        let (block, tmp) = VidFrameDiffRle16x16::parse_bytes(it)?;
                        out.push(block);

                        it = tmp;
                    }
                    out.shrink_to_fit();
                    out
                },
                Or::Second(seq) =>
                    seq.into_iter().map(|v| {
                        let rle = v.into_iter().map(|(cnt, id)| (cnt as u16, id as u32)).collect();
                        VidFrameDiffRle16x16{rle}
                    }).collect()
            };

            let pallete = match pal {
                Or::First(s) => {
                    let pal_b = utils::decompress_bytes(s.as_str()).ok()?;
                    VidDiffPallete::parse_bytes(pal_b.iter())?.0
                },
                Or::Second(pal) => VidDiffPallete{pal}
            };

            Some(Video {
                img,
                frames,
                blocks,
                pallete
            })
        })
    }
}

impl TermAct for Img {
    fn act<'a>(mut self, _orig: Rc<Msg>, msg: Unit, term: Rc<Term>, kern: &'a spin::Mutex<Kern>) -> TermActAsync<'a> {
        let hlr = move || {
            let res = kern.lock().drv.disp.res().map_err(|e| KernErr::DispErr(e))?;
            let pos = (
                (res.0 - self.size.0) as i32 / 2,
                (res.1 - self.size.1) as i32 / 2
            );
            yield;

            self.draw(pos, 0, &mut kern.lock()).map_err(|e| KernErr::DispErr(e))?;
            yield;

            term.flush(&ActMode::Gfx, &mut kern.lock()).map_err(|e| KernErr::DispErr(e))?;

            Ok(msg)
        };
        TermActAsync(Box::new(hlr))
    }
}

impl TermAct for Sprite {
    fn act<'a>(mut self, _orig: Rc<Msg>, msg: Unit, term: Rc<Term>, kern: &'a spin::Mutex<Kern>) -> TermActAsync<'a> {
        let hlr = move || {
            let w = self.img.size.0;
            let h = self.img.size.1;

            let pos = (
                self.pos.0 - (w as i32 / 2),
                self.pos.1 - (h as i32 / 2)
            );

            self.img.draw(pos, 0, &mut kern.lock()).map_err(|e| KernErr::DispErr(e))?;
            yield;

            term.flush(&ActMode::Gfx, &mut kern.lock()).map_err(|e| KernErr::DispErr(e))?;

            Ok(msg)
        };
        TermActAsync(Box::new(hlr))
    }
}

impl TermAct for Video {
    fn act<'a>(self, _orig: Rc<Msg>, msg: Unit, _term: Rc<Term>, kern: &'a spin::Mutex<Kern>) -> TermActAsync<'a> {
        let hlr = move || {
            let res = kern.lock().drv.disp.res().map_err(|e| KernErr::DispErr(e))?;
            let pos = (
                (res.0 - self.img.size.0) as i32 / 2,
                (res.1 - self.img.size.1) as i32 / 2
            );

            // render frames
            let mut it = self.into_iter();

            while let Some(img) = it.next() {
                img.borrow_mut().draw(pos, 0x00ff00, &mut kern.lock()).map_err(|e| KernErr::DispErr(e))?;
                kern.lock().drv.disp.flush_blk(pos, img.borrow().size).map_err(|e| KernErr::DispErr(e))?;

                // kern.lock().drv.time.wait(15000).map_err(|e| KernErr::TimeErr(e))?;
                yield;
            }

            Ok(msg)
        };
        TermActAsync(Box::new(hlr))
    }
}