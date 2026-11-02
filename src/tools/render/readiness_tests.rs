use super::*;
use crate::tools::render::process::{Output, ProcessPhase, ProcessState};
fn output(bytes: &[u8]) -> ProcessState {
    ProcessState {
        phase: ProcessPhase::Running,
        close_requested: false,
        exit_code: None,
        stdout: Output {
            bytes: bytes.to_vec(),
            ..Output::default()
        },
        stderr: Output::default(),
        error: None,
    }
}
fn monitor() -> ServerReadiness {
    ServerReadiness::new("http://127.0.0.1:4801", Duration::from_secs(2)).unwrap()
}
#[test]
fn url_requires_one_complete_line_and_deadline_stops_startup() {
    let mut m = monitor();
    let now = m.started;
    assert_eq!(
        m.observe_at(&output(b"http://127.0.0.1:4802/report"), now),
        Readiness::Waiting
    );
    assert!(matches!(
        m.observe_at(&output(b"http://127.0.0.1:4802/report\n"), now),
        Readiness::Ready(_)
    ));
    assert!(
        matches!(
            m.observe_at(
                &output(b"http://127.0.0.1:4802/report\n"),
                now + Duration::from_secs(3)
            ),
            Readiness::Ready(_)
        ),
        "startup deadline does not expire a listening view"
    );
    let mut m = monitor();
    let now = m.started;
    assert!(matches!(
        m.observe_at(&output(b""), now + Duration::from_secs(2)),
        Readiness::Failed(_)
    ));
    assert!(
        matches!(
            m.observe_at(
                &output(b"http://127.0.0.1:4802/\n"),
                now + Duration::from_secs(3)
            ),
            Readiness::Failed(_)
        ),
        "late output cannot recover a failed view"
    );
}
#[test]
fn only_literal_loopback_http_on_a_separate_origin_is_allowed() {
    for url in [
        "https://127.0.0.1:4802/",
        "http://localhost:4802/",
        "http://127.1:4802/",
        "http://2130706433:4802/",
        "http://[::1]:4802/",
        "http://127.0.0.1/",
        "http://127.0.0.1:0/",
        "http://127.0.0.1:65536/",
        "http://user@127.0.0.1:4802/",
        "http://127.0.0.1:4802@other.invalid/",
        "http://127.0.0.1:4801/report",
        " http://127.0.0.1:4802/",
        "http://127.0.0.1:4802/ ",
        "http://127.0.0.1:4802\\other",
        "http://127.0.0.1:+4802/",
    ] {
        let mut m = monitor();
        assert!(
            matches!(
                m.observe(&output(format!("{url}\n").as_bytes())),
                Readiness::Failed(_)
            ),
            "{url}"
        );
    }
    for url in [
        "http://127.0.0.1:4802/report?view=summary#finding",
        "http://127.0.0.1:80/",
    ] {
        let mut m = monitor();
        assert!(
            matches!(
                m.observe(&output(format!("{url}\r\n").as_bytes())),
                Readiness::Ready(_)
            ),
            "{url}"
        );
    }
}
#[test]
fn extra_stdout_invalidates_an_already_ready_view_and_never_recovers() {
    let mut m = monitor();
    let line = b"http://127.0.0.1:4802/\n";
    assert!(matches!(m.observe(&output(line)), Readiness::Ready(_)));
    assert!(matches!(
        m.observe(&output(&[line.as_slice(), b"extra"].concat())),
        Readiness::Failed(_)
    ));
    assert!(matches!(m.observe(&output(line)), Readiness::Failed(_)));
    for bytes in [
        b"http://127.0.0.1:4802/\n\n".as_slice(),
        b"\xff\n",
        b"http://127.0.0.1:4802/\0\n",
        b"not a URL\n",
    ] {
        assert!(matches!(
            monitor().observe(&output(bytes)),
            Readiness::Failed(_)
        ));
    }
}
#[test]
fn exited_stopping_overflow_and_broken_output_are_not_readiness() {
    let line = b"http://127.0.0.1:4802/\n";
    for phase in [
        ProcessPhase::Exited,
        ProcessPhase::Stopping,
        ProcessPhase::NeedsForce,
    ] {
        let mut state = output(line);
        state.phase = phase;
        state.exit_code = Some(0);
        assert!(matches!(monitor().observe(&state), Readiness::Failed(_)));
    }
    let mut state = output(line);
    state.stdout.overflow = true;
    assert!(matches!(monitor().observe(&state), Readiness::Failed(_)));
    state = output(line);
    state.stderr.overflow = true;
    assert!(matches!(monitor().observe(&state), Readiness::Failed(_)));
    state = output(line);
    state.stdout.error = Some("synthetic".into());
    assert!(matches!(monitor().observe(&state), Readiness::Failed(_)));
    state = output(b"partial");
    state.stdout.finished = true;
    assert!(matches!(monitor().observe(&state), Readiness::Failed(_)));
}
#[test]
fn readiness_configuration_rejects_unknown_host_origins_and_out_of_contract_deadlines() {
    for timeout in [
        Duration::ZERO,
        Duration::from_millis(500),
        Duration::from_secs(61),
    ] {
        assert!(ServerReadiness::new("http://127.0.0.1:4801", timeout).is_err());
    }
    assert!(ServerReadiness::new("https://other.invalid", Duration::from_secs(1)).is_err());
    assert!(ServerReadiness::new("http://127.0.0.1:4801", Duration::from_secs(60)).is_ok());
}
