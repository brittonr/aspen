mod support;

use support::mock_gossip::MockGossip;
use iroh_gossip::proto::TopicId;

fn create_topic_id(seed: u8) -> TopicId {
    TopicId::from_bytes([seed; 32])
}

#[tokio::test]
async fn test_mock_gossip_basic() {
    let gossip = MockGossip::new();
    let topic = create_topic_id(1);

    let handle1 = gossip.subscribe(topic).unwrap();
    let handle2 = gossip.subscribe(topic).unwrap();

    // Broadcast from handle1
    handle1.broadcast(b"hello".to_vec()).await.unwrap();

    // Receive on handle2
    let msg = handle2.receive().await.unwrap().unwrap();
    assert_eq!(msg, b"hello"[..]);

    // handle1 also receives its own broadcast
    let msg = handle1.receive().await.unwrap().unwrap();
    assert_eq!(msg, b"hello"[..]);
}

#[tokio::test]
async fn test_mock_gossip_multiple_topics() {
    let gossip = MockGossip::new();
    let topic1 = create_topic_id(1);
    let topic2 = create_topic_id(2);

    let handle1a = gossip.subscribe(topic1).unwrap();
    let handle1b = gossip.subscribe(topic1).unwrap();
    let handle2a = gossip.subscribe(topic2).unwrap();

    // Broadcast on topic1
    handle1a.broadcast(b"topic1".to_vec()).await.unwrap();

    // Only topic1 subscribers receive
    let msg = handle1b.receive().await.unwrap().unwrap();
    assert_eq!(msg, b"topic1"[..]);

    // topic2 subscriber doesn't receive
    let msg = handle2a.try_receive().await.unwrap();
    assert!(msg.is_none());

    assert_eq!(gossip.topic_count(), 2);
}

#[tokio::test]
async fn test_mock_gossip_receiver_count() {
    let gossip = MockGossip::new();
    let topic = create_topic_id(1);

    let handle1 = gossip.subscribe(topic).unwrap();
    assert_eq!(handle1.receiver_count(), 1);

    let handle2 = gossip.subscribe(topic).unwrap();
    assert_eq!(handle1.receiver_count(), 2);
    assert_eq!(handle2.receiver_count(), 2);

    drop(handle2);
    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
    assert_eq!(handle1.receiver_count(), 1);
}

#[tokio::test]
async fn test_mock_gossip_try_receive() {
    let gossip = MockGossip::new();
    let topic = create_topic_id(1);

    let handle = gossip.subscribe(topic).unwrap();

    // No message available
    let msg = handle.try_receive().await.unwrap();
    assert!(msg.is_none());

    // Broadcast a message
    handle.broadcast(b"test".to_vec()).await.unwrap();

    // Message immediately available
    let msg = handle.try_receive().await.unwrap().unwrap();
    assert_eq!(msg, b"test"[..]);

    // No more messages
    let msg = handle.try_receive().await.unwrap();
    assert!(msg.is_none());
}
