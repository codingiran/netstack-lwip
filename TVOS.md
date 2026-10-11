# Apple SDK compatibility branch

Based on the production upstream revision
`2817bf82740e04bbee6b7bf1165f55657a6ed163` from `Watfaq/netstack-lwip`
(now redirected to `Watfaq-legacy/netstack-lwip`).

- Select the tvOS device/simulator SDK using the full Rust target; also
  select the proper SDK for ARM64 iOS simulators.
- Generate or copy Rust bindings into OUT_DIR, avoiding shared-source races.
- Explicit `BINDINGS_GEN=1` uses cc-rs target/sysroot arguments on Apple;
  Windows keeps automatic binding generation.
- Preserve the lwIP sources, TCP/UDP implementation, ICMP and monotonic clock.
  Latest upstream HEAD contains unrelated transport changes, so it is not
  used as the platform port baseline.

Validation: seven Apple architectures, archive deployment metadata and
C/Swift app-extension linking for tvOS device/simulator. Apple TV runtime
packet forwarding still requires a signed application and real-device tests.
