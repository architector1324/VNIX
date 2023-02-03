use alloc::vec::Vec;
use alloc::string::String;

use compression::prelude::{GZipEncoder, GZipDecoder, Action, EncodeExt, DecodeExt};
use base64ct::{Base64, Encoding};

use super::core::{kern::KernErr, unit::Unit};


pub fn compress(s: &str) -> Result<String, KernErr> {
    let mut enc = GZipEncoder::new();
    let compressed = s.as_bytes().into_iter().cloned().encode(&mut enc, Action::Finish).collect::<Result<Vec<_>, _>>().map_err(|_| KernErr::CompressionFault)?;

    Ok(Base64::encode_string(&compressed))
}

pub fn decompress(s: &str) -> Result<String, KernErr> {
    let mut dec = GZipDecoder::new();

    let v = Base64::decode_vec(s).map_err(|_| KernErr::DecodeFault)?;
    let decompressed = v.iter().cloned().decode(&mut dec).collect::<Result<Vec<_>, _>>().map_err(|_| KernErr::DecompressionFault)?;

    String::from_utf8(decompressed).map_err(|_| KernErr::DecodeFault)
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
            data: Unit::Map(Vec::new())
        }
    }
}

impl RamStore {
    pub fn load(&self, key: Unit) -> Option<Unit> {
        if let Unit::Ref(path) = key {
            return Unit::find_ref(path.into_iter(), &self.data);
        }
        None
    }

    pub fn save(&mut self, key: Unit, val: Unit) {
        if let Unit::Ref(path) = key {
            if let Some(data) = Unit::merge_ref(path.into_iter(), val, self.data.clone()) {
                self.data = data;
            }
        }
    }
}
