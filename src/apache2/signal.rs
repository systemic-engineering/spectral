//! Signal and GraphMutation types.

/// The kind of signal flowing through a spectral runtime.
#[derive(Debug, Clone, PartialEq)]
pub enum SignalKind {
    Init,
    Tick,
    Tock,
    Crystal,
    Observe,
}

/// A signal carrying a kind and a payload.
#[derive(Debug, Clone)]
pub struct Signal {
    kind: SignalKind,
    payload: String,
}

impl Signal {
    pub fn new(kind: SignalKind, payload: String) -> Self {
        Signal { kind, payload }
    }

    pub fn kind(&self) -> &SignalKind {
        &self.kind
    }

    pub fn payload(&self) -> &str {
        &self.payload
    }
}
