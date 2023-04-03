use alloc::vec::Vec;
use alloc::string::String;

use compression::prelude::{GZipEncoder, GZipDecoder, Action, EncodeExt, DecodeExt};
use base64ct::{Base64, Encoding};

use super::core::kern::KernErr;
use super::core::unit::{Unit, UnitAs, UnitModify, UnitNew, UnitParse, UnitParseBytesIter};

pub type Maybe<T, E> = Result<Option<T>, E>;

#[macro_export]
macro_rules! maybe_ok {
    ($e:expr) => {
        {
            if let Some(res) = $e {
                res
            } else {
                return Ok(None)
            }
        }
    };
}

#[macro_export]
macro_rules! maybe {
    ($e:expr) => {
        crate::maybe_ok!($e?)
    };
}

pub fn compress(s: &str) -> Result<String, KernErr> {
    let mut enc = GZipEncoder::new();
    let compressed = s.as_bytes().into_iter().cloned().encode(&mut enc, Action::Finish).collect::<Result<Vec<_>, _>>().map_err(|_| KernErr::CompressionFault)?;

    Ok(Base64::encode_string(&compressed))
}

pub fn compress_bytes(b: &[u8]) -> Result<String, KernErr> {
    let mut enc = GZipEncoder::new();
    let compressed = b.into_iter().cloned().encode(&mut enc, Action::Finish).collect::<Result<Vec<_>, _>>().map_err(|_| KernErr::CompressionFault)?;

    Ok(Base64::encode_string(&compressed))
}

pub fn decompress(s: &str) -> Result<String, KernErr> {
    let mut dec = GZipDecoder::new();

    let v = Base64::decode_vec(s).map_err(|_| KernErr::DecodeFault)?;
    let decompressed = v.iter().cloned().decode(&mut dec).collect::<Result<Vec<_>, _>>().map_err(|_| KernErr::DecompressionFault)?;

    String::from_utf8(decompressed).map_err(|_| KernErr::DecodeFault)
}

pub fn decompress_bytes(s: &str) -> Result<Vec<u8>, KernErr> {
    let mut dec = GZipDecoder::new();

    let v = Base64::decode_vec(s).map_err(|_| KernErr::DecodeFault)?;
    let decompressed = v.iter().cloned().decode(&mut dec).collect::<Result<Vec<_>, _>>().map_err(|_| KernErr::DecompressionFault)?;

    Ok(decompressed)
}

// optimized units iterator from bytes
pub fn unit_compressed_iterator(s: &str) -> Maybe<UnitParseBytesIter, KernErr> {
    let dat = decompress_bytes(&s)?;
    let (_, it) = maybe_ok!(Unit::parse_list_partial(dat.iter()).ok());
    let dat = it.cloned().collect::<Vec<u8>>();

    Ok(Some(UnitParseBytesIter::new(dat)))
}

pub fn hex_to_u32(s: &str) -> Option<u32> {
    if s.starts_with("#") {
        return Some(<u32>::from_str_radix(&s.get(1..7)?, 16)
        .ok()?
        .to_le())
    }
    None
}

#[derive(Debug)]
pub struct RamStore {
    pub data: Unit
}

impl Default for RamStore {
    fn default() -> Self {
        RamStore {
            data: Unit::map(&[])
        }
    }
}

impl RamStore {
    pub fn load(&self, key: Unit) -> Option<Unit> {
        if let Some(path) = key.as_path() {
            return self.data.find(path.iter().map(|s| s.as_str()));
        }
        None
    }

    pub fn save(&mut self, key: Unit, val: Unit) {
        if let Some(path) = key.as_path() {
            if let Some(data) = self.data.clone().merge(path.iter().map(|s| s.as_str()), val) {
                self.data = data;
            }
        }
    }
}
