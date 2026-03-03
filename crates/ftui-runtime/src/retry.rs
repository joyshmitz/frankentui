// SPDX-License-Identifier: Apache-2.0
//! Retry policies and timeout-enforced task helpers.
//!
//! Provides [`RetryPolicy`] for configurable retry-with-backoff and
//! [`task_with_timeout`] / [`task_with_retry`] constructors that wrap
//! [`Cmd::Task`](crate::Cmd) with deterministic lifecycle guarantees.
//!
//! # Migration rationale
//!
//! Source frameworks often have retry/timeout baked into effect middleware.
//! These helpers give the migration code emitter explicit, testable primitives
//! to target instead of ad-hoc retry loops.
//!
//! # Determinism
//!
//! Backoff delays use fixed formulas (no jitter/randomness) so that
//! replay-based determinism tests can reproduce exact timing sequences.
//!
//! # Example
//!
//! ```
//! use ftui_runtime::retry::{RetryPolicy, BackoffStrategy};
//! use std::time::Duration;
//!
//! let policy = RetryPolicy::new(3, BackoffStrategy::Exponential {
//!     base_ms: 100,
//!     max_ms: 5000,
//! });
//!
//! assert_eq!(policy.delay(0), Duration::from_millis(100));
//! assert_eq!(policy.delay(1), Duration::from_millis(200));
//! assert_eq!(policy.delay(2), Duration::from_millis(400));
//! ```

#![forbid(unsafe_code)]

use crate::program::{Cmd, TaskSpec};
use web_time::Duration;

/// Backoff strategy for retry delays.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(
    feature = "state-persistence",
    derive(serde::Serialize, serde::Deserialize)
)]
pub enum BackoffStrategy {
    /// Fixed delay between retries.
    Fixed {
        /// Delay in milliseconds.
        delay_ms: u64,
    },
    /// Exponential backoff: `base_ms * 2^attempt`, capped at `max_ms`.
    Exponential {
        /// Base delay in milliseconds.
        base_ms: u64,
        /// Maximum delay cap in milliseconds.
        max_ms: u64,
    },
    /// Linear backoff: `base_ms * (attempt + 1)`, capped at `max_ms`.
    Linear {
        /// Base delay in milliseconds.
        base_ms: u64,
        /// Maximum delay cap in milliseconds.
        max_ms: u64,
    },
}

/// A retry policy with configurable attempts and backoff.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(
    feature = "state-persistence",
    derive(serde::Serialize, serde::Deserialize)
)]
pub struct RetryPolicy {
    /// Maximum number of retry attempts (0 = no retries, just the initial attempt).
    pub max_retries: u32,
    /// Backoff strategy between retries.
    pub backoff: BackoffStrategy,
}

impl RetryPolicy {
    /// Create a new retry policy.
    pub fn new(max_retries: u32, backoff: BackoffStrategy) -> Self {
        Self {
            max_retries,
            backoff,
        }
    }

    /// No retries — execute once.
    pub fn no_retry() -> Self {
        Self {
            max_retries: 0,
            backoff: BackoffStrategy::Fixed { delay_ms: 0 },
        }
    }

    /// Compute the delay before the given attempt (0-indexed).
    pub fn delay(&self, attempt: u32) -> Duration {
        match &self.backoff {
            BackoffStrategy::Fixed { delay_ms } => Duration::from_millis(*delay_ms),
            BackoffStrategy::Exponential { base_ms, max_ms } => {
                let multiplier = 1u64.checked_shl(attempt).unwrap_or(u64::MAX);
                let delay = base_ms.saturating_mul(multiplier);
                Duration::from_millis(delay.min(*max_ms))
            }
            BackoffStrategy::Linear { base_ms, max_ms } => {
                let delay = base_ms.saturating_mul(u64::from(attempt) + 1);
                Duration::from_millis(delay.min(*max_ms))
            }
        }
    }

    /// Total maximum delay across all retries (for timeout budgeting).
    pub fn total_max_delay(&self) -> Duration {
        let mut total = Duration::ZERO;
        for i in 0..self.max_retries {
            total += self.delay(i);
        }
        total
    }
}

/// Create a [`Cmd::Task`] that enforces a timeout.
///
/// If the closure does not complete within `timeout`, the task thread
/// continues running but its result is discarded. The `on_timeout` message
/// is sent instead.
///
/// Note: Rust does not support preemptive thread cancellation. The closure
/// continues executing but its result is ignored after the deadline.
pub fn task_with_timeout<M, F>(timeout: Duration, f: F, on_timeout: M) -> Cmd<M>
where
    M: Send + 'static,
    F: FnOnce() -> M + Send + 'static,
{
    Cmd::task(move || {
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let result = f();
            let _ = tx.send(result);
        });
        match rx.recv_timeout(timeout) {
            Ok(msg) => msg,
            Err(_) => on_timeout,
        }
    })
}

/// Create a [`Cmd::Task`] with a named spec and timeout.
pub fn task_with_timeout_named<M, F>(
    name: impl Into<String>,
    timeout: Duration,
    f: F,
    on_timeout: M,
) -> Cmd<M>
where
    M: Send + 'static,
    F: FnOnce() -> M + Send + 'static,
{
    Cmd::task_with_spec(TaskSpec::default().with_name(name), move || {
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let result = f();
            let _ = tx.send(result);
        });
        match rx.recv_timeout(timeout) {
            Ok(msg) => msg,
            Err(_) => on_timeout,
        }
    })
}

/// Create a [`Cmd::Task`] that retries on failure with the given policy.
///
/// The `f` closure returns `Result<M, String>`. On `Ok`, the message is
/// returned immediately. On `Err`, the task retries according to the policy,
/// sleeping between attempts. After all retries are exhausted, `on_exhaust`
/// is called with the last error to produce a fallback message.
pub fn task_with_retry<M, F>(policy: RetryPolicy, f: F, on_exhaust: fn(String) -> M) -> Cmd<M>
where
    M: Send + 'static,
    F: Fn() -> Result<M, String> + Send + 'static,
{
    Cmd::task(move || {
        let mut last_err = String::new();
        for attempt in 0..=policy.max_retries {
            match f() {
                Ok(msg) => return msg,
                Err(e) => {
                    last_err = e;
                    if attempt < policy.max_retries {
                        std::thread::sleep(policy.delay(attempt));
                    }
                }
            }
        }
        on_exhaust(last_err)
    })
}

/// Create a [`Cmd::Task`] with both retry and timeout.
///
/// Each individual attempt is bounded by `per_attempt_timeout`. The total
/// number of attempts is governed by the retry policy.
pub fn task_with_retry_and_timeout<M, F>(
    policy: RetryPolicy,
    per_attempt_timeout: Duration,
    f: F,
    on_exhaust: fn(String) -> M,
) -> Cmd<M>
where
    M: Send + 'static + Clone,
    F: Fn() -> Result<M, String> + Send + Sync + 'static,
{
    Cmd::task(move || {
        let f = std::sync::Arc::new(f);
        let mut last_err = String::new();
        for attempt in 0..=policy.max_retries {
            let (tx, rx) = std::sync::mpsc::channel();
            let f_clone = std::sync::Arc::clone(&f);
            std::thread::spawn(move || {
                let result = f_clone();
                let _ = tx.send(result);
            });
            match rx.recv_timeout(per_attempt_timeout) {
                Ok(Ok(msg)) => return msg,
                Ok(Err(e)) => {
                    last_err = e;
                }
                Err(_) => {
                    last_err = "timeout".into();
                }
            }
            if attempt < policy.max_retries {
                std::thread::sleep(policy.delay(attempt));
            }
        }
        on_exhaust(last_err)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixed_backoff_constant_delay() {
        let policy = RetryPolicy::new(3, BackoffStrategy::Fixed { delay_ms: 100 });
        assert_eq!(policy.delay(0), Duration::from_millis(100));
        assert_eq!(policy.delay(1), Duration::from_millis(100));
        assert_eq!(policy.delay(2), Duration::from_millis(100));
    }

    #[test]
    fn exponential_backoff_doubles() {
        let policy = RetryPolicy::new(
            5,
            BackoffStrategy::Exponential {
                base_ms: 100,
                max_ms: 5000,
            },
        );
        assert_eq!(policy.delay(0), Duration::from_millis(100));
        assert_eq!(policy.delay(1), Duration::from_millis(200));
        assert_eq!(policy.delay(2), Duration::from_millis(400));
        assert_eq!(policy.delay(3), Duration::from_millis(800));
    }

    #[test]
    fn exponential_backoff_caps_at_max() {
        let policy = RetryPolicy::new(
            5,
            BackoffStrategy::Exponential {
                base_ms: 1000,
                max_ms: 3000,
            },
        );
        assert_eq!(policy.delay(0), Duration::from_millis(1000));
        assert_eq!(policy.delay(1), Duration::from_millis(2000));
        assert_eq!(policy.delay(2), Duration::from_millis(3000)); // capped
        assert_eq!(policy.delay(3), Duration::from_millis(3000)); // capped
    }

    #[test]
    fn linear_backoff_increments() {
        let policy = RetryPolicy::new(
            4,
            BackoffStrategy::Linear {
                base_ms: 100,
                max_ms: 500,
            },
        );
        assert_eq!(policy.delay(0), Duration::from_millis(100));
        assert_eq!(policy.delay(1), Duration::from_millis(200));
        assert_eq!(policy.delay(2), Duration::from_millis(300));
        assert_eq!(policy.delay(3), Duration::from_millis(400));
        assert_eq!(policy.delay(4), Duration::from_millis(500)); // capped
    }

    #[test]
    fn linear_backoff_caps_at_max() {
        let policy = RetryPolicy::new(
            4,
            BackoffStrategy::Linear {
                base_ms: 200,
                max_ms: 500,
            },
        );
        assert_eq!(policy.delay(2), Duration::from_millis(500)); // 200*3 = 600, capped at 500
    }

    #[test]
    fn no_retry_policy() {
        let policy = RetryPolicy::no_retry();
        assert_eq!(policy.max_retries, 0);
    }

    #[test]
    fn total_max_delay_fixed() {
        let policy = RetryPolicy::new(3, BackoffStrategy::Fixed { delay_ms: 100 });
        assert_eq!(policy.total_max_delay(), Duration::from_millis(300));
    }

    #[test]
    fn total_max_delay_exponential() {
        let policy = RetryPolicy::new(
            3,
            BackoffStrategy::Exponential {
                base_ms: 100,
                max_ms: 10000,
            },
        );
        // Delays: 100 + 200 + 400 = 700
        assert_eq!(policy.total_max_delay(), Duration::from_millis(700));
    }

    #[test]
    fn total_max_delay_zero_retries() {
        let policy = RetryPolicy::no_retry();
        assert_eq!(policy.total_max_delay(), Duration::ZERO);
    }

    #[test]
    fn exponential_backoff_overflow_saturates() {
        let policy = RetryPolicy::new(
            1,
            BackoffStrategy::Exponential {
                base_ms: u64::MAX / 2,
                max_ms: u64::MAX,
            },
        );
        // Should not panic on overflow
        let _ = policy.delay(30);
    }

    #[test]
    fn linear_backoff_overflow_saturates() {
        let policy = RetryPolicy::new(
            1,
            BackoffStrategy::Linear {
                base_ms: u64::MAX / 2,
                max_ms: u64::MAX,
            },
        );
        let _ = policy.delay(30);
    }

    #[test]
    fn retry_policy_clone_eq() {
        let policy = RetryPolicy::new(
            3,
            BackoffStrategy::Exponential {
                base_ms: 100,
                max_ms: 5000,
            },
        );
        let cloned = policy.clone();
        assert_eq!(policy, cloned);
    }

    #[test]
    fn task_with_retry_succeeds_first_try() {
        #[derive(Debug, PartialEq)]
        enum Msg {
            Ok(i32),
            Err(String),
        }

        let policy = RetryPolicy::new(3, BackoffStrategy::Fixed { delay_ms: 1 });
        let cmd = task_with_retry(policy, || Ok(Msg::Ok(42)), Msg::Err);

        // Verify it produces a Task variant
        assert_eq!(cmd.type_name(), "Task");
    }

    #[test]
    fn task_with_timeout_produces_task() {
        #[derive(Debug)]
        #[allow(dead_code)]
        enum Msg {
            Result(i32),
            Timeout,
        }

        let cmd = task_with_timeout(Duration::from_secs(1), || Msg::Result(42), Msg::Timeout);
        assert_eq!(cmd.type_name(), "Task");
    }

    #[test]
    fn backoff_strategy_variants_debug() {
        let fixed = BackoffStrategy::Fixed { delay_ms: 100 };
        let exp = BackoffStrategy::Exponential {
            base_ms: 100,
            max_ms: 5000,
        };
        let linear = BackoffStrategy::Linear {
            base_ms: 100,
            max_ms: 500,
        };
        // Just verify Debug doesn't panic
        let _ = format!("{fixed:?}");
        let _ = format!("{exp:?}");
        let _ = format!("{linear:?}");
    }
}
