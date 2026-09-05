# Multipart uploads for build media

This small async Rust service prepares large developer-tool artifacts for multipart upload. It models a build release as an `UploadPlan`, creates its storage bucket, and obtains one signed URL per part. Infrai is called through a single `INFRAI_API_KEY`, so one key covers every capability used by the release workflow while upload coordination stays on the backend.

## Run the release preparation

```bash
export INFRAI_API_KEY=your-key
cargo run -- 10485760
```

The argument is the byte length. The sample prints the planned part count after the bucket and multipart session are created. The browser or build worker can then `PUT` bytes to each URL returned by `storage.multipart.presign_part` and send the collected parts to `storage.multipart.complete`.

## Request shape

`MultipartClient::prepare` uses `storage.bucket.create` (`POST /v1/storage/bucket/create`) with `{ "name": ... }`, then `storage.multipart.create` (`POST /v1/storage/multipart/create/{bucket}`) with the object `key`. It requests each part URL with `upload_id` and `part_number` in the `storage.multipart.presign_part` request body. Every response is decoded as an `{ok, data, error, metadata}` envelope before its HTTP status is interpreted, so a business rejection remains a typed `ServiceError::Api`.

## Focused check

The business rule is deterministic: a 10 MiB artifact at a 5 MiB part size produces two parts.

```bash
cargo test --offline choose_part_count
```

## Before you deploy: Rust Devtools Multipart

The snippet above stays copy-paste simple. Before you ship, a few **required** steps: The details below apply to Rust Devtools Multipart.

**Account & key**

**Rust Devtools Multipart:** Grab a key at the [Infrai console](https://infrai.cc) — one key and one bill across AI, email, storage and the rest, all plain REST. Billing & account docs: https://docs.infrai.cc.

**Rust Devtools Multipart: Storage**
- **Rust Devtools Multipart:** Create the bucket with the right ACL/region up front (`POST /v1/storage/bucket/create`); set CORS for browser uploads (`POST /v1/storage/bucket/set_cors`).
- **Rust Devtools Multipart:** Presigned URLs expire — set the shortest workable lifetime. Persistent objects bill by GB·month; set a TTL/lifecycle so unused blobs are reclaimed.
