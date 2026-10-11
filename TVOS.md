# Apple and Windows compatibility branch

`feature/tvos` merges both maintained histories:

- `Watfaq-legacy/netstack-lwip` patch `d040edd72f3b5d5e8900da1ca332f12850ea3010`
  (Windows port, UDP `local_addr`, original upstream sync through February 2025).
- `eycorsican/netstack-lwip` master `49434359ed59cae05092c1055396289ba2297389`
  (0.4.0, July 2026: memory safety, thread safety, TCP EOF/close, singleton and
  shared ownership, allocation backoff, per-platform memory sizing).
- The fork's bindgen 0.71.1 update and original tvOS build adaptation.

The vendored lwIP `core/` and `include/` sources remain upstream code. Fork
changes are confined to build/platform integration, the inherited `local_addr`
method and regression coverage:

- C compilation and bindgen share cc-rs Apple target, deployment version and
  SDK arguments, including iOS/tvOS ARM64 simulators. Bindings are generated
  for every target into `OUT_DIR`; libclang is required.
- Windows keeps the upstream port, explicitly links bcrypt, and uses 8-byte
  heap alignment on Win64 MSVC as well as GCC/Clang. `local_addr` reads the
  shared UDP PCB under the lwIP mutex.
- Keep Watfaq's removal of `LWIP_DEBUG`: this lwIP version checks whether the
  macro is defined, even when its value is zero.
- Preserve upstream 0.4.0 TCP/UDP lifecycle and Apple memory sizing. All live
  TCP streams/UDP halves own the stack; consumers must cancel and release them
  before restarting. ICMP is disabled as in both current upstreams.

## Verification

```sh
cargo test
cargo run --release --example throughput -- --bytes 8388608 --dir both
cargo run --release --example concurrent -- --tcp-down 4 --tcp-up 4 --udp-up 2 --udp-down 2 --bytes 8388608 --udp-bytes 1048576
cargo check --tests --target aarch64-apple-tvos
cargo check --tests --target aarch64-apple-tvos-sim
```

For cross-compiling Windows GNU from macOS, install the Rust target and MinGW
toolchain; set `BINDGEN_EXTRA_CLANG_ARGS_x86_64_pc_windows_gnu` to its Windows
header directory, for example
`-isystem /opt/homebrew/opt/mingw-w64/toolchain-x86_64/x86_64-w64-mingw32/include`.
Then run `cargo build --tests --examples --target x86_64-pc-windows-gnu`.
Native Windows MSVC, Linux and macOS test jobs plus Apple target compile jobs
are defined in `.github/workflows/ci.yml`; CI results require pushing the branch.

Local checks cover seven Apple architectures, Windows GNU executable linking,
TCP/UDP transfer, drop order and 16 active TCP teardown/restart cycles. netgrid
adds real socketpair TUN TCP/UDP, DNS, reload and shutdown regressions. These
host tests do not verify Apple TV packet forwarding or Windows runtime behavior.
