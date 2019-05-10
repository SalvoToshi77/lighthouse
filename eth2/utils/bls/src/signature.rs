use super::{PublicKey, SecretKey, BLS_SIG_BYTE_SIZE};
use bls_aggregates::Signature as RawSignature;
use cached_tree_hash::cached_tree_hash_ssz_encoding_as_vector;
use hex::encode as hex_encode;
use serde::de::{Deserialize, Deserializer};
use serde::ser::{Serialize, Serializer};
use serde_hex::HexVisitor;
use ssz::{decode, ssz_encode, Decodable, DecodeError, Encodable, SszStream};
use tree_hash::tree_hash_ssz_encoding_as_vector;

/// A single BLS signature.
///
/// This struct is a wrapper upon a base type and provides helper functions (e.g., SSZ
/// serialization).
#[derive(Debug, PartialEq, Clone, Eq)]
pub struct Signature {
    signature: RawSignature,
    is_empty: bool,
}

impl Signature {
    /// Instantiate a new Signature from a message and a SecretKey.
    pub fn new(msg: &[u8], domain: u64, sk: &SecretKey) -> Self {
        Signature {
            signature: RawSignature::new(msg, domain, sk.as_raw()),
            is_empty: false,
        }
    }

    /// Instantiate a new Signature from a message and a SecretKey, where the message has already
    /// been hashed.
    pub fn new_hashed(x_real_hashed: &[u8], x_imaginary_hashed: &[u8], sk: &SecretKey) -> Self {
        Signature {
            signature: RawSignature::new_hashed(x_real_hashed, x_imaginary_hashed, sk.as_raw()),
            is_empty: false,
        }
    }

    /// Verify the Signature against a PublicKey.
    pub fn verify(&self, msg: &[u8], domain: u64, pk: &PublicKey) -> bool {
        if self.is_empty {
            return false;
        }
        self.signature.verify(msg, domain, pk.as_raw())
    }

    /// Verify the Signature against a PublicKey, where the message has already been hashed.
    pub fn verify_hashed(
        &self,
        x_real_hashed: &[u8],
        x_imaginary_hashed: &[u8],
        pk: &PublicKey,
    ) -> bool {
        self.signature
            .verify_hashed(x_real_hashed, x_imaginary_hashed, pk.as_raw())
    }

    /// Returns the underlying signature.
    pub fn as_raw(&self) -> &RawSignature {
        &self.signature
    }

    /// Returns a new empty signature.
    pub fn empty_signature() -> Self {
        // Set RawSignature = infinity
        let mut empty: Vec<u8> = vec![0; BLS_SIG_BYTE_SIZE];
        empty[0] += u8::pow(2, 6) + u8::pow(2, 7);
        Signature {
            signature: RawSignature::from_bytes(&empty).unwrap(),
            is_empty: true,
        }
    }

    // Converts a BLS Signature to bytes
    pub fn as_bytes(&self) -> Vec<u8> {
        if self.is_empty {
            return vec![0; 96];
        }
        self.signature.as_bytes()
    }

    // Convert bytes to BLS Signature
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, DecodeError> {
        for byte in bytes {
            if *byte != 0 {
                let raw_signature =
                    RawSignature::from_bytes(&bytes).map_err(|_| DecodeError::Invalid)?;
                return Ok(Signature {
                    signature: raw_signature,
                    is_empty: false,
                });
            }
        }
        Ok(Signature::empty_signature())
    }

    // Check for empty Signature
    pub fn is_empty(&self) -> bool {
        self.is_empty
    }
}

impl Encodable for Signature {
    fn ssz_append(&self, s: &mut SszStream) {
        s.append_encoded_raw(&self.as_bytes());
    }
}

impl Decodable for Signature {
    fn ssz_decode(bytes: &[u8], i: usize) -> Result<(Self, usize), DecodeError> {
        if bytes.len() - i < BLS_SIG_BYTE_SIZE {
            return Err(DecodeError::TooShort);
        }
        let signature = Signature::from_bytes(&bytes[i..(i + BLS_SIG_BYTE_SIZE)])?;
        Ok((signature, i + BLS_SIG_BYTE_SIZE))
    }
}

tree_hash_ssz_encoding_as_vector!(Signature);
cached_tree_hash_ssz_encoding_as_vector!(Signature, 96);

impl Serialize for Signature {
    /// Serde serialization is compliant the Ethereum YAML test format.
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&hex_encode(ssz_encode(self)))
    }
}

impl<'de> Deserialize<'de> for Signature {
    /// Serde serialization is compliant the Ethereum YAML test format.
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let bytes = deserializer.deserialize_str(HexVisitor)?;
        let signature = decode(&bytes[..])
            .map_err(|e| serde::de::Error::custom(format!("invalid ssz ({:?})", e)))?;
        Ok(signature)
    }
}

#[cfg(test)]
mod tests {
    use super::super::Keypair;
    use super::*;
    use ssz::ssz_encode;
    use tree_hash::TreeHash;

    #[test]
    pub fn test_ssz_round_trip() {
        let keypair = Keypair::random();

        let original = Signature::new(&[42, 42], 0, &keypair.sk);

        let bytes = ssz_encode(&original);
        let decoded = decode::<Signature>(&bytes).unwrap();

        assert_eq!(original, decoded);
    }

    #[test]
    pub fn test_cached_tree_hash() {
        let keypair = Keypair::random();
        let original = Signature::new(&[42, 42], 0, &keypair.sk);

        let mut cache = cached_tree_hash::TreeHashCache::new(&original).unwrap();

        assert_eq!(
            cache.tree_hash_root().unwrap().to_vec(),
            original.tree_hash_root()
        );

        let modified = Signature::new(&[99, 99], 0, &keypair.sk);

        cache.update(&modified).unwrap();

        assert_eq!(
            cache.tree_hash_root().unwrap().to_vec(),
            modified.tree_hash_root()
        );
    }

    #[test]
    pub fn test_empty_signature() {
        let sig = Signature::empty_signature();

        let sig_as_bytes: Vec<u8> = sig.as_raw().as_bytes();

        assert_eq!(sig_as_bytes.len(), BLS_SIG_BYTE_SIZE);
        for (i, one_byte) in sig_as_bytes.iter().enumerate() {
            if i == 0 {
                assert_eq!(*one_byte, u8::pow(2, 6) + u8::pow(2, 7));
            } else {
                assert_eq!(*one_byte, 0);
            }
        }
    }
}