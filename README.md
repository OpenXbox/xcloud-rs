# XCloud / SmartGlass - New API in RUST

## Building

```text
git clone --recursive https://github.com/OpenXbox/xcloud-rs.git
cd xcloud-rs
cargo build
# Run tests
cargo test
# Build docs
cargo doc
```

## Examples

### Fetch tokens

CLI / Manually copying redirect URI

```text
cargo run --example auth-gssv
```

### Test Gssv Api

Note: Requires tokens (see above)

```text
cargo run --example gssv-api
```

### Test client

Note: Requires tokens (see above)

Writes audio/video tracks to file

```text
cargo run --bin client_sdl3
```