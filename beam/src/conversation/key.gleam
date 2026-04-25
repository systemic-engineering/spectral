//// Key — actor cryptographic identity.
////
//// Ed25519 keypairs for signing and verifying actor messages.

import conversation/oid
import conversation/ref.{type ScopedOid}

/// An Ed25519 keypair. Private key material — never expose.
pub opaque type KeyPair {
  KeyPair(public: BitArray, private: BitArray)
}

/// A public key. Can be freely shared.
pub type Key {
  Ed25519(public: BitArray)
}

/// Generate a new Ed25519 keypair.
pub fn generate() -> KeyPair {
  let #(public, private) = do_generate()
  KeyPair(public: public, private: private)
}

/// Generate a deterministic Ed25519 keypair from a 32-byte seed.
/// Same seed = same keypair = same identity. This is the cairn pattern.
pub fn from_seed(seed: BitArray) -> KeyPair {
  let #(public, private) = do_generate_from_seed(seed)
  KeyPair(public: public, private: private)
}

/// Extract the public key from a keypair.
pub fn public_key(kp: KeyPair) -> Key {
  Ed25519(public: kp.public)
}

/// Sign data with a keypair.
pub fn sign(kp: KeyPair, data: BitArray) -> BitArray {
  do_sign(kp.private, data)
}

/// Verify a signature against a public key and data.
pub fn verify(key: Key, data: BitArray, signature: BitArray) -> Bool {
  let Ed25519(public) = key
  do_verify(public, data, signature)
}

/// Derive a child keypair from a root public key and domain name.
/// sha512(root_pub || name) → first 32 bytes → Ed25519 seed.
/// Anyone with the root public key can derive any actor's public key.
pub fn derive_child(root: Key, name: String) -> KeyPair {
  let Ed25519(public) = root
  let seed_input = <<public:bits, name:utf8>>
  let assert <<seed:bytes-size(32), _rest:bytes>> = do_sha512(seed_input)
  from_seed(seed)
}

/// Content address of a public key.
pub fn oid(key: Key) -> ScopedOid(Key) {
  let Ed25519(public) = key
  ref.scope(oid.from_bytes(public))
}

@external(erlang, "crypto_ffi", "generate_ed25519")
fn do_generate() -> #(BitArray, BitArray)

@external(erlang, "crypto_ffi", "generate_ed25519_from_seed")
fn do_generate_from_seed(seed: BitArray) -> #(BitArray, BitArray)

@external(erlang, "crypto_ffi", "sign_ed25519")
fn do_sign(private: BitArray, message: BitArray) -> BitArray

@external(erlang, "crypto_ffi", "verify_ed25519")
fn do_verify(public: BitArray, message: BitArray, signature: BitArray) -> Bool

@external(erlang, "crypto_ffi", "sha512")
fn do_sha512(data: BitArray) -> BitArray
