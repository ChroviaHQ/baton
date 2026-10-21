# Baton

Baton chains a local proxy, such as Clash, with a remote landing proxy and exposes the chain as one local endpoint for macOS and Windows.

```
apps ──► Baton (127.0.0.1:6868) ──► upstream ──► landing proxy ──► internet
```

## Features

- HTTP and SOCKS5 on one local port, with HTTP or SOCKS5 hops and optional authentication
- Upstream from a fixed address or the current system proxy
- Optional system proxy mode, restored automatically on exit
- Egress IP check, live traffic stats, and tray controls

## Usage

1. Set the upstream to your local proxy port, or choose **System proxy**.
2. Set the landing proxy.
3. Click **Start Relay** and point your apps at `127.0.0.1:6868`.

## Build

Requires Rust, Node.js, and the [Tauri prerequisites](https://tauri.app/start/prerequisites/).

```sh
npm install
npm run tauri dev
npm run tauri build
```

## License

[MIT](LICENSE)
