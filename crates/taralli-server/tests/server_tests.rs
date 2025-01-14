use axum::body::{to_bytes, BodyDataStream};
use common::helpers::{setup_app, submit, subscribe, MAX_BODY_SIZE};
use futures::{stream::MapOk, StreamExt};
use hyper::StatusCode;
use serde_json::{json, Value};

mod common;

#[tokio::test]
async fn test_submit_with_no_subscribers() {
    let app = setup_app(None).await;
    let response = submit(app.clone(), None).await;
    assert_eq!(response.status(), StatusCode::ACCEPTED);
    let body = to_bytes(response.into_body(), MAX_BODY_SIZE).await.unwrap();
    let body: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(
        body["message"],
        json!("Request accepted, but there were no receivers to submit to.")
    );
    assert!(body["broadcast_error"].is_string());
}

#[tokio::test]
async fn test_brodcast_over_with_no_more_subscribers() {
    // Setup broadcast queue with size 1.
    let app = setup_app(None).await;
    let sse_stream = subscribe(app.clone()).await;

    // Submit two different messages
    let submit_response_1 = submit(app.clone(), Some(r#"{"message": "first"}"#.to_string())).await;
    assert_eq!(submit_response_1.status(), StatusCode::OK);
    drop(sse_stream);
    let submit_response_2 = submit(app.clone(), Some(r#"{"message": "second"}"#.to_string())).await;
    assert_eq!(submit_response_2.status(), StatusCode::ACCEPTED);
    let body = to_bytes(submit_response_2.into_body(), MAX_BODY_SIZE)
        .await
        .unwrap();
    let body: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(
        body["message"],
        json!("Request accepted, but there were no receivers to submit to.")
    );
    assert!(body["broadcast_error"].is_string());
}

#[tokio::test]
async fn test_broadcast_single() {
    let app = setup_app(None).await;
    let mut sse_stream = subscribe(app.clone()).await;
    let submit_response = submit(app.clone(), None).await;
    assert_eq!(submit_response.status(), StatusCode::OK);
    if let Ok(event_data) = sse_stream
        .next()
        .await
        .expect("No event received on subscription stream")
    {
        assert_eq!(event_data, "data: {}\n\n");
    } else {
        panic!("No event received on subscription stream");
    }
}

#[tokio::test]
async fn test_single_submit_and_multiple_subscribers() {
    let app = setup_app(None).await;
    let mut subscribers: Vec<MapOk<BodyDataStream, _>> = vec![
        subscribe(app.clone()).await,
        subscribe(app.clone()).await,
        subscribe(app.clone()).await,
    ];

    let submit_response = submit(app.clone(), None).await;
    assert_eq!(submit_response.status(), StatusCode::OK);

    // Loop over each subscriber and check if it received the expected data
    for sse_stream in subscribers.iter_mut() {
        if let Some(Ok(event_data)) = sse_stream.next().await {
            assert_eq!(event_data, "data: {}\n\n");
        } else {
            panic!("No event received on subscription stream");
        }
    }
}

#[tokio::test]
async fn test_multiple_submitters_and_subscribers() {
    // Setup broadcast queue with size 2.
    let app = setup_app(Some(2)).await;
    let mut subscribers: Vec<MapOk<BodyDataStream, _>> = vec![
        subscribe(app.clone()).await,
        subscribe(app.clone()).await,
        subscribe(app.clone()).await,
    ];

    // Submit two different messages
    let submit_response_1 = submit(app.clone(), Some(r#"{"message": "first"}"#.to_string())).await;
    assert_eq!(submit_response_1.status(), StatusCode::OK);
    let submit_response_2 = submit(app.clone(), Some(r#"{"message": "second"}"#.to_string())).await;
    assert_eq!(submit_response_2.status(), StatusCode::OK);

    // Loop over each subscriber and check if they received both messages
    for sse_stream in subscribers.iter_mut() {
        // First message
        if let Some(Ok(event_data)) = sse_stream.next().await {
            assert_eq!(event_data, "data: {\"message\":\"first\"}\n\n");
        } else {
            panic!("No event received for the first message");
        }

        // Second message
        if let Some(Ok(event_data)) = sse_stream.next().await {
            assert_eq!(event_data, "data: {\"message\":\"second\"}\n\n");
        } else {
            panic!("No event received for the second message");
        }
    }
}
