# Title: ADR-004 Retry

## Status: Accepted

## Context

Task like verification need retry with interval and max retry count.

The interval sleep should not block other event been processed.

## Decision

Introduce `NipartEvent.postpone_millis` hold integer in milliseconds unit
instructing switch thread to postpone the process of received event.

The switch use `tokio_util::time::DelayQueue` to hold those postponed events.

The `WorkFlow::process()` will check `Task::can_retry()` upon error happens
and send to switch instantly with `NipartEvent.postpone_millis` set to
non-zero.

The timeout of task and workflow is unchanged.

## Consequences

### Better

 * With tokio handling the threads, we do not need invoke new thread for each
   postponed events.

### Worse

 * The `tokio_util` is still 0.7 which is considered less stable than
   `tokio 1.x`.
