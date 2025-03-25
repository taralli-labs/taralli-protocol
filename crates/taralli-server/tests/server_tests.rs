use std::sync::Arc;

use axum::Router;
use common::fixtures::provider_fixture;
use hyper::StatusCode;
use rstest::*;
use serde_json::{json, Value};
use serial_test::serial;
use taralli_client::api::{submit::SubmitApiClient, subscribe::SubscribeApiClient};
use taralli_primitives::{
    compression_utils::{
        compression,
        intents::{ComputeRequestCompressed, PartialComputeRequest},
    },
    intents::request::ComputeRequest,
    systems::{SystemId, SystemParams},
};
use taralli_server::subscription_manager::{BroadcastedMessage, SubscriptionManager};
use tokio::net::TcpListener;
use tokio_stream::StreamExt;
use url::Url;
mod common;
use crate::common::fixtures::{requester_fixture, risc0_request_fixture, setup_app};
use futures::FutureExt;

#[tokio::test]
#[rstest]
#[serial]
async fn test_submit_with_no_subscribers(
    requester_fixture: SubmitApiClient,
    risc0_request_fixture: ComputeRequest<SystemParams>,
) {
    let response = requester_fixture
        .submit_intent(risc0_request_fixture)
        .await
        .expect("Couldn't submit");
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    let response_body: Value = response.json().await.unwrap();
    assert_eq!(
        response_body,
        json!({"error": "No proof providers available"})
    );
}

#[tokio::test]
#[rstest]
#[serial]
async fn test_broadcast_single(
    requester_fixture: SubmitApiClient,
    provider_fixture: SubscribeApiClient,
    risc0_request_fixture: ComputeRequest<SystemParams>,
) {
    let _subscription = provider_fixture
        .subscribe_to_markets()
        .await
        .expect("Couldn't subscribe");
    let response = requester_fixture
        .submit_intent(risc0_request_fixture)
        .await
        .expect("Couldn't submit");
    assert_eq!(response.status(), StatusCode::OK);
    let response_body: Value = response.json().await.unwrap();
    assert_eq!(
        response_body,
        json!({
            "message": "compute request broadcast to providers",
            "broadcast_receivers": 1
        })
    );
}

#[tokio::test]
#[rstest]
#[serial]
// RsTest won't let us fixture two providers, so we just call it below normally for this one.
async fn test_broadcast_multiple(
    requester_fixture: SubmitApiClient,
    risc0_request_fixture: ComputeRequest<SystemParams>,
) {
    let _subscription = provider_fixture()
        .subscribe_to_markets()
        .await
        .expect("Couldn't subscribe");
    let _other_sub = provider_fixture()
        .subscribe_to_markets()
        .await
        .expect("Couldn't subscribe");
    let response = requester_fixture
        .submit_intent(risc0_request_fixture)
        .await
        .expect("Couldn't submit");
    assert_eq!(response.status(), StatusCode::OK);
    let response_body: Value = response.json().await.unwrap();
    assert_eq!(
        response_body,
        json!({
            "message": "compute request broadcast to providers",
            "broadcast_receivers": 2
        })
    );
}

#[tokio::test]
#[rstest]
#[serial]
async fn test_broadcast_dropped_subscriber(
    requester_fixture: SubmitApiClient,
    provider_fixture: SubscribeApiClient,
    risc0_request_fixture: ComputeRequest<SystemParams>,
) {
    let subscription = provider_fixture
        .subscribe_to_markets()
        .await
        .expect("Couldn't subscribe");
    let response = requester_fixture
        .submit_intent(risc0_request_fixture.clone())
        .await
        .expect("Couldn't submit");
    assert_eq!(response.status(), StatusCode::OK);
    let response_body: Value = response.json().await.unwrap();
    assert_eq!(
        response_body,
        json!({
            "message": "compute request broadcast to providers",
            "broadcast_receivers": 1
        })
    );
    drop(subscription);
    let response = requester_fixture
        .submit_intent(risc0_request_fixture)
        .await
        .expect("Couldn't submit");
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    let response_body: Value = response.json().await.unwrap();
    assert_eq!(
        response_body,
        json!({"error": "No proof providers available"})
    );
}

#[tokio::test]
#[rstest]
#[serial]
// We test that proof requests are broadcasted to the correct providers.
// The Arkworks provider only listens for Arkworks requests, and the Risc0 provider only listens for Risc0 requests.
async fn test_broadcast_with_specific_proving_systems(
    requester_fixture: SubmitApiClient,
    mut risc0_request_fixture: ComputeRequest<SystemParams>,
) {
    let subscribe_url = Url::parse("http://localhost:8080").unwrap();

    let provider_arkworks =
        SubscribeApiClient::new(subscribe_url.clone(), SystemId::Arkworks.as_bit());

    let provider_risc0 = SubscribeApiClient::new(subscribe_url.clone(), SystemId::Risc0.as_bit());

    let provider_arkworks_risc0 = SubscribeApiClient::new(
        subscribe_url,
        SystemId::Risc0.as_bit() | SystemId::Arkworks.as_bit(),
    );

    let mut subscription_arkworks = provider_arkworks
        .subscribe_to_markets()
        .await
        .expect("Couldn't subscribe");
    let mut subscription_risc0 = provider_risc0
        .subscribe_to_markets()
        .await
        .expect("Couldn't subscribe");
    let mut subscription_arkworks_risc0 = provider_arkworks_risc0
        .subscribe_to_markets()
        .await
        .expect("Couldn't subscribe");

    // Let's submit 2 requests, each with a different proving system.
    let mut response = requester_fixture
        .submit_intent(risc0_request_fixture.clone())
        .await
        .expect("Couldn't submit");
    assert_eq!(response.status(), StatusCode::OK);
    // Change the req proving system type and resubmit.
    risc0_request_fixture.system_id = SystemId::Arkworks;
    response = requester_fixture
        .submit_intent(risc0_request_fixture)
        .await
        .expect("Couldn't submit");
    assert_eq!(response.status(), StatusCode::OK);

    // We assert that the Arkworks provider received only the Arkworks request, despite the submission of a Risc0 request.
    // Await for the first request from the Arkworks provider. Rest should have arrived afterwards.
    // We need to await because github actions aren't beefy enough to handle the load and sometimes now_or_never() fails.
    let arkworks_message = subscription_arkworks
        .next()
        .await
        .expect("No Arkworks request received")
        .unwrap();
    assert_eq!(arkworks_message.system_id, SystemId::Arkworks);
    // assert!(subscription_arkworks.peek
    assert!(subscription_arkworks
        .peekable()
        .peek()
        .now_or_never()
        .is_none());

    // Same logic as above, but for Risc0.
    let risc0_message = subscription_risc0
        .next()
        .now_or_never()
        .expect("Couldn't get Risc0 request from stream")
        .expect("No Risc0 request received")
        .unwrap();
    assert_eq!(risc0_message.system_id, SystemId::Risc0);
    assert!(subscription_risc0
        .peekable()
        .peek()
        .now_or_never()
        .is_none());

    // Finally, assert the provider subscribed to both proving systems has received both requests.
    for i in 0..2 {
        let message = subscription_arkworks_risc0
            .next()
            .now_or_never()
            .unwrap_or_else(|| panic!("Missing request {i} from stream"))
            .expect("No request received")
            .unwrap();
        if i == 0 {
            assert_eq!(message.system_id, SystemId::Risc0);
        } else {
            assert_eq!(message.system_id, SystemId::Arkworks);
        }
    }
    assert!(subscription_arkworks_risc0
        .peekable()
        .peek()
        .now_or_never()
        .is_none());
}

#[tokio::test]
#[rstest]
#[serial]
async fn test_reconnect_dropped_subscriber(
    requester_fixture: SubmitApiClient,
    provider_fixture: SubscribeApiClient,
    risc0_request_fixture: ComputeRequest<SystemParams>,
) {
    let subscription = provider_fixture
        .subscribe_to_markets()
        .await
        .expect("Couldn't subscribe");
    let response = requester_fixture
        .submit_intent(risc0_request_fixture.clone())
        .await
        .expect("Couldn't submit");
    assert_eq!(response.status(), StatusCode::OK);
    let response_body: Value = response.json().await.unwrap();
    assert_eq!(
        response_body,
        json!({
            "message": "compute request broadcast to providers",
            "broadcast_receivers": 1
        })
    );

    drop(subscription);
    let mut subscription = provider_fixture
        .subscribe_to_markets()
        .await
        .expect("Couldn't subscribe");
    let response = requester_fixture
        .submit_intent(risc0_request_fixture.clone())
        .await
        .expect("Couldn't submit");
    assert_eq!(response.status(), StatusCode::OK);
    let response_body: Value = response.json().await.unwrap();
    assert_eq!(
        response_body,
        json!({
            "message": "compute request broadcast to providers",
            "broadcast_receivers": 1
        })
    );
    let message = subscription
        .next()
        .now_or_never()
        .expect("Couldn't get message from stream")
        .expect("No message received");
    assert!(message.is_ok());
}

#[tokio::test]
#[rstest]
#[serial]
// Assert multiple concurrent requests are broadcasted to all subscribers.
// If you check the server's logs you'll see that, due to how fast we're submitting requests, some subscribers will lag.
// This test ensures that all subscribers receive all requests, even if they lag behind.
// When each request is received by the subscriptions below, providers/subscribers will have to deserialize, decompress and so on, hence why they might lag.
// Maybe if you're running this on a really fast machine, there won't be any lag. in which case we should increase the number of requests.
// If this test is failing repeatedly, it might be worth checking the default values for the subscription manager's buffer on `subscription_manager.rs`.
async fn test_multiple_concurrent_requests_with_multiple_subscribers_which_can_lag(
    requester_fixture: SubmitApiClient,
    risc0_request_fixture: ComputeRequest<SystemParams>,
) {
    let num_requests = 10;
    let mut subscription1 = provider_fixture()
        .subscribe_to_markets()
        .await
        .expect("Couldn't subscribe provider 1");
    let mut subscription2 = provider_fixture()
        .subscribe_to_markets()
        .await
        .expect("Couldn't subscribe provider 2");

    // Spawn multiple concurrent tasks that submit requests and then submit it.
    let submit_tasks: Vec<_> = (0..num_requests)
        .map(|_| {
            let value = risc0_request_fixture.clone();
            let client = &requester_fixture;
            async move {
                let response = client
                    .submit_intent(value.clone())
                    .await
                    .expect("Submission failed");
                assert_eq!(response.status(), StatusCode::OK);
            }
        })
        .collect();
    futures::future::join_all(submit_tasks).await;

    // Stop execution for a bit via await so server can process requests received on `submit.rs`
    // Collect all messages.
    let mut messages1 = Vec::new();
    let mut messages2 = Vec::new();
    for i in 0..num_requests {
        let msg1 = subscription1
            .next()
            .await
            .unwrap_or_else(|| panic!("Missing request {i} from stream 1"))
            .expect("Subscription 1 stream ended unexpectedly");
        messages1.push(msg1);

        let msg2 = subscription2
            .next()
            .await
            .unwrap_or_else(|| panic!("Missing request {i} from stream 2"))
            .expect("Subscription 2 stream ended unexpectedly");
        messages2.push(msg2);
    }

    // Assert that the number of messages equals the number of submissions.
    assert_eq!(messages1.len(), num_requests);
    assert_eq!(messages2.len(), num_requests);
}

#[tokio::test]
#[rstest]
// Assert someone subscribed with wrong proving system id masks won't actually keep a connection open.
async fn test_invalid_proving_system_id(mut provider_fixture: SubscribeApiClient) {
    provider_fixture.subscribed_to = 0b10000000; // Invalid proving system id as of now.
    assert!(provider_fixture.subscribe_to_markets().await.is_err());
}

#[tokio::test]
#[rstest]
#[serial]
// Assert that the subscriber can handle wrong/corrupted data.
// We can't just submit it to the server directly via requester, because the server would reject it.
// We instantiate a server with a subscription manager.
// Looking at `subscribe.rs` you'll see the subscription manager is the one that broadcasts messages across websocket connections.
async fn test_corrupted_data_on_subscribe(
    setup_app: (Router, Arc<SubscriptionManager>),
    risc0_request_fixture: ComputeRequest<SystemParams>,
) {
    let port = 8888;
    let server_url = format!("0.0.0.0:{port}");
    let listener = TcpListener::bind(server_url)
        .await
        .expect("Couldn't bind server");
    let server_handle = tokio::spawn(async move {
        axum::serve(listener, setup_app.0)
            .await
            .expect("Couldn't serve");
    });

    let partial_request = PartialComputeRequest {
        system_id: risc0_request_fixture.system_id,
        proof_request: risc0_request_fixture.proof_request,
        signature: risc0_request_fixture.signature,
    };
    let proving_system_information_bytes = compression::compress_brotli(
        &serde_json::to_vec(&risc0_request_fixture.system)
            .expect("Couldn't serialize proving system information"),
    )
    .expect("Couldn't compress proving system information");
    // This bit below is done under the submit.rs file.
    let request_compressed = ComputeRequestCompressed::from((
        partial_request.clone(),
        proving_system_information_bytes.clone(),
    ));
    let request_serialized = bincode::serialize(&request_compressed)
        .expect("Couldn't serialize request for BroadcastedMessage");
    let message_to_broadcast = BroadcastedMessage {
        content: request_serialized,
        subscribed_to: partial_request.system_id.as_bit(),
    };

    // Let's add some bogus data to proving_system_information_bytes so we can check how the subscriber handles it.
    let request_compressed = ComputeRequestCompressed::from((
        partial_request.clone(),
        proving_system_information_bytes
            .into_iter()
            .map(|byte| byte ^ 0b10101010)
            .collect(),
    ));
    let corrupted_serialized = bincode::serialize(&request_compressed)
        .expect("Couldn't serialize corrupted request for BroadcastedMessage");
    let message_to_broadcast_corrupted = BroadcastedMessage {
        content: corrupted_serialized,
        subscribed_to: partial_request.system_id.as_bit(),
    };

    // let mut subscription = ProviderApi::new(ApiConfig {
    //     server_url: Url::parse(&format!("http://localhost:{}", port)).unwrap(),
    //     request_timeout: 0,
    //     max_retries: 0,
    //     subscribed_to: ProvingSystemId::Risc0.as_bit(),
    // })

    let mut subscription = SubscribeApiClient::new(
        Url::parse(&format!("http://localhost:{port}")).unwrap(),
        SystemId::Risc0.as_bit(),
    )
    .subscribe_to_markets()
    .await
    .expect("Couldn't subscribe provider");

    setup_app
        .1
        .broadcast(message_to_broadcast_corrupted)
        .expect("Couldn't broadcast");

    setup_app
        .1
        .broadcast(message_to_broadcast)
        .expect("Couldn't broadcast");

    assert!(subscription
        .next()
        .await
        .expect("No message received")
        .is_err());
    assert!(subscription
        .next()
        .await
        .expect("No message received")
        .is_ok());

    server_handle.abort();
}
