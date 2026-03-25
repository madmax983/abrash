# 👺 Havoc: Prevent panic/NaN propagation on zero-duration animations

## 🧨 The Trigger
Input animation sequence with length `0.0` or `NaN` caused panic in debug and `NaN` propagation / Division-by-Zero in release.

## 📉 The Stack Trace
```
thread 'timeline_havoc_test::test_timeline_havoc' (7405) panicked at crates/abrash-anim/src/clock.rs:58:9:
Clock duration must be positive
```

## 🧪 Reproduction
Run `cargo test -p abrash-anim clock_havoc`. Fuzz logic feeds `{ 0.0, NaN }` into `AnimationClock::tick`.

## 😈 Comment
You assumed animation sequences would always take physical time to complete. You forgot instantaneous actions exist. You were wrong.
