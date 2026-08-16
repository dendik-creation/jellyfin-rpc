# Jellyfin-RPC
[![License](https://shields.io/github/license/radiicall/jellyfin-rpc?color=purple)](https://www.gnu.org/licenses/gpl-3.0-standalone.html)
[![Documentation](https://docs.rs/jellyfin-rpc/badge.svg)](https://docs.rs/jellyfin-rpc/)
[![Crates.io](https://img.shields.io/crates/v/jellyfin-rpc.svg)](https://crates.io/crates/jellyfin-rpc)

This is the backend for the Jellyfin-RPC-cli and Jellyfin-RPC-Iced projects.

## Architecture

Three layers, dependencies point inwards only.

```
domain/          media, sessions, presences, blacklist rules
  ^
application/     PresenceService + the ports it needs
  ^
infrastructure/  Jellyfin HTTP, Discord IPC, imgur/litterbox, image encoding
```

* **`domain`** — pure data and rules. No HTTP, no filesystem, no Discord. The only
  outside crate is `serde`, and only on the few value objects users write
  verbatim in config files (`MediaType`, `DisplayFormat`, `Button`).
* **`application`** — `PresenceService` is the single use case: poll every
  configured source, decide what should be shown, publish it. It talks to the
  outside world through three traits in `application::ports`:
  * `MediaSource` — somewhere sessions can be read from
  * `PresenceSink` — somewhere a presence can be published
  * `ImageProvider` — turns artwork into a URL the sink can load
* **`infrastructure`** — the adapters: `JellyfinHttpSource`, `DiscordIpcSink`,
  `ImgurImageProvider`, `LitterboxImageProvider`, `DirectImageProvider`.

`RpcBuilder` is the composition root helper: frontends describe what they want
and get back a wired `PresenceService`.

Because sources are a `Vec<Box<dyn MediaSource>>`, one process can watch several
Jellyfin servers — a public one and one on `localhost`, say — and the first with
a playing session wins.

## Example

```rust
use jellyfin_rpc::{ImageHosting, RpcBuilder, ServerConfig};

let mut builder = RpcBuilder::new();
builder
    .username("user")
    .add_server(ServerConfig::new("vps", "https://jellyfin.example.com", "abcd1234"))
    .add_server(ServerConfig::new("local", "http://localhost:8096", "efgh5678"))
    .image_hosting(ImageHosting::Litterbox);

let mut service = builder.build()?;
service.connect()?;

loop {
    service.tick()?;
    std::thread::sleep(std::time::Duration::from_secs(3));
}
```

## Testing

The domain and application layers are pure, so they are tested directly. The
adapters are behind traits, so a fake `MediaSource` or `PresenceSink` is enough
to test the service without a network.
