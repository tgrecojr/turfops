use super::*;
use std::cell::Cell;

// @spec DEPLOY-START-007
#[test]
fn startup_waits_about_a_minute_with_growing_gaps() {
    let delays = startup_delays();
    assert!(delays.len() >= 4, "{delays:?}");
    assert!(
        delays.windows(2).all(|w| w[0] <= w[1]),
        "never shrinking: {delays:?}"
    );
    let total: Duration = delays.iter().sum();
    assert!(
        (50..=75).contains(&total.as_secs()),
        "about a minute: {total:?}"
    );
}

// @spec DEPLOY-START-007
#[tokio::test]
async fn a_database_that_comes_up_late_is_reached() {
    let tries = Cell::new(0);
    let delays = [
        Duration::from_millis(1),
        Duration::from_millis(2),
        Duration::from_millis(4),
    ];
    let result: std::result::Result<&str, String> = with_retry(
        &delays,
        |_: &String| true,
        || {
            tries.set(tries.get() + 1);
            let n = tries.get();
            async move {
                if n < 3 {
                    Err(format!("refused #{n}"))
                } else {
                    Ok("connected")
                }
            }
        },
    )
    .await;
    assert_eq!(result, Ok("connected"));
    assert_eq!(tries.get(), 3);

    tries.set(0);
    let gave_up: std::result::Result<(), String> = with_retry(
        &delays,
        |_: &String| true,
        || async { Err("refused".to_string()) },
    )
    .await;
    assert!(gave_up.is_err());
}

// @spec DEPLOY-START-007
#[tokio::test]
async fn a_wrong_password_is_not_waited_out() {
    let tries = Cell::new(0);
    let delays = [Duration::from_millis(1); 3];
    let result: std::result::Result<(), String> = with_retry(
        &delays,
        |e: &String| e.contains("refused"),
        || {
            tries.set(tries.get() + 1);
            async { Err("password authentication failed".to_string()) }
        },
    )
    .await;
    assert!(result.is_err());
    assert_eq!(
        tries.get(),
        1,
        "no retries for a failure that won't fix itself"
    );
    assert!(unreachable(&sqlx::Error::PoolTimedOut));
    assert!(!unreachable(&sqlx::Error::RowNotFound));
}
