//! Deterministic simulation tests for actor-based components.
//!
//! This test suite uses madsim to verify the behavior of actor-based components
//! in a controlled, deterministic environment. It tests actor lifecycle,
//! message handling, and supervision.

use std::time::Duration;

use madsim::time::sleep;
use ractor::Actor;
use ractor::ActorRef;
use ractor::RpcReplyPort;
use ractor::SupervisionEvent;
use snafu::prelude::*;
use tracing::info;

/// Error type for simulation tests.
#[derive(Debug, Snafu)]
enum SimulationError {
    #[snafu(display("Actor spawn failed: {source}"))]
    ActorSpawn { source: ractor::SpawnErr },

    #[snafu(display("RPC call failed: {message}"))]
    RpcCall { message: String },

    #[snafu(display("Timeout waiting for condition"))]
    Timeout,
}

type Result<T> = std::result::Result<T, SimulationError>;

/// Simple test actor for demonstrating actor patterns.
struct TestActor;

/// Messages for the test actor.
#[derive(Debug)]
enum TestMessage {
    Ping(RpcReplyPort<String>),
    Increment(u32),
    GetCount(RpcReplyPort<u32>),
    CrashNow,
    Shutdown,
}

impl ractor::Message for TestMessage {}

/// State for the test actor.
struct TestActorState {
    count: u32,
    max_count: u32,
}

/// Arguments for the test actor.
struct TestActorArgs {
    initial_count: u32,
    max_count: u32,
}

#[async_trait::async_trait]
impl Actor for TestActor {
    type Msg = TestMessage;
    type State = TestActorState;
    type Arguments = TestActorArgs;

    async fn pre_start(
        &self,
        _myself: ActorRef<Self::Msg>,
        args: Self::Arguments,
    ) -> std::result::Result<Self::State, ractor::ActorProcessingErr> {
        info!("TestActor starting with initial count: {}", args.initial_count);
        Ok(TestActorState {
            count: args.initial_count,
            max_count: args.max_count,
        })
    }

    async fn post_stop(
        &self,
        _myself: ActorRef<Self::Msg>,
        state: &mut Self::State,
    ) -> std::result::Result<(), ractor::ActorProcessingErr> {
        info!("TestActor stopping with final count: {}", state.count);
        Ok(())
    }

    async fn handle(
        &self,
        myself: ActorRef<Self::Msg>,
        message: Self::Msg,
        state: &mut Self::State,
    ) -> std::result::Result<(), ractor::ActorProcessingErr> {
        match message {
            TestMessage::Ping(reply) => {
                let _ = reply.send(format!("Pong! Count is {}", state.count));
            }
            TestMessage::Increment(amount) => {
                // Tiger Style: Bounded counter
                state.count = state.count.saturating_add(amount).min(state.max_count);
                info!("Count incremented to {}", state.count);
            }
            TestMessage::GetCount(reply) => {
                let _ = reply.send(state.count);
            }
            TestMessage::CrashNow => {
                return Err("Intentional crash for testing".into());
            }
            TestMessage::Shutdown => {
                info!("Shutdown requested");
                myself.stop(None);
            }
        }
        Ok(())
    }
}

/// Test basic actor lifecycle and message handling.
#[madsim::test]
async fn test_actor_lifecycle() -> Result<()> {
    let args = TestActorArgs {
        initial_count: 0,
        max_count: 100,
    };

    let (actor_ref, handle) =
        Actor::spawn(Some("test-actor".to_string()), TestActor, args).await.context(ActorSpawnSnafu)?;

    // Test ping-pong
    let response =
        ractor::call_t!(actor_ref, TestMessage::Ping, Duration::from_secs(1).as_millis() as u64).map_err(|e| {
            SimulationError::RpcCall {
                message: format!("Failed to send ping: {:?}", e),
            }
        })?;

    assert_eq!(response, "Pong! Count is 0");

    // Test increment
    actor_ref.send_message(TestMessage::Increment(5)).map_err(|e| SimulationError::RpcCall {
        message: format!("RPC call failed: {:?}", e),
    })?;

    sleep(Duration::from_millis(100)).await;

    // Get count
    let count =
        ractor::call_t!(actor_ref, TestMessage::GetCount, Duration::from_secs(1).as_millis() as u64).map_err(|e| {
            SimulationError::RpcCall {
                message: format!("RPC call failed: {:?}", e),
            }
        })?;

    assert_eq!(count, 5);

    // Test bounded increment (Tiger Style)
    actor_ref.send_message(TestMessage::Increment(200)).map_err(|e| SimulationError::RpcCall {
        message: format!("RPC call failed: {:?}", e),
    })?;

    sleep(Duration::from_millis(100)).await;

    let count =
        ractor::call_t!(actor_ref, TestMessage::GetCount, Duration::from_secs(1).as_millis() as u64).map_err(|e| {
            SimulationError::RpcCall {
                message: format!("RPC call failed: {:?}", e),
            }
        })?;

    assert_eq!(count, 100, "Count should be capped at max_count");

    // Shutdown
    actor_ref.send_message(TestMessage::Shutdown).map_err(|e| SimulationError::RpcCall {
        message: format!("RPC call failed: {:?}", e),
    })?;

    // Wait for actor to stop
    let _ = handle.await;

    Ok(())
}

/// Test actor supervision and recovery.
#[madsim::test]
async fn test_actor_supervision() -> Result<()> {
    // Create a supervisor actor
    struct SupervisorActor;

    #[derive(Debug)]
    enum SupervisorMessage {
        GetFailureCount(RpcReplyPort<u32>),
    }

    impl ractor::Message for SupervisorMessage {}

    struct SupervisorState {
        failure_count: u32,
    }

    #[async_trait::async_trait]
    impl Actor for SupervisorActor {
        type Msg = SupervisorMessage;
        type State = SupervisorState;
        type Arguments = ();

        async fn pre_start(
            &self,
            _myself: ActorRef<Self::Msg>,
            _args: Self::Arguments,
        ) -> std::result::Result<Self::State, ractor::ActorProcessingErr> {
            Ok(SupervisorState { failure_count: 0 })
        }

        async fn handle(
            &self,
            _myself: ActorRef<Self::Msg>,
            message: Self::Msg,
            state: &mut Self::State,
        ) -> std::result::Result<(), ractor::ActorProcessingErr> {
            match message {
                SupervisorMessage::GetFailureCount(reply) => {
                    let _ = reply.send(state.failure_count);
                    Ok(())
                }
            }
        }

        async fn handle_supervisor_evt(
            &self,
            _myself: ActorRef<Self::Msg>,
            event: SupervisionEvent,
            state: &mut Self::State,
        ) -> std::result::Result<(), ractor::ActorProcessingErr> {
            match event {
                SupervisionEvent::ActorFailed(_, _) => {
                    state.failure_count += 1;
                    info!("Supervised actor failed, count: {}", state.failure_count);
                }
                SupervisionEvent::ActorTerminated(_, _, _) => {
                    info!("Supervised actor terminated");
                }
                _ => {}
            }
            Ok(())
        }
    }

    // Spawn supervisor
    let (supervisor_ref, _) =
        Actor::spawn(Some("supervisor".to_string()), SupervisorActor, ()).await.context(ActorSpawnSnafu)?;

    // Spawn test actor under supervision
    let args = TestActorArgs {
        initial_count: 0,
        max_count: 100,
    };

    let (test_ref, _) =
        Actor::spawn_linked(Some("supervised-actor".to_string()), TestActor, args, supervisor_ref.clone().into())
            .await
            .context(ActorSpawnSnafu)?;

    // Trigger a failure
    let _ = test_ref.send_message(TestMessage::CrashNow);

    // Wait for supervision to handle the failure
    sleep(Duration::from_millis(500)).await;

    // Check failure count
    let failure_count =
        ractor::call_t!(supervisor_ref, SupervisorMessage::GetFailureCount, Duration::from_secs(1).as_millis() as u64)
            .map_err(|e| SimulationError::RpcCall {
                message: format!("RPC call failed: {:?}", e),
            })?;

    assert!(failure_count > 0, "Supervisor should have detected failure");

    Ok(())
}

/// Test concurrent actor interactions.
#[madsim::test]
async fn test_concurrent_actors() -> Result<()> {
    let mut actors = Vec::new();

    // Spawn multiple actors
    for i in 0..5 {
        let args = TestActorArgs {
            initial_count: i * 10,
            max_count: 1000,
        };

        let (actor_ref, _) =
            Actor::spawn(Some(format!("actor-{}", i)), TestActor, args).await.context(ActorSpawnSnafu)?;

        actors.push(actor_ref);
    }

    // Send concurrent messages
    for (i, actor) in actors.iter().enumerate() {
        actor.send_message(TestMessage::Increment(i as u32)).map_err(|e| SimulationError::RpcCall {
            message: format!("RPC call failed: {:?}", e),
        })?;
    }

    // Wait for processing
    sleep(Duration::from_millis(200)).await;

    // Verify all actors processed their messages
    for (i, actor) in actors.iter().enumerate() {
        let count =
            ractor::call_t!(actor, TestMessage::GetCount, Duration::from_secs(1).as_millis() as u64).map_err(|e| {
                SimulationError::RpcCall {
                    message: format!("RPC call failed: {:?}", e),
                }
            })?;

        let expected = (i * 10 + i) as u32;
        assert_eq!(count, expected, "Actor {} should have count {}", i, expected);
    }

    // Shutdown all actors
    for actor in &actors {
        actor.send_message(TestMessage::Shutdown).map_err(|e| SimulationError::RpcCall {
            message: format!("RPC call failed: {:?}", e),
        })?;
    }

    sleep(Duration::from_millis(200)).await;

    Ok(())
}

/// Test actor state transitions and Tiger Style bounded resources.
#[madsim::test]
async fn test_tiger_style_bounds() -> Result<()> {
    // Test that actor enforces Tiger Style resource bounds
    let args = TestActorArgs {
        initial_count: 0,
        max_count: 10, // Small max for testing
    };

    let (actor_ref, _) =
        Actor::spawn(Some("bounded-actor".to_string()), TestActor, args).await.context(ActorSpawnSnafu)?;

    // Try to exceed bounds
    for _ in 0..20 {
        actor_ref.send_message(TestMessage::Increment(1)).map_err(|e| SimulationError::RpcCall {
            message: format!("RPC call failed: {:?}", e),
        })?;
    }

    sleep(Duration::from_millis(300)).await;

    let count =
        ractor::call_t!(actor_ref, TestMessage::GetCount, Duration::from_secs(1).as_millis() as u64).map_err(|e| {
            SimulationError::RpcCall {
                message: format!("RPC call failed: {:?}", e),
            }
        })?;

    assert_eq!(count, 10, "Count should be capped at max_count");

    // Test saturation with large increment
    actor_ref.send_message(TestMessage::Increment(u32::MAX)).map_err(|e| SimulationError::RpcCall {
        message: format!("RPC call failed: {:?}", e),
    })?;

    sleep(Duration::from_millis(100)).await;

    let count =
        ractor::call_t!(actor_ref, TestMessage::GetCount, Duration::from_secs(1).as_millis() as u64).map_err(|e| {
            SimulationError::RpcCall {
                message: format!("RPC call failed: {:?}", e),
            }
        })?;

    assert_eq!(count, 10, "Count should still be capped at max_count");

    actor_ref.send_message(TestMessage::Shutdown).map_err(|e| SimulationError::RpcCall {
        message: format!("RPC call failed: {:?}", e),
    })?;

    Ok(())
}

/// Test actor message ordering and processing guarantees.
#[madsim::test]
async fn test_message_ordering() -> Result<()> {
    let args = TestActorArgs {
        initial_count: 0,
        max_count: 1000,
    };

    let (actor_ref, _) = Actor::spawn(Some("ordering-test-actor".to_string()), TestActor, args)
        .await
        .context(ActorSpawnSnafu)?;

    // Send multiple increments in order
    for i in 1..=10 {
        actor_ref.send_message(TestMessage::Increment(i)).map_err(|e| SimulationError::RpcCall {
            message: format!("RPC call failed: {:?}", e),
        })?;
    }

    // Wait for all messages to be processed
    sleep(Duration::from_millis(200)).await;

    // The sum of 1+2+3+...+10 = 55
    let count =
        ractor::call_t!(actor_ref, TestMessage::GetCount, Duration::from_secs(1).as_millis() as u64).map_err(|e| {
            SimulationError::RpcCall {
                message: format!("RPC call failed: {:?}", e),
            }
        })?;

    assert_eq!(count, 55, "All increments should be processed in order");

    actor_ref.send_message(TestMessage::Shutdown).map_err(|e| SimulationError::RpcCall {
        message: format!("RPC call failed: {:?}", e),
    })?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simulation_error_display() {
        let err = SimulationError::Timeout;
        assert_eq!(err.to_string(), "Timeout waiting for condition");

        let spawn_err = SimulationError::ActorSpawn {
            source: ractor::SpawnErr::StartupFailed("test".into()),
        };
        assert!(spawn_err.to_string().contains("Actor spawn failed"));
    }
}
