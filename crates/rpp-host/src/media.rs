//! The system media session (what Windows shows in its volume flyout).
#![allow(unsafe_code)]

use std::thread;
use std::time::{Duration, Instant};

use serde_json::{Value, json};
use windows::Media::Control::{
    GlobalSystemMediaTransportControlsSessionManager as Manager,
    GlobalSystemMediaTransportControlsSessionPlaybackStatus as Status,
};
use windows::Win32::System::Com::{COINIT_MULTITHREADED, CoInitializeEx};
use windows_future::{AsyncStatus, IAsyncOperation};

/// One tick of a `TimeSpan` is 100 nanoseconds.
const TICKS_PER_SECOND: f64 = 10_000_000.0;
/// Longest wait for one answer of the media service. It usually answers in a
/// few milliseconds; past this it is stuck and the script gets an error.
const ANSWER_WAIT: Duration = Duration::from_secs(2);
/// Pause between two looks at an operation that has not finished.
const POLL: Duration = Duration::from_millis(5);

/// Where an asynchronous operation stands.
enum Step<T> {
    Pending,
    Done(Result<T, String>),
}

/// Waits for an operation until `limit`, cancelling it if it does not finish.
///
/// Raises:
///     String: The operation's own error, or that it did not answer in time.
fn wait_for<T>(
    limit: Duration,
    mut step: impl FnMut() -> Step<T>,
    cancel: impl FnOnce(),
) -> Result<T, String> {
    let deadline = Instant::now() + limit;
    loop {
        if let Step::Done(result) = step() {
            return result;
        }
        if Instant::now() >= deadline {
            cancel();
            return Err("the system media session did not answer in time".into());
        }
        thread::sleep(POLL);
    }
}

/// Waits for a WinRT operation for at most [`ANSWER_WAIT`].
fn answer<T: windows::core::RuntimeType + 'static>(
    operation: IAsyncOperation<T>,
) -> Result<T, String> {
    wait_for(
        ANSWER_WAIT,
        || match operation.Status() {
            Ok(AsyncStatus::Started) => Step::Pending,
            Ok(_) => Step::Done(operation.GetResults().map_err(|e| e.to_string())),
            Err(error) => Step::Done(Err(error.to_string())),
        },
        || {
            let _ = operation.Cancel();
        },
    )
}

fn seconds(ticks: i64) -> f64 {
    (ticks as f64 / TICKS_PER_SECOND * 1000.0).round() / 1000.0
}

fn status_name(status: Status) -> &'static str {
    match status {
        Status::Playing => "playing",
        Status::Paused => "paused",
        Status::Stopped => "stopped",
        Status::Changing => "changing",
        Status::Opened => "opened",
        Status::Closed => "closed",
        _ => "unknown",
    }
}

thread_local! {
    static COM_READY: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Initializes COM once per thread.
fn initialize_com() {
    COM_READY.with(|ready| {
        if !ready.get() {
            // SAFETY: initializing COM for this thread has no memory-safety
            // preconditions. A thread already initialized in another mode
            // returns an error that is safe to ignore.
            let _ = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
            ready.set(true);
        }
    });
}

/// Reads the current system media session, the one Windows shows in its
/// volume flyout.
///
/// Returns:
///     `{app, title, artist, album, status, position, duration}` with times in
///     seconds, or `null` when nothing is registered.
///
/// Raises:
///     String: The session manager cannot be reached or does not answer
///         within two seconds.
pub fn current() -> Result<Value, String> {
    initialize_com();
    let manager = answer(Manager::RequestAsync().map_err(|e| e.to_string())?)?;
    let Ok(session) = manager.GetCurrentSession() else {
        return Ok(Value::Null);
    };
    let text = |result: windows::core::Result<windows::core::HSTRING>| {
        result.map(|value| value.to_string()).unwrap_or_default()
    };
    let properties = answer(
        session
            .TryGetMediaPropertiesAsync()
            .map_err(|e| e.to_string())?,
    )?;
    let status = session
        .GetPlaybackInfo()
        .and_then(|info| info.PlaybackStatus())
        .map(status_name)
        .unwrap_or("unknown");
    let (position, duration) = session
        .GetTimelineProperties()
        .map(|timeline| {
            let start = timeline.StartTime().map_or(0, |t| t.Duration);
            let end = timeline.EndTime().map_or(0, |t| t.Duration);
            let position = timeline.Position().map_or(0, |t| t.Duration);
            // Another program sets these numbers, so they can be anything.
            (
                seconds(position.saturating_sub(start)),
                seconds(end.saturating_sub(start)),
            )
        })
        .unwrap_or((0.0, 0.0));
    Ok(json!({
        "app": text(session.SourceAppUserModelId()),
        "title": text(properties.Title()),
        "artist": text(properties.Artist()),
        "album": text(properties.AlbumTitle()),
        "status": status,
        "position": position,
        "duration": duration,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tick_conversion_rounds_to_milliseconds() {
        assert_eq!(seconds(10_000_000), 1.0);
        assert_eq!(seconds(15_004_000), 1.5);
        assert_eq!(seconds(0), 0.0);
    }

    #[test]
    fn a_finished_operation_is_returned_at_once() {
        let started = Instant::now();
        let result = wait_for(
            ANSWER_WAIT,
            || Step::Done(Ok(7)),
            || panic!("not cancelled"),
        );
        assert_eq!(result, Ok(7));
        assert!(started.elapsed() < Duration::from_millis(100));
        let failed: Result<u8, String> =
            wait_for(ANSWER_WAIT, || Step::Done(Err("boom".into())), || {});
        assert_eq!(failed, Err("boom".into()));
    }

    #[test]
    fn an_operation_that_finishes_late_but_in_time_is_waited_for() {
        let mut looks = 0;
        let result = wait_for(
            Duration::from_secs(1),
            || {
                looks += 1;
                if looks < 5 {
                    Step::Pending
                } else {
                    Step::Done(Ok("ready"))
                }
            },
            || panic!("not cancelled"),
        );
        assert_eq!(result, Ok("ready"));
    }

    #[test]
    fn a_stuck_operation_is_cancelled_at_the_limit() {
        let cancelled = std::cell::Cell::new(false);
        let started = Instant::now();
        let result: Result<(), String> = wait_for(
            Duration::from_millis(150),
            || Step::Pending,
            || cancelled.set(true),
        );
        let waited = started.elapsed();
        assert!(result.unwrap_err().contains("did not answer in time"));
        assert!(cancelled.get());
        assert!(
            waited >= Duration::from_millis(150) && waited < Duration::from_millis(600),
            "{waited:?}"
        );
    }

    #[test]
    fn querying_the_session_does_not_fail() {
        // Whether something is playing depends on the machine; the call must
        // still succeed and return an object or null.
        let answer = current().unwrap();
        assert!(answer.is_null() || answer["status"].is_string());
    }
}
