use alloc::string::String;
use alloc::vec::Vec;

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


pub struct RamDB {
    pub data: Vec<(Unit, Unit)>
}

impl Default for RamDB {
    fn default() -> Self {
        RamDB {
            data: Vec::new()
        }
    }
}

impl RamDB {
    pub fn load(&self, key: Unit) -> Option<Unit> {
        self.data.iter().find_map(|(k, val)| {
            if k.clone() == key {
                return Some(val.clone())
            }
            None
        })
    }

    pub fn save(&mut self, key: Unit, val: Unit) {
        self.data.retain(|(k, _)| k.clone() != key);
        self.data.push((key, val));
    }
}
