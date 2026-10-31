use wafflehouse::events::Events;
#[tokio::test]
async fn slow_subscribers_reset_and_new_sessions_cannot_reuse_event_sequences() {
    let events = Events::new().unwrap();
    let mut subscriber = events.subscribe();
    assert_eq!(subscriber.next().await.unwrap().kind, "reset");
    for _ in 0..100 {
        events.changed();
    }
    let event = subscriber.next().await.unwrap();
    assert_eq!(event.kind, "reset");
    assert_eq!(event.sequence, 100);
    let other = Events::new().unwrap();
    assert_ne!(other.session(), events.session());
    events.close();
    while subscriber.next().await.is_some() {}
}
