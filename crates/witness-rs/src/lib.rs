//! Witness — operation witnessing for spectral.
//!
//! A witness observes a spectral operation (diff, commit, merge) and produces
//! a signed attestation. The attestation is content-addressed: same operation
//! = same witness Oid.

#[cfg(test)]
mod tests {
    #[test]
    fn attestation_is_content_addressed() {
        // RED: Attestation type doesn't exist yet
        todo!("Attestation type not implemented");
    }
}
