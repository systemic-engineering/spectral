# spectral sign / spectral verify — Dual Signature

> Traditional signature: who signed it.
> Spectral signature: what the world looked like when they signed it.

## Why Dual

The traditional signature bridges to the existing trust ecosystem.
Ed25519. SSH keys. GPG. Sigstore. The world already knows how to
verify these. Ship without them and nobody trusts the binary.

The spectral signature bridges to the new trust model.
SpectralOid. Eigenvalue hash. Content-addressed graph state.
The signature carries the context of the signing: how many nodes,
what settlement, what Shannon loss. The signature ages. The drift
IS information.

Both signatures on every binary. Both verifiable. Different
questions answered.

```
Traditional:  "Did Reed sign this?"           (identity)
Spectral:     "What was the garden's state    (context)
               when Reed signed this?"
```

## Commands

### spectral sign

```
$ spectral sign <binary>

Options:
  --key <path>          Ed25519 private key (default: ~/.ssh/id_ed25519)
  --garden <path>       Garden to snapshot for spectral signature (default: .)
  --release <tag>       Sign all platform binaries for a release
  --output <path>       Write signature file (default: <binary>.sig)
```

### spectral verify

```
$ spectral verify <binary>

Options:
  --key <path>          Ed25519 public key (default: from signature)
  --garden <path>       Current garden to compute drift (default: .)
  --strict              Fail if drift exceeds threshold
  --threshold <float>   Maximum acceptable drift (default: 0.10 = 10%)
```

## Signature File Format

```
spectral-sig v1
binary: spectral-v0.1.0-darwin-arm64
sha256: a7f3b2e1c4d8...                    (binary hash)
timestamp: 2026-04-06T18:42:00Z

traditional:
  algorithm: ed25519
  signer: reed@systemic.engineer
  public_key: ssh-ed25519 AAAA...
  signature: base64...

spectral:
  spectral_oid: SpectralOid("a7f3b2e1...")
  precision: 0.97
  garden_state:
    nodes: 847
    edges: 2341
    grammars: 12
    settlement: 0.673
    loss: 4.21
    eigenvalues: [0.00, 1.23, 2.47, ...]   (top-k eigenvalues of garden graph)
  graph_hash: sha256 of garden's Laplacian eigenvalue vector
  signature: base64...                       (Ed25519 signature over spectral block)
```

The spectral block is ALSO signed by Ed25519. The traditional
signature covers the binary hash. The spectral signature covers
the garden state at signing time. Both signed by the same key.

## Verification Output

### Happy path

```
$ spectral verify spectral-db

Traditional:  ✓ Ed25519 valid (reed@systemic.engineer)
Spectral:     ✓ SpectralOid valid
              signed at:    2026-04-06T18:42:00Z
              graph state:  847 nodes, 12 grammars
              settlement:   67.3%
              loss:         4.21 bits
              drift:        0.0% (just signed)
```

### After time passes

```
$ spectral verify spectral-db

Traditional:  ✓ Ed25519 valid (reed@systemic.engineer)
Spectral:     ✓ SpectralOid valid
              signed at:    2026-04-06T18:42:00Z
              graph at sign: 847 nodes, 67.3% settlement
              graph now:     1,203 nodes, 71.8% settlement
              drift:         4.5% (within threshold)

              The garden grew since signing.
              Settlement improved. The binary is still valid.
              The drift tells you: the world moved forward.
```

### Drift exceeds threshold

```
$ spectral verify --strict --threshold 0.05 spectral-db

Traditional:  ✓ Ed25519 valid
Spectral:     ⚠ drift 4.5% (exceeds threshold 5.0%)

              The binary was signed when the garden had 847 nodes.
              The garden now has 1,203 nodes. Consider re-signing.

              This is NOT a security failure.
              The binary is authentic (traditional sig valid).
              The garden evolved past the signed state.
              A new signature would capture the current state.
```

### Tampered binary

```
$ spectral verify spectral-db

Traditional:  ✗ Ed25519 INVALID
              Binary hash does not match signature.

              DO NOT RUN THIS BINARY.
```

Traditional catches tampering. Spectral catches drift.
Different threats. Both covered.

## Release Signing

```
$ spectral sign --release v0.1.0

Signing release v0.1.0...

  spectral-v0.1.0-darwin-arm64
    sha256: a7f3...
    Traditional: Ed25519 ✓
    Spectral: SpectralOid ✓

  spectral-v0.1.0-darwin-x86_64
    sha256: b2c4...
    Traditional: Ed25519 ✓
    Spectral: SpectralOid ✓

  spectral-v0.1.0-linux-x86_64
    sha256: c3d5...
    Traditional: Ed25519 ✓
    Spectral: SpectralOid ✓

  spectral-v0.1.0-linux-aarch64
    sha256: d4e6...
    Traditional: Ed25519 ✓
    Spectral: SpectralOid ✓

  spectral-v0.1.0-wasm32
    sha256: e5f7...
    Traditional: Ed25519 ✓
    Spectral: SpectralOid ✓

Release manifest: spectral-v0.1.0.manifest
  5 binaries, dual-signed
  Garden state at release: 847 nodes, 12 grammars, 67.3% settlement
```

The release manifest includes the garden state. Anyone verifying
any binary from this release can see the garden's state at release
time. The manifest IS a snapshot of the garden's eigenvalues.

## Embedded spectral-db

The spectral binary embeds spectral-db as a signed dependency.
The spectral-db binary has its own dual signature. At build time:

```
1. spectral-db is built and dual-signed
2. spectral embeds the signed spectral-db
3. spectral is dual-signed (covering the embedded spectral-db)
4. At runtime: spectral verifies spectral-db's embedded signature
   before initializing the database
```

Two layers of signature. The outer (spectral) and the inner
(spectral-db). Both dual-signed. Both verifiable.

```
spectral binary:
  ├── traditional sig (Ed25519, covers whole binary)
  ├── spectral sig (SpectralOid, covers garden state)
  └── embedded:
      └── spectral-db:
          ├── traditional sig (Ed25519)
          └── spectral sig (SpectralOid)
```

## Implementation

### Signing (Rust)

```rust
pub struct DualSignature {
    pub traditional: TraditionalSig,
    pub spectral: SpectralSig,
}

pub struct TraditionalSig {
    pub algorithm: String,        // "ed25519"
    pub signer: String,           // "reed@systemic.engineer"
    pub public_key: Vec<u8>,
    pub signature: Vec<u8>,
}

pub struct SpectralSig {
    pub spectral_oid: SpectralOid,
    pub garden_state: GardenSnapshot,
    pub graph_hash: [u8; 32],
    pub signature: Vec<u8>,       // Ed25519 over the spectral block
}

pub struct GardenSnapshot {
    pub nodes: u64,
    pub edges: u64,
    pub grammars: u64,
    pub settlement: f64,
    pub loss: f64,
    pub eigenvalues: Vec<f64>,    // top-k eigenvalues
}
```

### Drift Computation

```rust
pub fn drift(signed: &GardenSnapshot, current: &GardenSnapshot) -> f64 {
    // Cosine distance between eigenvalue vectors
    let dot: f64 = signed.eigenvalues.iter()
        .zip(&current.eigenvalues)
        .map(|(a, b)| a * b)
        .sum();
    let mag_a: f64 = signed.eigenvalues.iter().map(|x| x * x).sum::<f64>().sqrt();
    let mag_b: f64 = current.eigenvalues.iter().map(|x| x * x).sum::<f64>().sqrt();

    if mag_a == 0.0 || mag_b == 0.0 {
        return 1.0; // maximum drift
    }

    1.0 - (dot / (mag_a * mag_b)) // 0.0 = identical, 1.0 = orthogonal
}
```

Drift is cosine distance between eigenvalue vectors. The
eigenvalues capture the graph's topology. Cosine distance
captures how much the topology changed. Clean. Measurable.
The same spectral math used everywhere else.

## Connection to the Architecture

The spectral signature IS a Beam:

```
Beam {
    result: binary_hash,
    path: [signer_oid, garden_oid],
    loss: settlement_gap (1.0 - settlement),
    precision: eigenvalue_precision,
    recovered: None,
}
```

Signing IS a refract operation. The signer observes the garden
(focus), decides it's ready to sign (project), the binary exists
for multiple platforms (split), each platform is signed (zoom),
and the signature crystallizes (refract).

The signature IS a crystal. Content-addressed. Immutable.
The drift measures how far the garden moved from that crystal.

## Build Path

1. `DualSignature` struct + `GardenSnapshot` (types only)
2. Traditional signing (Ed25519 via ssh-key crate, already in deps)
3. Spectral signing (eigenvalue snapshot + SpectralOid)
4. `spectral sign` CLI command
5. `spectral verify` CLI command with drift computation
6. Release manifest format
7. Embedded binary verification at startup
8. Integration with CI/CD (sign on release tag)

## The Open Source Release

The first `spectral sign --release v0.1.0` is the moment the
code goes public. Dual-signed. The traditional signature says
Reed released it. The spectral signature says: the garden had
847 nodes and 67.3% settlement when this was released. The
world can verify both. Forever.

The signature IS the birth certificate of the public release.
Content-addressed. Dual-signed. Carrying the garden's eigenvalues
at the moment it became visible to the world.
