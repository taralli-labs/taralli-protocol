use axum::body::to_bytes;
use hyper::StatusCode;
use serde_json::{json, Value};
use taralli_primitives::systems::ProvingSystemId;
use tokio_stream::StreamExt;
use common::helpers::{setup_app, submit, subscribe, MAX_BODY_SIZE};

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
    // println!("Router: {:?}", router);
    let stream = subscribe(router, &[ProvingSystemId::Arkworks, ProvingSystemId::Gnark]).await;
}

#[tokio::test]
async fn test_broadcast_over_with_no_more_subscribers() {
    // Setup broadcast queue with size 1
    let router = setup_app(Some(1)).await;

    // Create and immediately drop the subscriber to ensure no receivers
    drop(subscribe(router.clone(), &[ProvingSystemId::Arkworks]).await);

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
    /*let stream = subscribe(router.clone(), &[
        ProvingSystemId::Arkworks, 
        ProvingSystemId::Risc0
    ]).await;
    
    // spawn a task to read from `stream` (SSE) if needed:
    let stream_task = tokio::spawn(async move {
        let mut stream = Box::pin(stream);
        while let Some(result) = stream.next().await {
            println!("Received: {:?}", result);
        }
    });

    // Give time for subscription setup
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;*/

}

/*#[tokio::test]
async fn test_single_subscriber_multiple_systems() {
    let app = setup_app(Some(3)).await;

    // Create a single subscriber listening to multiple systems
    let response = app
        .clone()
        .oneshot(subscribe_request_body(&[
            ProvingSystemId::Arkworks,
            ProvingSystemId::Risc0,
        ]))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let mut stream = response.into_body().into_data_stream();

    // Submit simple messages
    let arkworks_msg = json!({
        "proving_system_id": "arkworks",
        "message": "test1"
    }).to_string();
    
    let risc0_msg = json!({
        "proving_system_id": "risc0",
        "message": "test2"
    }).to_string();

    let submit1 = submit(app.clone(), Some(arkworks_msg.to_string())).await;
    assert_eq!(submit1.status(), StatusCode::OK);

    let submit2 = submit(app.clone(), Some(risc0_msg.to_string())).await;
    assert_eq!(submit2.status(), StatusCode::OK);

    // Verify we receive both messages on the single stream
    for _ in 0..2 {
        if let Some(Ok(event_data)) = stream.next().await {
            let data = String::from_utf8(event_data.to_vec()).unwrap();
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
    let app = setup_app(Some(3)).await;

    // Create three separate subscribers all listening to Arkworks
    let mut streams = Vec::new();
    for _ in 0..3 {
        let response = app
            .clone()
            .oneshot(subscribe_request_body(&[ProvingSystemId::Arkworks]))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        streams.push(response.into_body().into_data_stream());
    }

    // Submit a single message
    let msg = r#"{"proving_system_id": "arkworks", "message": "test"}"#;
    let submit_response = submit(app.clone(), Some(msg.to_string())).await;
    assert_eq!(submit_response.status(), StatusCode::OK);

    // Verify each subscriber receives the message
    for (i, mut stream) in streams.into_iter().enumerate() {
        if let Some(Ok(event_data)) = stream.next().await {
            println!("Subscriber {} received message", i);
            assert_eq!(
                String::from_utf8(event_data.to_vec()).unwrap(),
                format!("data: {}\n\n", msg)
            );
        } else {
            panic!("Subscriber {} didn't receive message", i);
        }
    }
}

#[tokio::test]
async fn test_multiple_subscribers_multiple_systems() {
    let app = setup_app(Some(3)).await;

    // Create subscribers with different system combinations
    let mut streams = Vec::new();

    // Subscriber 1: Arkworks only
    let response1 = app
        .clone()
        .oneshot(subscribe_request_body(&[ProvingSystemId::Arkworks]))
        .await
        .unwrap();
    streams.push((
        vec![ProvingSystemId::Arkworks],
        response1.into_body().into_data_stream(),
    ));

    // Subscriber 2: Risc0 only
    let response2 = app
        .clone()
        .oneshot(subscribe_request_body(&[ProvingSystemId::Risc0]))
        .await
        .unwrap();
    streams.push((
        vec![ProvingSystemId::Risc0],
        response2.into_body().into_data_stream(),
    ));

    // Subscriber 3: Both systems
    let response3 = app
        .clone()
        .oneshot(subscribe_request_body(&[
            ProvingSystemId::Arkworks,
            ProvingSystemId::Risc0,
        ]))
        .await
        .unwrap();
    streams.push((
        vec![ProvingSystemId::Arkworks, ProvingSystemId::Risc0],
        response3.into_body().into_data_stream(),
    ));

    // Submit messages to both systems
    let arkworks_msg = r#"{"proving_system_id": "arkworks", "message": "test1"}"#;
    let risc0_msg = r#"{"proving_system_id": "risc0", "message": "test2"}"#;

    let submit1 = submit(app.clone(), Some(arkworks_msg.to_string())).await;
    assert_eq!(submit1.status(), StatusCode::OK);

    let submit2 = submit(app.clone(), Some(risc0_msg.to_string())).await;
    assert_eq!(submit2.status(), StatusCode::OK);

    // Verify each subscriber receives appropriate messages
    for (i, (systems, mut stream)) in streams.into_iter().enumerate() {
        let expected_msgs = systems.len();
        let mut received = 0;

        for _ in 0..expected_msgs {
            if let Some(Ok(event_data)) = stream.next().await {
                let data = String::from_utf8(event_data.to_vec()).unwrap();
                assert!(
                    (systems.contains(&ProvingSystemId::Arkworks) && data.contains("test1"))
                        || (systems.contains(&ProvingSystemId::Risc0) && data.contains("test2")),
                    "Subscriber {} received unexpected message: {}",
                    i,
                    data
                );
                received += 1;
            }
        }
        assert_eq!(
            received, expected_msgs,
            "Subscriber {} didn't receive all expected messages",
            i
        );
    }
}*/
