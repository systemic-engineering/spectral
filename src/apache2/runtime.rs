//! The Runtime trait. Mirror compiles. Runtimes execute.

use terni::{Imperfect, Loss};

/// A runtime that processes signals and produces state transitions
/// with measured loss.
pub trait Runtime {
    type State;
    type Signal;
    type Error;
    type L: Loss;

    fn tick(&mut self, signal: Self::Signal) -> Imperfect<Self::State, Self::Error, Self::L>;
}
