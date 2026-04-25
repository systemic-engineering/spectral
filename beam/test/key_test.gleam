import conversation/key
import conversation/oid
import conversation/ref
import gleeunit/should

pub fn generate_produces_keypair_test() {
  // Generate should succeed and produce a keypair
  let kp = key.generate()
  let pub_key = key.public_key(kp)
  // Public key should be constructable
  case pub_key {
    key.Ed25519(_) -> should.be_true(True)
  }
}

pub fn sign_verify_roundtrip_test() {
  let kp = key.generate()
  let pub_key = key.public_key(kp)
  let message = <<"hello world":utf8>>
  let signature = key.sign(kp, message)
  key.verify(pub_key, message, signature) |> should.be_true()
}

pub fn verify_wrong_message_fails_test() {
  let kp = key.generate()
  let pub_key = key.public_key(kp)
  let signature = key.sign(kp, <<"correct":utf8>>)
  key.verify(pub_key, <<"wrong":utf8>>, signature) |> should.be_false()
}

pub fn verify_wrong_key_fails_test() {
  let kp1 = key.generate()
  let kp2 = key.generate()
  let pub_key2 = key.public_key(kp2)
  let signature = key.sign(kp1, <<"message":utf8>>)
  key.verify(pub_key2, <<"message":utf8>>, signature) |> should.be_false()
}

pub fn key_oid_deterministic_test() {
  let kp = key.generate()
  let pub_key = key.public_key(kp)
  let oid1 = key.oid(pub_key)
  let oid2 = key.oid(pub_key)
  // Same key always produces same oid
  oid.equals(ref.oid(oid1), ref.oid(oid2)) |> should.be_true()
}

pub fn different_keys_different_oids_test() {
  let kp1 = key.generate()
  let kp2 = key.generate()
  let oid1 = key.oid(key.public_key(kp1))
  let oid2 = key.oid(key.public_key(kp2))
  oid.equals(ref.oid(oid1), ref.oid(oid2)) |> should.be_false()
}

pub fn hierarchical_derivation_deterministic_test() {
  let root = key.generate()
  let root_pub = key.public_key(root)
  let kp1 = key.derive_child(root_pub, "compiler")
  let kp2 = key.derive_child(root_pub, "compiler")
  // Same root + same name = same derived key
  let oid1 = key.oid(key.public_key(kp1))
  let oid2 = key.oid(key.public_key(kp2))
  oid.equals(ref.oid(oid1), ref.oid(oid2)) |> should.be_true()
}

pub fn hierarchical_differs_from_flat_test() {
  let root = key.generate()
  let root_pub = key.public_key(root)
  let derived = key.derive_child(root_pub, "compiler")
  let flat = key.from_seed(<<0:256>>)
  // Hierarchical derivation should produce a different key than flat
  let oid_derived = key.oid(key.public_key(derived))
  let oid_flat = key.oid(key.public_key(flat))
  oid.equals(ref.oid(oid_derived), ref.oid(oid_flat)) |> should.be_false()
}

pub fn different_names_different_derived_keys_test() {
  let root = key.generate()
  let root_pub = key.public_key(root)
  let compiler_kp = key.derive_child(root_pub, "compiler")
  let garden_kp = key.derive_child(root_pub, "garden")
  // Different names under same root = different keys
  let oid1 = key.oid(key.public_key(compiler_kp))
  let oid2 = key.oid(key.public_key(garden_kp))
  oid.equals(ref.oid(oid1), ref.oid(oid2)) |> should.be_false()
}

pub fn derive_sign_verify_roundtrip_test() {
  let root = key.generate()
  let root_pub = key.public_key(root)
  let derived = key.derive_child(root_pub, "actor")
  let pub_key = key.public_key(derived)
  let message = <<"signed by derived key":utf8>>
  let signature = key.sign(derived, message)
  key.verify(pub_key, message, signature) |> should.be_true()
}
