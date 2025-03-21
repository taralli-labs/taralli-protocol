use std::time::Duration;

use axum::body::to_bytes;
use common::fixtures::{setup_app, submit, subscribe, MAX_BODY_SIZE};
use futures::Stream;
use hyper::StatusCode;
use serde_json::{json, Value};
use taralli_primitives::systems::SystemId;
use tokio_stream::StreamExt;

mod common;

#[tokio::test]
async fn test_submit_with_no_subscribers() {
    let router = setup_app(None).await;

    let arkworks_msg = json!({
        "proving_system_id": "arkworks",
        "message": "test1"
    })
    .to_string();

    let response = submit(router.clone(), arkworks_msg).await;
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
async fn test_broadcast_single() {
    let router = setup_app(None).await;
    let _stream = subscribe(router, &[SystemId::Arkworks]).await;
}

#[tokio::test]
async fn test_broadcast_over_with_no_more_subscribers() {
    // Setup broadcast queue with size 1
    let router = setup_app(Some(1)).await;

    // Create and immediately drop the subscriber to ensure no receivers
    drop(subscribe(router.clone(), &[SystemId::Arkworks]).await);

    let arkworks_msg = json!({
        "proving_system_id": "arkworks",
        "message": "test1"
    })
    .to_string();

    // Submit a message when there are no subscribers
    let submit_response = submit(router.clone(), arkworks_msg).await;
    assert_eq!(submit_response.status(), StatusCode::ACCEPTED);

    let body = to_bytes(submit_response.into_body(), MAX_BODY_SIZE)
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
async fn test_single_subscriber_multiple_systems() {
    let router = setup_app(Some(3)).await;

    // We now pass multiple IDs, each generating system_ids=arkworks&system_ids=risc0
    let mut stream = subscribe(
        router.clone(),
        &[
            SystemId::Arkworks,
            SystemId::Risc0,
            SystemId::Gnark,
            SystemId::Sp1,
            SystemId::AlignedLayer,
        ],
    )
    .await;

    // Give time for subscription setup
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    // Submit simple messages
    let arkworks_msg = json!({
        "proving_system_id": "arkworks",
        "message": "test1"
    })
    .to_string();

    let risc0_msg = json!({
        "proving_system_id": "risc0",
        "message": "test2"
    })
    .to_string();

    let submit1 = submit(router.clone(), arkworks_msg.to_string()).await;
    assert_eq!(submit1.status(), StatusCode::OK);

    let submit2 = submit(router.clone(), risc0_msg.to_string()).await;
    assert_eq!(submit2.status(), StatusCode::OK);

    // Verify we receive both messages on the single stream
    for _ in 0..2 {
        if let Some(Ok(event_data)) = stream.next().await {
            let data = String::from_utf8(event_data.into()).unwrap();
            assert!(
                data.contains("test1") || data.contains("test2"),
                "Received unexpected message: {}",
                data
            );
        } else {
            panic!("Missing expected message");
        }
    }
}

#[tokio::test]
async fn test_multiple_subscribers_single_system() {
    let router = setup_app(Some(3)).await;

    // Create three separate subscribers all listening to Arkworks
    let mut streams = Vec::new();
    for _ in 0..3 {
        let stream = subscribe(router.clone(), &[SystemId::Arkworks]).await;
        streams.push(stream);
    }

    // Submit simple messages
    let arkworks_msg = json!({
        "proving_system_id": "arkworks",
        "message": "test1"
    })
    .to_string();

    let submit_response = submit(router.clone(), arkworks_msg.to_string()).await;
    assert_eq!(submit_response.status(), StatusCode::OK);

    // Verify each subscriber receives the message
    for (i, mut stream) in streams.into_iter().enumerate() {
        if let Some(Ok(event_data)) = stream.next().await {
            println!("Subscriber {} received message", i);
            assert_eq!(
                String::from_utf8(event_data.into()).unwrap(),
                format!("data: {}\n\n", arkworks_msg)
            );
        } else {
            panic!("Subscriber {} didn't receive message", i);
        }
    }
}

#[tokio::test]
async fn test_multiple_subscribers_multiple_systems() {
    let router = setup_app(Some(10)).await;

    // Create subscribers for different combinations of systems
    let stream1 = subscribe(
        router.clone(),
        &[
            SystemId::Arkworks,
            SystemId::Risc0,
            SystemId::Gnark,
            SystemId::Sp1,
            SystemId::AlignedLayer,
        ],
    )
    .await;

    let stream2 = subscribe(router.clone(), &[SystemId::Arkworks, SystemId::Risc0]).await;

    let stream3 = subscribe(
        router.clone(),
        &[SystemId::Arkworks, SystemId::AlignedLayer],
    )
    .await;

    let stream4 = subscribe(router.clone(), &[SystemId::Sp1]).await;

    // Create test messages for different systems
    let arkworks_msg = json!({
        "proving_system_id": "arkworks",
        "data": "test_arkworks_data"
    })
    .to_string();

    let risc0_msg = json!({
        "proving_system_id": "risc0",
        "data": "test_risc0_data"
    })
    .to_string();

    let gnark_msg = json!({
        "proving_system_id": "gnark",
        "data": "test_gnark_data"
    })
    .to_string();

    let sp1_msg = json!({
        "proving_system_id": "sp1",
        "data": "test_sp1_data"
    })
    .to_string();

    let aligned_msg = json!({
        "proving_system_id": "aligned-layer",
        "data": "test_aligned_data"
    })
    .to_string();

    // Submit all messages
    submit(router.clone(), arkworks_msg.clone()).await;
    submit(router.clone(), risc0_msg.clone()).await;
    submit(router.clone(), gnark_msg.clone()).await;
    submit(router.clone(), sp1_msg.clone()).await;
    submit(router.clone(), aligned_msg.clone()).await;

    // Collect messages from all streams for a short duration
    let timeout = Duration::from_millis(100);

    let messages1 = collect_messages(stream1, timeout).await;
    let messages2 = collect_messages(stream2, timeout).await;
    let messages3 = collect_messages(stream3, timeout).await;
    let messages4 = collect_messages(stream4, timeout).await;

    // Verify stream1 (subscribed to all) received all messages
    assert_eq!(messages1.len(), 5);
    assert!(messages1
        .iter()
        .any(|msg| msg.contains("test_arkworks_data")));
    assert!(messages1.iter().any(|msg| msg.contains("test_risc0_data")));
    assert!(messages1.iter().any(|msg| msg.contains("test_gnark_data")));
    assert!(messages1.iter().any(|msg| msg.contains("test_sp1_data")));
    assert!(messages1
        .iter()
        .any(|msg| msg.contains("test_aligned_data")));

    // Verify stream2 (Arkworks, Risc0) received only its messages
    assert_eq!(messages2.len(), 2);
    assert!(messages2
        .iter()
        .any(|msg| msg.contains("test_arkworks_data")));
    assert!(messages2.iter().any(|msg| msg.contains("test_risc0_data")));

    // Verify stream3 (Arkworks, AlignedLayer) received only its messages
    assert_eq!(messages3.len(), 2);
    assert!(messages3
        .iter()
        .any(|msg| msg.contains("test_arkworks_data")));
    assert!(messages3
        .iter()
        .any(|msg| msg.contains("test_aligned_data")));

    // Verify stream4 (Sp1 only) received only its message
    assert_eq!(messages4.len(), 1);
    assert!(messages4.iter().any(|msg| msg.contains("test_sp1_data")));
}

// Helper function to collect messages from a stream for a given duration
async fn collect_messages(
    mut stream: impl Stream<Item = Result<String, axum::Error>> + Unpin,
    timeout: Duration,
) -> Vec<String> {
    let mut messages = Vec::new();

    let collection_task = async {
        while let Some(Ok(msg)) = stream.next().await {
            messages.push(msg);
        }
    };

    tokio::select! {
        _ = collection_task => {},
        _ = tokio::time::sleep(timeout) => {},
    }

    messages
}
