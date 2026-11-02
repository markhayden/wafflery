use super::*;
use std::time::Duration;
use tokio::process::Command;

#[test]
fn observations_require_the_pinned_leader_and_only_zombies_are_drained() {
    for text in [
        "",
        "bad",
        "12 12 Z\n12 12 Z",
        "12 99 Z",
        "13 12 Z",
        "12 12 invalid",
        "12 12 Z extra",
    ] {
        assert!(members(text.as_bytes(), 12).is_err(), "{text}");
    }
    let snapshot = members(b" 12 12 Z\n13 12 T\n", 12).unwrap();
    assert!(snapshot.leader_exited);
    assert!(!snapshot.drained, "a stopped descendant is still live");
    assert!(members(b"12 12 Z\n13 12 Z\n", 12).unwrap().drained);
    assert!(!members(b"12 12 S\n", 12).unwrap().leader_exited);
}

#[test]
fn transitional_mach_states_are_valid_observations_but_never_drained() {
    for state in ["?", "?E", "H", "HE"] {
        let snapshot = members(format!("12 12 {state}\n").as_bytes(), 12).unwrap();
        assert!(!snapshot.leader_exited);
        assert!(!snapshot.drained);
        let snapshot = members(format!("12 12 Z\n13 12 {state}\n").as_bytes(), 12).unwrap();
        assert!(snapshot.leader_exited);
        assert!(
            !snapshot.drained,
            "uncertain descendants keep the group owned"
        );
    }
}

#[tokio::test]
async fn an_uncertain_snapshot_does_not_erase_a_confirmed_leader_exit_event() {
    let mut group = OwnedGroup::spawn(&mut Command::new("/usr/bin/true")).unwrap();
    // A received NOTE_EXIT is durable even when ps's separate observation is uncertain.
    group.leader_exited = true;
    let snapshot = members(
        format!("{} {} ?E\n", group.pid, group.pid).as_bytes(),
        group.pid,
    )
    .unwrap();
    assert!(matches!(
        group.finish_observation(snapshot).await.unwrap(),
        GroupState::Draining
    ));
    assert!(group.child.id().is_some());
    assert!(settled(&mut group).await.success());
}

#[tokio::test]
async fn owned_group_signals_cannot_target_a_reaped_identity() {
    let mut command = Command::new("/bin/sleep");
    command.arg("60");
    let mut group = OwnedGroup::spawn(&mut command).unwrap();
    assert!(matches!(group.check().await.unwrap(), GroupState::Running));
    group.terminate().unwrap();
    let status = settled(&mut group).await;
    use std::os::unix::process::ExitStatusExt;
    assert_eq!(status.signal(), Some(15));
    assert!(group.terminate().is_err());
    assert!(group.force().is_err());
    assert!(matches!(
        group.check().await.unwrap(),
        GroupState::Exited(_)
    ));
}

#[tokio::test]
async fn an_exit_before_event_registration_keeps_the_leader_owned_until_checked() {
    let mut command = Command::new("/usr/bin/true");
    let mut group = OwnedGroup::spawn_unwatched(&mut command).unwrap();
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if group.observe_members().await.unwrap().leader_exited {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    group.register();
    assert!(settled(&mut group).await.success());
}

#[tokio::test]
async fn observer_failure_overflow_and_timeout_are_errors_not_empty_groups() {
    let fail = Command::new("/usr/bin/false");
    assert!(capture(fail, 32, Duration::from_secs(1)).await.is_err());
    let flood = Command::new("/usr/bin/yes");
    assert!(capture(flood, 32, Duration::from_secs(1)).await.is_err());
    let mut slow = Command::new("/bin/sleep");
    slow.arg("60");
    assert!(capture(slow, 32, Duration::from_millis(20)).await.is_err());
    let mut group = OwnedGroup::spawn(&mut Command::new("/usr/bin/true")).unwrap();
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if group.observe_members().await.unwrap().leader_exited {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    group.observer = "/usr/bin/false".into();
    assert!(group.check().await.is_err());
    assert!(
        group.child.id().is_some(),
        "failed observation must not reap the leader"
    );
    group.observer = "/bin/ps".into();
    assert!(settled(&mut group).await.success());
}

async fn settled(group: &mut OwnedGroup) -> std::process::ExitStatus {
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if let GroupState::Exited(status) = group.check().await.unwrap() {
                break status;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap()
}
