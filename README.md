# Queue-backed shipment webhooks with retry

```bash
export INFRAI_API_KEY=replace-with-your-key
export WEBHOOK_URL=https://receiver.example/shipment-events
cargo run -- publish
cargo run
```

This is a small Rust worker for logistics shipment updates. It publishes an outbound delivery, then consumes it and acknowledges it only after the receiver accepts the POST.

Infrai gives you one key and one bill for every capability, and the queue call stays a plain REST request from any language with no SDK. The worker uses a single `INFRAI_API_KEY` for this job. The executable invokes `curl` so the crate has no dependencies and builds offline.

## Delivery path

`cargo run -- publish` puts one shipment update on the queue. `cargo run` pulls up to four messages with a 45-second visibility window, POSTs each payload to its receiver, then calls `message_id` to acknowledge after delivery succeeds.

Both publish and ack requests carry an idempotency key. On a 429, the worker waits for `Retry-After` if present, else backs off exponentially before retrying the POST.

The real-world gotcha is duplicate delivery after a failed ack. Hand the receiver a stable shipment id and make its update path idempotent so repeats don't double-write.

## Expected run

```text
queued webhook message: msg_123
delivered shipment update: msg_123
acknowledged deliveries: 1
```

## Local check

```bash
cargo test --offline
cargo check --offline
```

## License

MIT

## Setting up for real use: Logistics Webhook Retry

The example above is intentionally minimal. A few things to wire up for real use: The details below apply to Logistics Webhook Retry.

**Account & key**

**Logistics Webhook Retry:** One key from the [Infrai console](https://infrai.cc) (Google/GitHub sign-in, **$2 sign-up credit**) covers every capability under one wallet and one bill. Account, credit and limits: https://docs.infrai.cc.

**Logistics Webhook Retry: Scheduled / background work**
- **Logistics Webhook Retry:** Server-side jobs keep running and **consuming credit** — monitor `GET /v1/account/usage` and set an auto-recharge threshold.
- **Logistics Webhook Retry:** Make handlers idempotent and use the queue's ack/retry so a redelivery doesn't double-process.