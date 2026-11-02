use super::*;

#[test]
fn grants_are_distinct_scoped_and_only_one_consumer_can_win() {
    let grants = FrameGrants::default();
    let first = grants.issue("view-one").unwrap();
    let second = grants.issue("view-two").unwrap();
    assert_ne!(first, second);
    assert_eq!(first.len(), 64);
    assert!(first.bytes().all(|c| c.is_ascii_hexdigit()));
    assert_eq!(grants.consume("unknown"), None);
    let threads = (0..8)
        .map(|_| {
            let grants = grants.clone();
            let token = first.clone();
            std::thread::spawn(move || grants.consume(&token))
        })
        .collect::<Vec<_>>();
    let winners = threads
        .into_iter()
        .filter_map(|t| t.join().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(winners, vec!["view-one"]);
    assert_eq!(grants.consume(&second).as_deref(), Some("view-two"));
    assert_eq!(grants.consume(&first), None);
}
#[tokio::test(start_paused = true)]
async fn expiry_is_exact_and_cannot_be_extended_by_use() {
    let grants = FrameGrants::default();
    let first = grants.issue("view-one").unwrap();
    tokio::time::advance(std::time::Duration::from_secs(59)).await;
    let second = grants.issue("view-two").unwrap();
    assert_eq!(grants.consume(&first).as_deref(), Some("view-one"));
    tokio::time::advance(std::time::Duration::from_secs(60)).await;
    assert_eq!(grants.consume(&second), None);
    assert_eq!(grants.consume(&first), None);
}
#[tokio::test(start_paused = true)]
async fn capacity_is_bounded_and_consumption_or_expiry_releases_it() {
    let grants = FrameGrants::default();
    let tokens = (0..64)
        .map(|_| grants.issue("same-view").unwrap())
        .collect::<Vec<_>>();
    assert!(grants.issue("excess").is_err());
    assert_eq!(grants.consume(&tokens[0]).as_deref(), Some("same-view"));
    assert!(grants.issue("replacement").is_ok());
    assert!(grants.issue("excess").is_err());
    tokio::time::advance(std::time::Duration::from_secs(60)).await;
    assert!(grants.issue("after-expiry").is_ok());
    for token in tokens {
        assert_eq!(grants.consume(&token), None);
    }
    let private = grants.issue("host-private").unwrap();
    assert_eq!(FrameGrants::default().consume(&private), None);
}
