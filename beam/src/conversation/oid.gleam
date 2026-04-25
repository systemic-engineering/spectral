//// Oid — content-addressed identity.
////
//// Same content = same Oid. Value type wrapping a SHA-512 hex string.

import gleam/string

/// A content address. Wraps a SHA-512 hex digest.
pub opaque type Oid {
  Oid(hash: String)
}

/// Compute an Oid from raw bytes (SHA-512, hex-encoded).
pub fn from_bytes(data: BitArray) -> Oid {
  Oid(hash: hex_encode(do_sha512(data)))
}

/// Construct an Oid from its hex string representation.
pub fn from_string(hex: String) -> Oid {
  Oid(hash: hex)
}

/// The hex string representation of the Oid.
pub fn to_string(oid: Oid) -> String {
  oid.hash
}

/// Are two Oids equal?
pub fn equals(a: Oid, b: Oid) -> Bool {
  a.hash == b.hash
}

@external(erlang, "crypto_ffi", "sha512")
fn do_sha512(data: BitArray) -> BitArray

fn hex_encode(data: BitArray) -> String {
  do_hex_encode(data, "")
}

fn do_hex_encode(data: BitArray, acc: String) -> String {
  case data {
    <<byte:int, rest:bits>> -> {
      let hi = hex_char(byte / 16)
      let lo = hex_char(byte % 16)
      do_hex_encode(rest, string.append(acc, string.append(hi, lo)))
    }
    _ -> acc
  }
}

fn hex_char(nibble: Int) -> String {
  case nibble {
    0 -> "0"
    1 -> "1"
    2 -> "2"
    3 -> "3"
    4 -> "4"
    5 -> "5"
    6 -> "6"
    7 -> "7"
    8 -> "8"
    9 -> "9"
    10 -> "a"
    11 -> "b"
    12 -> "c"
    13 -> "d"
    14 -> "e"
    _ -> "f"
  }
}
