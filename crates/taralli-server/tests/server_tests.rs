use std::collections::HashSet;

use common::fixtures::provider_fixture;
use hyper::StatusCode;
use rstest::*;
use serde_json::{json, Value};
use serial_test::serial;
use taralli_primitives::systems::ProvingSystemId;
use taralli_provider::{api::ProviderApi, config::ApiConfig};
use tokio_stream::StreamExt;
mod common;
use crate::common::fixtures::{request_fixture, requester_fixture};
use futures::FutureExt;
use taralli_requester::api::RequesterApi;

#[tokio::test]
#[rstest]
#[serial]
async fn test_submit_with_no_subscribers(requester_fixture: RequesterApi) {
    let request = request_fixture().await;
    let response = requester_fixture
        .submit_request(request)
        .await
        .expect("Couldn't submit");
    assert_eq!(response.status(), StatusCode::OK);
    let response_body: Value = response.json().await.unwrap();
    assert_eq!(
        response_body["message"],
        json!("No providers subscribed to listen for this request.")
    );
}

#[tokio::test]
#[rstest]
#[serial]
async fn test_broadcast_single(requester_fixture: RequesterApi, provider_fixture: ProviderApi) {
    let request = request_fixture().await;
    let mut subscription = provider_fixture
        .subscribe_to_markets()
        .await
        .expect("Couldn't subscribe");
    let response = requester_fixture
        .submit_request(request)
        .await
        .expect("Couldn't submit");
    assert_eq!(response.status(), StatusCode::OK);
    let response_body: Value = response.json().await.unwrap();
    assert_eq!(
        response_body,
        json!({
            "message": "Proof request accepted and submitted to Proof Providers.",
            "broadcasted_to": 1
        })
    );
    let message = subscription.next().now_or_never().unwrap().unwrap();
    assert!(message.is_ok());
}

#[tokio::test]
#[rstest]
#[serial]
// RsTest won't let us fixture two providers, so we just call it below normally for this one.
async fn test_broadcast_multiple(requester_fixture: RequesterApi) {
    let request = request_fixture().await;
    let _subscription = provider_fixture()
        .subscribe_to_markets()
        .await
        .expect("Couldn't subscribe");
    let _other_sub = provider_fixture()
        .subscribe_to_markets()
        .await
        .expect("Couldn't subscribe");
    let response = requester_fixture
        .submit_request(request)
        .await
        .expect("Couldn't submit");
    assert_eq!(response.status(), StatusCode::OK);
    let response_body: Value = response.json().await.unwrap();
    assert_eq!(
        response_body,
        json!({
            "message": "Proof request accepted and submitted to Proof Providers.",
            "broadcasted_to": 2
        })
    )
}

#[tokio::test]
#[rstest]
#[serial]
async fn test_broadcast_dropped_subscriber(
    requester_fixture: RequesterApi,
    provider_fixture: ProviderApi,
) {
    let request = request_fixture().await;
    let subscription = provider_fixture
        .subscribe_to_markets()
        .await
        .expect("Couldn't subscribe");
    let response = requester_fixture
        .submit_request(request.clone())
        .await
        .expect("Couldn't submit");
    assert_eq!(response.status(), StatusCode::OK);
    let response_body: Value = response.json().await.unwrap();
    assert_eq!(
        response_body,
        json!({
            "message": "Proof request accepted and submitted to Proof Providers.",
            "broadcasted_to": 1
        })
    );
    drop(subscription);
    let response = requester_fixture
        .submit_request(request)
        .await
        .expect("Couldn't submit");
    assert_eq!(response.status(), StatusCode::OK);
    let response_body: Value = response.json().await.unwrap();
    assert_eq!(
        response_body["message"],
        json!("No providers subscribed to listen for this request.")
    );
}

#[tokio::test]
#[rstest]
#[serial]
async fn test_reconnect_dropped_subscriber(
    requester_fixture: RequesterApi,
    provider_fixture: ProviderApi,
) {
    let request = request_fixture().await;
    let subscription = provider_fixture
        .subscribe_to_markets()
        .await
        .expect("Couldn't subscribe");
    let response = requester_fixture
        .submit_request(request.clone())
        .await
        .expect("Couldn't submit");
    assert_eq!(response.status(), StatusCode::OK);
    let response_body: Value = response.json().await.unwrap();
    assert_eq!(
        response_body,
        json!({
            "message": "Proof request accepted and submitted to Proof Providers.",
            "broadcasted_to": 1
        })
    );
    drop(subscription);
    let mut subscription = provider_fixture
        .subscribe_to_markets()
        .await
        .expect("Couldn't subscribe");
    let response = requester_fixture
        .submit_request(request)
        .await
        .expect("Couldn't submit");
    assert_eq!(response.status(), StatusCode::OK);
    let response_body: Value = response.json().await.unwrap();
    assert_eq!(
        response_body,
        json!({
            "message": "Proof request accepted and submitted to Proof Providers.",
            "broadcasted_to": 1
        })
    );
    let message = subscription.next().now_or_never().unwrap().unwrap();
    assert!(message.is_ok());
}

#[tokio::test]
#[rstest]
#[serial]
// We test that proof requests are broadcasted to the correct providers.
// The Arkworks provider only listens for Arkworks requests, and the Risc0 provider only listens for Risc0 requests.
async fn test_broadcast_with_specific_proving_systems(requester_fixture: RequesterApi) {
    let provider_arkworks = ProviderApi::new(ApiConfig {
        subscribed_to: ProvingSystemId::Arkworks.as_bit(),
        ..Default::default()
    });

    let provider_risc0 = ProviderApi::new(ApiConfig {
        subscribed_to: ProvingSystemId::Risc0.as_bit(),
        ..Default::default()
    });

    let provider_arkworks_risc0 = ProviderApi::new(ApiConfig {
        subscribed_to: ProvingSystemId::Arkworks.as_bit() | ProvingSystemId::Risc0.as_bit(),
        ..Default::default()
    });
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
    let mut request = request_fixture().await;
    let mut response = requester_fixture
        .submit_request(request.clone())
        .await
        .expect("Couldn't submit");
    assert_eq!(response.status(), StatusCode::OK);
    // Change the req proving system type and resubmit.
    request.proving_system_id = ProvingSystemId::Arkworks;
    response = requester_fixture
        .submit_request(request)
        .await
        .expect("Couldn't submit");
    assert_eq!(response.status(), StatusCode::OK);

    // We assert that the Arkworks provider received only the Arkworks request, despite the submission of a Risc0 request.
    // The timeout is in case this test becomes broken. This way it won't hang forever.
    let arkworks_message = subscription_arkworks
        .next()
        .now_or_never()
        .expect("Couldn't get Arkworks request from stream")
        .expect("No Arkworks request received")
        .unwrap();
    assert_eq!(
        arkworks_message.proving_system_id,
        ProvingSystemId::Arkworks
    );
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
    assert_eq!(risc0_message.proving_system_id, ProvingSystemId::Risc0);
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
            .expect(format!("Missing request {} from stream", i).as_str())
            .expect("No request received")
            .unwrap();
        if i == 0 {
            assert_eq!(message.proving_system_id, ProvingSystemId::Risc0);
        } else {
            assert_eq!(message.proving_system_id, ProvingSystemId::Arkworks);
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
// Assert multiple concurrent requests are broadcasted to all subscribers.
async fn test_multiple_concurrent_requests_with_multiple_subscribers(
    requester_fixture: RequesterApi,
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
            let requester_fixture = requester_fixture.clone();
            async move {
                let request = request_fixture().await;
                let response = requester_fixture
                    .submit_request(request)
                    .await
                    .expect("Submission failed");
                assert_eq!(response.status(), StatusCode::OK);
            }
        })
        .collect();
    futures::future::join_all(submit_tasks).await;

    // Collect all messages.
    let mut messages1 = Vec::new();
    let mut messages2 = Vec::new();
    for _ in 0..num_requests {
        let msg1 = subscription1
            .next()
            .await
            .expect("Subscription 1 stream ended unexpectedly")
            .unwrap();
        messages1.push(msg1);

        let msg2 = subscription2
            .next()
            .await
            .expect("Subscription 2 stream ended unexpectedly")
            .unwrap();
        messages2.push(msg2);
    }

    // Assert that the number of messages equals the number of submissions.
    assert_eq!(messages1.len(), num_requests);
    assert_eq!(messages2.len(), num_requests);

    // We generate tokens randomly via fixture, hence why we can safely check for the size of the sets.
    let unique_signatures1: HashSet<_> = messages1
        .into_iter()
        .map(|msg| msg.onchain_proof_request.token)
        .collect();
    let unique_signatures2: HashSet<_> = messages2
        .into_iter()
        .map(|msg| msg.onchain_proof_request.token)
        .collect();
    assert_eq!(
        unique_signatures1.len(),
        num_requests,
        "Duplicate messages detected in subscription 1"
    );
    assert_eq!(
        unique_signatures2.len(),
        num_requests,
        "Duplicate messages detected in subscription 2"
    );
}
