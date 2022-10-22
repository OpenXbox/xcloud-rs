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

Graphical / via WebView

```text
cargo run --bin auth_webview --features=webview
```

CLI / Manually copying rdirect URI

```text
cargo run --example sisu-auth
```

### Test Gssv Api

Note: Requires tokens (see above)

```text
cargo run --example gssv-api
```

### Test client

Note: Requires tokens (see above)

```text
cargo run --bin client_webrtc --features="xal webrtc-rs"
```