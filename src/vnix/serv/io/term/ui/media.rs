use alloc::vec::Vec;
use alloc::vec;

use crate::vnix::core::msg::Msg;
use crate::vnix::core::kern::{KernErr, Kern};
use crate::vnix::core::unit::{Unit, FromUnit, SchemaMapEntry, SchemaInt, SchemaStr, SchemaUnit, Schema, SchemaMapRequire, SchemaPair, SchemaOr, SchemaSeq, Or, SchemaMapSeq};

use crate::vnix::utils;

use super::{TermAct, Term};


trait ParseBytes: Sized {
    fn parse_bytes<'a, I>(it: I) -> Option<(Self, I)> where I: Iterator<Item = &'a u8>;
}

#[derive(Debug, Clone)]
pub enum Pixels {
    Rgb(Vec<u8>),
    Rgba(Vec<u32>),
    RgbRle(Vec<(usize, (u8, u8, u8))>),
    RgbaRle(Vec<(usize, u32)>)
}

#[derive(Debug, Clone)]
pub struct Img {
    pub size: (usize, usize),
    pub img: Pixels,
    pub _cache: Option<Vec<u32>>
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
    Vid(VideoIterLoop)
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
    img: Img,

    idx: usize
}

#[derive(Debug, Clone)]
pub struct VideoIterLoop(VideoIter);

impl Img {
    pub fn draw(&mut self, pos: (i32, i32), src: u32, kern: &mut Kern) -> Result<(), KernErr> {
        let size = self.size.clone();
        let pixels = self.get_pixels().ok_or(KernErr::DecompressionFault)?;

        kern.disp.blk(pos, size, src, &pixels).map_err(|e| KernErr::DispErr(e))
    }

    pub fn get_pixels(&mut self) -> Option<&mut Vec<u32>> {
        if self._cache.is_none() {
            self._cache = match &self.img {
                Pixels::Rgb(img) => {
                    if img.len() / 3 != self.size.0 * self.size.1 {
                        return None;
                    }
                    Some(img.array_chunks::<3>().map(|ch| u32::from_le_bytes([ch[0], ch[1], ch[2], 0])).collect())
                },
                Pixels::Rgba(img) => {
                    if img.len() != self.size.0 * self.size.1 {
                        return None;
                    }
                    Some(img.clone())
                },
                Pixels::RgbRle(rle) => {
                    let tmp = rle.iter().map(|(cnt, px)| (0..*cnt).map(|_| u32::from_le_bytes([px.0, px.1, px.2, 0]))).flatten().collect::<Vec<_>>();
    
                    if tmp.len() != self.size.0 * self.size.1 {
                        return None;
                    }
    
                    Some(tmp)
                },
                Pixels::RgbaRle(rle) => {
                    let tmp = rle.iter().map(|(cnt, px)| (0..*cnt).map(|_| *px)).flatten().collect::<Vec<_>>();
    
                    if tmp.len() != self.size.0 * self.size.1 {
                        return None;
                    }
    
                    Some(tmp)
                }
            }
        }

        self._cache.as_mut()
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
    type Item = Img;

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

        for ((block_x, block_y), diff) in &diff.diff {
            let mut it = self.vid.blocks.get(*diff as usize)?.into_iter(&self.vid.pallete);
            let size = self.img.size.clone();
            let img = self.img.get_pixels()?;

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

impl Iterator for VideoIterLoop {
    type Item = Img;

    fn next(&mut self) -> Option<Self::Item> {
        match self.0.next() {
            Some(img) => Some(img),
            None => {
                self.0.idx = 0;
                self.0.img = self.0.vid.img.clone();
                self.0.next()
            }
        }
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

        schm.find(glob, u).and_then(|(size, (fmt, or))|{
            let img = match or {
                Or::First(s) => {
                    let img0 = utils::decompress_bytes(s.as_str()).ok()?;

                    match fmt.as_str() {
                        "rgb" => Pixels::Rgb(img0),
                        "rgba" => Pixels::Rgba(img0.into_iter().array_chunks::<4>().map(|ch| u32::from_le_bytes(ch)).collect()),
                        "rgb.rle" => Pixels::RgbRle(img0.into_iter().array_chunks::<6>().map(|ch| {
                            let cnt = u32::from_le_bytes([ch[0], ch[1], ch[2], 0]);
                            (cnt as usize, (ch[3], ch[4], ch[5]))
                        }).collect()),
                        "rgba.rle" => Pixels::RgbaRle(img0.into_iter().array_chunks::<7>().map(|ch| {
                            let cnt = u32::from_le_bytes([ch[0], ch[1], ch[2], 0]);
                            let px = u32::from_le_bytes([ch[3], ch[4], ch[5], ch[6]]);
                            (cnt as usize, px)
                        }).collect()),
                        _ => return None
                    }
                },
                Or::Second(or) =>
                    match or {
                        Or::First(rle) => {
                            match fmt.as_str() {
                                "rgb.rle" =>
                                    Pixels::RgbRle(rle.into_iter().map(|(cnt, px)| {
                                        let b = (px as u32).to_le_bytes();
                                        (cnt as usize, (b[0], b[1], b[2]))
                                    }).collect()),
                                "rgba.rle" =>
                                Pixels::RgbaRle(rle.into_iter().map(|(cnt, px)|(cnt as usize, px as u32)).collect()),
                                _ => return None
                            }
                        },
                        Or::Second(seq) => {
                            match fmt.as_str() {
                                "rgb" => Pixels::Rgb(seq.into_iter().map(|px| {
                                    let b = (px as u32).to_le_bytes();
                                    vec![b[0], b[1], b[2]]
                                }).flatten().collect()),
                                "rgba" => Pixels::Rgba(seq.into_iter().map(|px| px as u32).collect()),
                                _ => return None
                            }
                        }
                    }
            };

            Some(Img {
                size: (size.0 as usize, size.1 as usize),
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
                        return Some(Tex::Vid(VideoIterLoop(vid.into_iter())));
                    }

                    None
                }
            }
        })
    }
}

impl TermAct for Img {
    fn act(mut self, _term: &mut Term, _orig: &Msg, msg: Unit, kern: &mut Kern) -> Result<Unit, KernErr> {
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
    fn act(mut self, _term: &mut Term, _orig: &Msg, msg: Unit, kern: &mut Kern) -> Result<Unit, KernErr> {
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

        // render frames
        let mut it = self.into_iter();

        while let Some(mut img) = it.next() {
            img.draw(pos, 0x00ff00, kern)?;
            kern.disp.flush_blk(pos, img.size).map_err(|e| KernErr::DispErr(e))?;

            kern.time.wait(15000).map_err(|e| KernErr::TimeErr(e))?;
        }

        Ok(msg)
    }
}
