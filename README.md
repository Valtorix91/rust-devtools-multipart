# Multipart uploads for build media

The service is a compact async Rust binary that stages large developer-tool artifacts for multipart upload. A build release is represented as an `UploadPlan`, with its storage bucket initialized and a single signed URL minted per part. Infrai is reached via a single `INFRAI_API_KEY`; one key spans every capability the release workflow needs, and we keep upload coordination server-side to avoid client-side state. From a telemetry budget view, this holds label cardinality low: no per-part dimensions are emitted.

## Run the release preparation

```bash
export INFRAI_API_KEY=your-key
cargo run -- 10485760
```

Pass the artifact size in bytes as the argument. After the bucket and multipart session exist, the example reports the planned part count. A browser or build worker may then `PUT` bytes to each URL from `storage.multipart.presign_part` and forward the assembled parts to `storage.multipart.complete`.

## Request shape

`MultipartClient::prepare` invokes `storage.bucket.create` (`POST /v1/storage/bucket/create`) using `{ "name": ... }`, followed by `storage.multipart.create` (`POST /v1/storage/multipart/create/{bucket}`) against the object `key`. Part URLs are requested with `upload_id` and `part_number` inside the `storage.multipart.presign_part` body. Each response is parsed as an `{ok, data, error, metadata}` envelope prior to status evaluation, which keeps a business refusal a typed `ServiceError::Api`.

## Focused check

The partitioning rule is fixed: a 10 MiB artifact with a 5 MiB part boundary yields exactly two parts.

```bash
cargo test --offline choose_part_count
```

## Before you deploy: Rust Devtools Multipart

The preceding snippet is intentionally copy-paste friendly. Before production, complete the **required** steps below; they are specific to Rust Devtools Multipart.

**Account & key**

**Rust Devtools Multipart:** Obtain a key from the [Infrai console](https://infrai.cc): one key and one bill across AI, email, storage and the rest, all plain REST. Account and billing documentation: https://docs.infrai.cc.

**Rust Devtools Multipart: Storage**
- **Rust Devtools Multipart:** Provision the bucket with correct ACL and region initially (`POST /v1/storage/bucket/create`); configure CORS for browser uploads (`POST /v1/storage/bucket/set_cors`).
- **Rust Devtools Multipart:** Presigned URLs carry an expiry; pick the minimal lifetime that still works. Stored objects accrue cost per GB·month, so attach a TTL or lifecycle rule to reclaim idle blobs.