
```bash
MIRIFLAGS="-Zmiri-ignore-leaks -Zmiri-disable-isolation -Zmiri-strict-provenance -Zmiri-retag-fields" cargo +nightly miri nextest run --target x86_64-unknown-linux-gnu --no-fail-fast
```