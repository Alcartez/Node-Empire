//! Engine error plumbing.
//!
//! Systems are allowed to fail here: they return `Result` and Bevy hands the
//! error to its fallback handler. Ours logs it and parks the message in a queue
//! that the HUD drains into the toast line, so a failed save, a broken lookup
//! or a bad action shows up as a line on screen instead of taking the process
//! down (Bevy's default handler panics on `Severity::Panic`).
//!
//! The queue is a plain `Mutex` and not an ECS resource on purpose: the handler
//! is a bare `fn`, so it has no world access, and it may run from any thread.

use bevy::ecs::error::{ErrorContext, FallbackErrorHandler};
use bevy::prelude::*;
use std::sync::Mutex;

/// Problems waiting to be shown. Bounded, because a failing system can fail
/// every frame and the player only ever reads one toast at a time.
static QUEUE: Mutex<Vec<String>> = Mutex::new(Vec::new());

/// Bevy's fallback: log it and put it on screen.
fn capture(err: BevyError, ctx: ErrorContext) {
    error!("{} `{}` failed: {err}", ctx.kind(), ctx.name());
    report(err.to_string());
}

/// Record a problem for the next HUD frame.
pub fn report(msg: impl Into<String>) {
    if let Ok(mut queue) = QUEUE.lock() {
        if queue.len() < 8 {
            queue.push(msg.into());
        }
    }
}

/// Take the oldest queued problem, if any.
pub fn take() -> Option<String> {
    let mut queue = QUEUE.lock().ok()?;
    if queue.is_empty() {
        None
    } else {
        Some(queue.remove(0))
    }
}

/// The one error a game-layer call can produce that the player should read.
pub fn engine_error(msg: String) -> BevyError {
    BevyError::new(Severity::Error, msg)
}

/// Install the handler.
pub struct ErrorPlugin;

impl Plugin for ErrorPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(FallbackErrorHandler(capture));
    }
}
