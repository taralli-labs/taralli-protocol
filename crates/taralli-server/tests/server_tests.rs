use common::fixtures::provider_fixture;
use hyper::StatusCode;
use rstest::*;
use serde_json::{json, Value};
use serial_test::serial;
use taralli_provider::api::ProviderApi;
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
