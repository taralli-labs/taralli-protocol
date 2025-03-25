use std::{sync::LazyLock, time::Duration};

use crate::common::fixtures::groth16_request_fixture;
use rstest::rstest;
use serial_test::serial;
use taralli_primitives::{
    compression_utils::{
        compression,
        intents::{ComputeRequestCompressed, PartialComputeRequest},
    },
    intents::request::ComputeRequest,
    systems::SystemParams,
};
use taralli_server::subscription_manager::{BroadcastedMessage, SubscriptionManager};
use tokio::time::sleep;

pub mod common;
#[test]
/// Ensures a broadcast is sending data correctly.
fn should_broadcast() {
    let subscription_manager: SubscriptionManager<i32> = SubscriptionManager::default();
    let mut recv = subscription_manager.add_subscription();
    subscription_manager.broadcast(1).unwrap();
    assert_eq!(Some(1), Some(recv.blocking_recv().unwrap()));
}

#[test]
/// Ensures we error out
fn should_not_broadcast_without_receivers() {
    let subscription_manager: SubscriptionManager<i32> = SubscriptionManager::default();
    assert!(subscription_manager.broadcast(1).is_err());
}

#[test]
/// Ensures only that the 0th item won't be sent to receivers when the buffer is full.
fn should_receive_lagged() {
    let subscription_manager: SubscriptionManager<i32> = SubscriptionManager::new(1);
    assert!(subscription_manager.broadcast(1).is_err());
    let mut r1 = subscription_manager.add_subscription();
    subscription_manager.broadcast(2).unwrap();
    assert_eq!(Some(2), Some(r1.blocking_recv().unwrap()));
}

#[test]
/// Ensures multiple broadcasts are possible to the same consumer for a given buffer size.
fn should_broadcast_multiple_times() {
    let subscription_manager: SubscriptionManager<i32> = SubscriptionManager::new(10);
    let mut recv = subscription_manager.add_subscription();
    for i in 0..10 {
        subscription_manager.broadcast(i).unwrap();
    }
    for i in 0..10 {
        assert_eq!(Some(i), Some(recv.blocking_recv().unwrap()));
    }
}

#[test]
/// Ensures multiple subscribers are indeed receiving messages.
fn should_broadcast_multiple_times_to_many_subscribers() {
    let subscription_manager: SubscriptionManager<i32> = SubscriptionManager::new(10);
    let mut r1 = subscription_manager.add_subscription();
    let mut r2 = subscription_manager.add_subscription();
    for i in 0..10 {
        subscription_manager.broadcast(i).unwrap();
        assert_eq!(Some(i), Some(r1.blocking_recv().unwrap()));
        assert_eq!(Some(i), Some(r2.blocking_recv().unwrap()));
    }
}

#[test]
/// Ensures that upon the removal of the receiver (imagine if client connection drops), we no longer keep it alive.
/// That because `sender.send()` should fail, causing the subscriber removal.
fn should_drop_receivers() {
    let subscription_manager: SubscriptionManager<i32> = SubscriptionManager::default();
    let recv = subscription_manager.add_subscription();
    drop(recv);
    // `broadcast()` to no recvs returns errored.
    let _ = subscription_manager.broadcast(1).unwrap_err();
    assert_eq!(subscription_manager.active_subscriptions(), 0);
}

#[test]
/// Ensures the removal of one subscriber is not affecting the next one.
fn should_continue_broadcast_after_subscriber_removed() {
    let subscription_manager: SubscriptionManager<i32> = SubscriptionManager::default();
    let r1 = subscription_manager.add_subscription();
    let mut r2 = subscription_manager.add_subscription();
    // Simulate one client disconnection and broadcast
    drop(r1);
    subscription_manager.broadcast(1).unwrap();
    // Verify that the remaining subscriber receives the message
    assert_eq!(Some(1), Some(r2.blocking_recv().unwrap()));
    // Ensure that the dropped subscriber was removed
    assert_eq!(subscription_manager.active_subscriptions(), 1);
}

#[tokio::test]
#[rstest]
#[serial]
/// Ensure that we're able to handle buffering of messages when the subscriber is lagging.
/// This is more of a stress test rather than a regular unit test, but can be useful to explore limits of the machine running the server.
/// Here, we send the sha256_512 r1cs request, which is known to be large. A 1000 of it is just a sensible number that we might use in production.
/// You'd also expect, in a production environment, that the lagged messages won't all be of the same size, so this is a bit of a worst-case scenario.
async fn should_handle_lagging_subscribers(groth16_request_fixture: ComputeRequest<SystemParams>) {
    // We create the subscription manager with a buffer of 1000 messages.
    static SUBSCRIPTION_MANAGER: LazyLock<SubscriptionManager<BroadcastedMessage>> =
        LazyLock::new(|| SubscriptionManager::new(1000));

    // Here we build the format of the message which is sent and broadcasted around the server.
    // This bit is done on api.rs, requester side to build the http multipart request.
    let partial_request = PartialComputeRequest {
        system_id: groth16_request_fixture.system_id,
        proof_request: groth16_request_fixture.proof_request,
        signature: groth16_request_fixture.signature,
    };
    let proving_system_information_bytes = compression::compress_brotli(
        &serde_json::to_vec(&groth16_request_fixture.system)
            .expect("Couldn't serialize proving system information"),
    )
    .expect("Couldn't compress proving system information");
    // This bit below is done under the submit.rs file.
    let request_compressed =
        ComputeRequestCompressed::from((partial_request.clone(), proving_system_information_bytes));
    let request_serialized = bincode::serialize(&request_compressed)
        .expect("Couldn't serialize request for BroadcastedMessage");
    let message_to_broadcast = BroadcastedMessage {
        content: request_serialized,
        subscribed_to: partial_request.system_id.as_bit(),
    };

    // Once we have the compressed and serialized message, we broadcast it.
    // We spawn a separate thread to do it so we can actually simulate a producer and a subscriber.
    let mgr = &SUBSCRIPTION_MANAGER;
    let messages_produced = 1000;
    tokio::spawn(async move {
        // Pause for a bit so we can execute `add_subscription` before we start broadcasting.
        sleep(Duration::from_millis(10)).await;
        for i in 0..messages_produced {
            let broadcasted_to = mgr
                .broadcast(message_to_broadcast.clone())
                .unwrap_or_else(|_| panic!("Couldn't broadcast {i} message"));
            assert!(broadcasted_to == 1);
        }
    });

    // We add a subscriber and immediately stop it so we simulate a lagging subscriber.
    // You can tweat this time based on how many messages we're sent above.
    let mut rx = SUBSCRIPTION_MANAGER.add_subscription();
    sleep(Duration::from_millis(1000)).await;

    // Here we see that the buffer indeed is full and none of the messages have been broadcasted.
    assert!(SUBSCRIPTION_MANAGER.buffer_len() == messages_produced);
    for _ in 0..messages_produced {
        assert!(rx.recv().await.is_ok());
    }

    assert!(SUBSCRIPTION_MANAGER.buffer_len() == 0);
}
