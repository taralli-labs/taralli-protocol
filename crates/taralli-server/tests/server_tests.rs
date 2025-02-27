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
    let _subscription = provider_fixture
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
    )
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
