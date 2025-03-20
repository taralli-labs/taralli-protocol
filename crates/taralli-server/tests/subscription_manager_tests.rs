/*use taralli_primitives::systems::SystemId;
use taralli_server::subscription_manager::SubscriptionManager;

#[tokio::test]
/// Ensures a broadcast is sending data correctly.
async fn should_broadcast() {
    let subscription_manager: SubscriptionManager<i32> = SubscriptionManager::default();
    let mut recv = subscription_manager
        .subscribe_to_ids(&[SystemId::Arkworks])
        .await
        .remove(0);
    subscription_manager
        .broadcast(SystemId::Arkworks, 1)
        .await
        .unwrap();
    assert_eq!(Some(1), Some(recv.recv().await.unwrap()));
}

#[tokio::test]
async fn should_not_broadcast_without_receivers() {
    let subscription_manager: SubscriptionManager<i32> = SubscriptionManager::default();
    assert!(subscription_manager
        .broadcast(SystemId::Arkworks, 1)
        .await
        .is_err());
}

#[tokio::test]
async fn should_receive_lagged() {
    let subscription_manager: SubscriptionManager<i32> = SubscriptionManager::new(1);
    assert!(subscription_manager
        .broadcast(SystemId::Arkworks, 1)
        .await
        .is_err());
    let mut recv = subscription_manager
        .subscribe_to_ids(&[SystemId::Arkworks])
        .await
        .remove(0);
    subscription_manager
        .broadcast(SystemId::Arkworks, 2)
        .await
        .unwrap();
    assert_eq!(Some(2), Some(recv.recv().await.unwrap()));
}

#[tokio::test]
async fn should_broadcast_multiple_times() {
    let subscription_manager: SubscriptionManager<i32> = SubscriptionManager::new(10);
    let mut recv = subscription_manager
        .subscribe_to_ids(&[SystemId::Arkworks])
        .await
        .remove(0);
    for i in 0..10 {
        subscription_manager
            .broadcast(SystemId::Arkworks, i)
            .await
            .unwrap();
    }
    for i in 0..10 {
        assert_eq!(Some(i), Some(recv.recv().await.unwrap()));
    }
}

#[tokio::test]
async fn should_broadcast_multiple_times_to_many_subscribers() {
    let subscription_manager: SubscriptionManager<i32> = SubscriptionManager::new(10);
    let mut r1 = subscription_manager
        .subscribe_to_ids(&[SystemId::Arkworks])
        .await
        .remove(0);
    let mut r2 = subscription_manager
        .subscribe_to_ids(&[SystemId::Arkworks])
        .await
        .remove(0);
    for i in 0..10 {
        subscription_manager
            .broadcast(SystemId::Arkworks, i)
            .await
            .unwrap();
        assert_eq!(Some(i), Some(r1.recv().await.unwrap()));
        assert_eq!(Some(i), Some(r2.recv().await.unwrap()));
    }
}

#[tokio::test]
/// Ensures that upon the removal of the receiver (imagine if client connection drops), we no longer keep it alive.
async fn should_drop_receivers() {
    let subscription_manager: SubscriptionManager<i32> = SubscriptionManager::default();
    let recv = subscription_manager
        .subscribe_to_ids(&[SystemId::Arkworks])
        .await
        .remove(0);
    drop(recv);
    // `broadcast()` to no recvs returns errored.
    let _ = subscription_manager
        .broadcast(SystemId::Arkworks, 1)
        .await
        .unwrap_err();
    // Check receiver count directly from the channel
    let count = subscription_manager
        .get_or_create_sender(SystemId::Arkworks)
        .await
        .receiver_count();
    assert_eq!(count, 0);
}

#[tokio::test]
/// Ensures the removal of one subscriber is not affecting the next one.
async fn should_continue_broadcast_after_subscriber_removed() {
    let subscription_manager: SubscriptionManager<i32> = SubscriptionManager::default();
    let r1 = subscription_manager
        .subscribe_to_ids(&[SystemId::Arkworks])
        .await
        .remove(0);
    let mut r2 = subscription_manager
        .subscribe_to_ids(&[SystemId::Arkworks])
        .await
        .remove(0);
    // Simulate one client disconnection and broadcast
    drop(r1);
    subscription_manager
        .broadcast(SystemId::Arkworks, 1)
        .await
        .unwrap();
    // Verify that the remaining subscriber receives the message
    assert_eq!(Some(1), Some(r2.recv().await.unwrap()));
    // Check receiver count directly from the channel
    let count = subscription_manager
        .get_or_create_sender(SystemId::Arkworks)
        .await
        .receiver_count();
    assert_eq!(count, 1);
}
*/
