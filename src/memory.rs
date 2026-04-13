//! spectral memory — lens CLI for agent memory.
//!
//! Temporarily stubbed — lens and spectral-db need fate → prism-core migration.
//! Memory commands print a message and exit. The interface is preserved.

use std::process;

pub fn store(_args: &[String]) {
    eprintln!("spectral memory store: not yet wired (lens needs fate migration)");
    process::exit(1);
}

pub fn recall(_args: &[String]) {
    eprintln!("spectral memory recall: not yet wired (lens needs fate migration)");
    process::exit(1);
}

pub fn crystallize(_args: &[String]) {
    eprintln!("spectral memory crystallize: not yet wired (lens needs fate migration)");
    process::exit(1);
}

pub fn status() {
    eprintln!("spectral memory status: not yet wired (lens needs fate migration)");
}

pub fn export(_args: &[String]) {
    eprintln!("spectral memory export: not yet wired (lens needs fate migration)");
    process::exit(1);
}

pub fn ingest(_args: &[String]) {
    eprintln!("spectral memory ingest: not yet wired (lens needs fate migration)");
    process::exit(1);
}
