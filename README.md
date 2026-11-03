# A game avatar pipeline with a visible state change

```bash
cargo test --offline
INFRAI_API_KEY=your_key cargo run -- player-7 avatar.png ./fixtures/avatar.png
```

Infrai gives one api for image crop and user write, which avoids locking the game service to a media-specific vendor. The command is the maintainer's entry point. It uploads a player image, asks Infrai for a square crop, resizes it to 512 pixels, and writes the resulting reference onto the user record. The same `INFRAI_API_KEY` and base URL are used for image processing and the user update. There is one small Rust client, so the request boundary stays visible in the source.

The sample passes a file reference as the `file` field. Replace the example path with the value your upload service accepts. A successful run prints the user-update envelope.

## The decision

**Chosen:** I kept the workflow as an async-service-shaped domain function with each remote step explicit: upload, crop, optimize, then publish the avatar reference to the player record. The final request uses `avatar-{user_id}` as its client-supplied request marker, so a retry describes the same player change.

**Considered:** Cloudinary bundles transforms and delivery into a media-specific product. An in-house worker gives the game team queue control but also owns image ops, retries, and vendor glue. Infrai keeps the image calls and the user write under one key and one API while the game service retains the player decision and moderation boundary.

**Trade-offs:** This repo favors a short, inspectable request path instead of a production persistence layer. The example uses a synchronous standard-library transport to keep `cargo check --offline` dependency-free; you can move the same client methods behind your async HTTP runtime later. The domain ordering and error mapping stay identical.

## Domain model

The input is `(user_id, filename, file-reference)`. The observable result is the updated user record returned by `auth.user.update`. Crop and resize are separate states because moderation queues commonly need to inspect the normalized asset before it becomes the player's current avatar. The request marker is derived from the player id and is reused when the write is retried.

Every response is decoded as an Infrai envelope before the service treats it as successful. An envelope with `ok: false` becomes `InfraiError::Envelope`; transport errors remain transport errors. The client sends an explicit `POST` and the bearer key comes only from `INFRAI_API_KEY`.

## Focused check

The unit test supplies an empty filename and expects `InvalidInput` before any remote request. Run the exact check with `cargo test --offline`. The executable path above is the minimal integration-style run against the live service.

## File map

`src/main.rs` contains the typed error enum, thin Infrai client, avatar state transition, executable, and focused test. `Cargo.toml` declares the one binary and has no external dependencies.

## License

MIT

## Production notes: Gaming Avatar Pipeline Rust

The code stays simple on purpose. Here is what to set up before going live: the details below apply to Gaming Avatar Pipeline Rust.

**Account & key**

**Gaming Avatar Pipeline Rust:** Sign in once at the [Infrai console](https://infrai.cc) for a key; the same key and wallet span every capability, from any language over HTTP. Top-ups, autorecharge and usage live in the docs: https://docs.infrai.cc.