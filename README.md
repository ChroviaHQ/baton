# Baton

Baton is a desktop proxy relay for macOS and Windows. It chains an existing local proxy with a remote landing proxy and serves the combined route through a single local endpoint.

<p align="center">
  <img src="docs/screenshot.png" alt="Baton" width="400">
</p>

```
Application ──► Baton (127.0.0.1:1080) ──► Upstream proxy ──► Landing proxy ──► Destination
```

## Features

- Accepts HTTP and SOCKS5 connections on a single local port.
- Chains two hops over HTTP CONNECT or SOCKS5, with optional authentication on each hop.
- Uses a fixed upstream address or the operating system's current proxy setting as the first hop.
- Optionally acts as the system proxy and restores the original settings when the relay stops.
- Reports the egress IP address and live traffic statistics.

## Getting Started

1. Set the upstream to the address of an existing local proxy, or select **System proxy**.
2. Set the landing proxy.
3. Select **Start Relay** and configure applications to use `127.0.0.1:1080`.

## Building from Source

Building requires Rust, Node.js 22 or later, and the [Tauri prerequisites](https://tauri.app/start/prerequisites/) for your platform.

```sh
npm install
npm run tauri build
```

Use `npm run tauri dev` to run the app in development mode.

## License

Baton is released under the [MIT License](LICENSE).
