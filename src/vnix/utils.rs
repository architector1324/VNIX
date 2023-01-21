use alloc::string::String;
use alloc::vec::Vec;

use compression::prelude::{GZipEncoder, GZipDecoder, Action, EncodeExt, DecodeExt};
use base64ct::{Base64, Encoding};

use super::core::kern::KernErr;


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
