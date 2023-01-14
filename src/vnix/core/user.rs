use alloc::format;
use alloc::string::{String, ToString};
use core::fmt::{Display, Formatter};

use p256::ecdsa::{SigningKey, VerifyingKey};
use p256::ecdsa::signature::{Signature, Signer, Verifier};

use base64ct::{Base64, Encoding};

use super::kern::{KernErr, Kern};
use super::unit::Unit;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Usr {
    pub name: String,
    priv_key: Option<String>,
    pub_key: String // sec1: elliptic curve
}

impl Display for Usr {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        if self.name.contains(" ") {
            write!(f, "{{ath:`{}` pub:{} priv:{}}}", self.name, self.pub_key, if self.priv_key.is_some() {".."} else {"-"})
        } else {
            write!(f, "{{ath:{} pub:{} priv:{}}}", self.name, self.pub_key, if self.priv_key.is_some() {".."} else {"-"})
        }
    }
}

impl Usr {
    pub fn new(name: &str, kern: &mut Kern) -> Result<Self, KernErr> {
        // gen private key
        let mut priv_key_b: [u8; 32] = [0; 32];
        kern.rnd.get_bytes(&mut priv_key_b).map_err(|e| KernErr::RndErr(e))?;

        let p = SigningKey::from_bytes(&priv_key_b).map_err(|_| KernErr::CreatePrivKeyFault)?;

        // gen public key
        let v = VerifyingKey::from(&p);
        let pub_key_b: [u8; 33] = v.to_encoded_point(true).as_bytes().try_into().map_err(|_| KernErr::CreatePubKeyFault)?;

        // encode base64
        let mut buf = [0; 256];

        let priv_key = Base64::encode(&priv_key_b, &mut buf).map_err(|_| KernErr::EncodeFault)?.to_string(); 
        let pub_key = Base64::encode(&pub_key_b, &mut buf).map_err(|_| KernErr::EncodeFault)?.to_string();

        Ok(Usr {
            name: name.into(),
            priv_key: Some(priv_key),
            pub_key
        })
    }

    pub fn sign(&self, u: &Unit) -> Result<String, KernErr> {
        if let Some(priv_key_s) = &self.priv_key {
            let mut buf = [0; 256];

            let priv_key_b = Base64::decode(priv_key_s.as_bytes(), &mut buf).map_err(|_| KernErr::DecodeFault)?;
            let priv_key = SigningKey::from_bytes(priv_key_b).map_err(|_| KernErr::CreatePrivKeyFault)?;

            let msg = format!("{}", u);

            let sign_b = priv_key.sign(msg.as_bytes());
            let sign = Base64::encode(&sign_b.as_bytes(), &mut buf).map_err(|_| KernErr::EncodeFault)?.to_string(); 

            return Ok(sign)
        }
        Err(KernErr::SignFault)
    }

    pub fn verify(&self, u: &Unit, sign: &String) -> Result<(), KernErr> {
        let mut buf = [0; 256];

        let sign_b = Base64::decode(&sign.as_bytes(), &mut buf).map_err(|_| KernErr::DecodeFault)?;
        let sign = Signature::from_bytes(&sign_b).map_err(|_| KernErr::SignVerifyFault)?;

        let pub_key_b = Base64::decode(&self.pub_key.as_bytes(), &mut buf).map_err(|_| KernErr::DecodeFault)?;
        let pub_key = VerifyingKey::from_sec1_bytes(&pub_key_b).map_err(|_| KernErr::CreatePubKeyFault)?;

        let msg = format!("{}", u);

        pub_key.verify(msg.as_bytes(), &sign).map_err(|_| KernErr::SignVerifyFault)
    }
}
