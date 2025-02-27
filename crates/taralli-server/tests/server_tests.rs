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
// We test that proof requests are broadcasted to the correct providers.
// The Arkworks provider only listens for Arkworks requests, and the Risc0 provider only listens for Risc0 requests.
async fn test_broadcast_with_specific_proving_systems(
    requester_fixture: RequesterApi,
) {
    let provider_arkworks = ProviderApi::new(ApiConfig {
        server_url: "http://localhost:8000".parse().unwrap(),
        request_timeout: 30,
        max_retries: 3,
        subscribed_to: ProvingSystemId::Arkworks.as_bit(),
    });
    let provider_risc0 = ProviderApi::new(ApiConfig {
        server_url: "http://localhost:8000".parse().unwrap(),
        request_timeout: 30,
        max_retries: 3,
        subscribed_to: ProvingSystemId::Risc0.as_bit(),
    });
    let provider_arkworks_risc0 = ProviderApi::new(ApiConfig {
        server_url: "http://localhost:8000".parse().unwrap(),
        request_timeout: 30,
        max_retries: 3,
        subscribed_to: ProvingSystemId::Arkworks.as_bit() | ProvingSystemId::Risc0.as_bit(),
    });
    let mut request = request_fixture().await;
    let mut subscription_arkworks = provider_arkworks
        .subscribe_to_markets()
        .await
        .expect("Couldn't subscribe");
    let mut subscription_risc0 = provider_risc0
        .subscribe_to_markets()
        .await
        .expect("Couldn't subscribe");
    let mut response = requester_fixture
        .submit_request(request.clone())
        .await
        .expect("Couldn't submit");
    assert_eq!(response.status(), StatusCode::OK);
    // let subscription.
    request.proving_system_id = ProvingSystemId::Arkworks;
    response = requester_fixture
    .submit_request(request)
    .await
    .expect("Couldn't submit");
    while let Some(result) = subscription_arkworks.next().await {
        match result {
            Ok(res) => {
                assert_eq!(res.proving_system_id, ProvingSystemId::Arkworks);
                break;
            }
            _ => {}
        }
    }
    while let Some(result) = subscription_risc0.next().await {
        match result {
            Ok(res) => {
                assert_eq!(res.proving_system_id, ProvingSystemId::Risc0);
                break;
            }
            _ => {}
        }
    }
    for i in 0..2 {
        let result = subscription_arkworks.next().await.unwrap();
        if i == 0 {
            match result {
                Ok(res) => {
                    assert_eq!(res.proving_system_id, ProvingSystemId::Risc0);
                }
                _ => {}
            }
        } else {
            match result {
                Ok(res) => {
                    assert_eq!(res.proving_system_id, ProvingSystemId::Arkworks);
                    break;
                }
                _ => {}
            }
        }
    }
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
