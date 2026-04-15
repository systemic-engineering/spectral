//! FateActor — a Ractor actor wrapping Fate::tick().
//!
//! Single actor proof: spawn, send features, receive a decision.
//! The actor holds a Fate instance as state and processes tick messages.

use fate::{Fate, FateOutput, Features};
use ractor::{Actor, ActorProcessingErr, ActorRef};

/// Messages the FateActor can receive.
pub enum FateMsg {
    /// Run one tick of Fate with the given features.
    /// Reply channel sends back the FateOutput.
    Tick(Features, ractor::RpcReplyPort<FateOutput>),
}

/// The actor's persistent state: holds the Fate instance.
pub struct FateState {
    pub fate: Fate,
}

/// The FateActor: wraps Fate in a Ractor actor.
pub struct FateActor;

impl FateActor {
    /// Spawn a FateActor with untrained weights.
    /// Name is optional — use `None` for anonymous actors (e.g., in tests).
    pub async fn spawn_untrained(
        name: Option<String>,
    ) -> Result<ActorRef<FateMsg>, ractor::SpawnErr> {
        let (actor_ref, _handle) =
            Actor::spawn(name, FateActor, ()).await?;
        Ok(actor_ref)
    }
}

#[ractor::async_trait]
impl Actor for FateActor {
    type Msg = FateMsg;
    type State = FateState;
    type Arguments = ();

    async fn pre_start(
        &self,
        _myself: ActorRef<Self::Msg>,
        _args: (),
    ) -> Result<Self::State, ActorProcessingErr> {
        Ok(FateState {
            fate: Fate::untrained(),
        })
    }

    async fn handle(
        &self,
        _myself: ActorRef<Self::Msg>,
        message: Self::Msg,
        state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        match message {
            FateMsg::Tick(_features, _reply) => {
                todo!("implement tick dispatch")
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fate::FEATURE_DIM;

    #[tokio::test]
    async fn fate_actor_spawn_and_tick() {
        let actor_ref = FateActor::spawn_untrained(None).await.expect("spawn failed");

        let features = [1.0f64; FEATURE_DIM];
        let output = ractor::call!(actor_ref, FateMsg::Tick, features)
            .expect("tick call failed");

        // Untrained Fate produces a decision — model should not be Fate
        // (resolve exits on first non-Fate decision)
        assert_ne!(output.model, fate::Model::Fate);
        assert!(output.decision.confidence > 0.0);

        actor_ref.stop(None);
    }

    #[tokio::test]
    async fn fate_actor_multiple_ticks() {
        let actor_ref = FateActor::spawn_untrained(None).await.expect("spawn failed");

        // Send 5 ticks with different features
        for i in 0..5 {
            let mut features = [0.0f64; FEATURE_DIM];
            features[i % FEATURE_DIM] = 10.0;

            let output = ractor::call!(actor_ref, FateMsg::Tick, features)
                .expect("tick call failed");

            assert!(output.decision.confidence > 0.0);
        }

        actor_ref.stop(None);
    }
}
