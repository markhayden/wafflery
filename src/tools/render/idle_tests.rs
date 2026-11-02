use super::*;

#[tokio::test(start_paused = true)]
async fn visible_activity_extends_five_minutes_but_late_activity_never_resurrects_expiry() {
    let lease = IdleLease::new(IDLE_TIMEOUT);
    assert_eq!(IDLE_TIMEOUT, Duration::from_secs(300));
    tokio::time::advance(Duration::from_secs(299)).await;
    assert!(lease.touch());
    tokio::time::advance(Duration::from_secs(299)).await;
    assert!(!lease.expire_if_due());
    tokio::time::advance(Duration::from_secs(1)).await;
    assert!(
        !lease.touch(),
        "late activity cannot race the expiry worker to renew"
    );
    assert!(lease.expired());
    lease.wait_expired().await;
    assert!(!lease.touch());
}

#[tokio::test(start_paused = true)]
async fn observation_does_not_extend_idle_time_and_a_waiting_timer_follows_activity() {
    let lease = IdleLease::new(IDLE_TIMEOUT);
    let worker = lease.clone();
    let task = tokio::spawn(async move {
        worker.wait_expired().await;
    });
    tokio::task::yield_now().await;
    tokio::time::advance(Duration::from_secs(250)).await;
    assert!(lease.touch());
    tokio::time::advance(Duration::from_secs(50)).await;
    assert!(!lease.expired());
    assert!(
        !task.is_finished(),
        "the old deadline cannot win after renewal"
    );
    tokio::time::advance(Duration::from_secs(250)).await;
    task.await.unwrap();
    assert!(lease.expired());
}

#[tokio::test]
async fn abandoned_view_closes_its_owned_process_without_browser_polling() {
    use crate::tools::render::process::tests::setup;
    let (_temp, plan) = setup("hold");
    let process = RenderProcess::spawn(&plan).unwrap();
    let mut state = process.observe();
    tokio::time::timeout(
        Duration::from_secs(5),
        state.wait_for(|s| !s.stdout.bytes.is_empty()),
    )
    .await
    .unwrap()
    .unwrap();
    let lease = IdleLease::new(Duration::from_millis(50));
    lease.supervise(process.clone());
    tokio::time::timeout(
        Duration::from_secs(5),
        state.wait_for(|s| s.phase == ProcessPhase::Exited),
    )
    .await
    .unwrap()
    .unwrap();
    assert!(lease.expired());
    assert_eq!(process.snapshot().exit_code, Some(0));
    assert!(!lease.touch());
}
