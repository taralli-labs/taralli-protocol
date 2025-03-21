use taralli_server::subscription_manager::SubscriptionManager;

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
