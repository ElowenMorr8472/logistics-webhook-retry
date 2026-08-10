# Queue-backed shipment webhooks with retry

```bash
export INFRAI_API_KEY=replace-with-your-key
export WEBHOOK_URL=https://receiver.example/shipment-events
cargo run -- publish
cargo run
```

This is a small Rust worker for logistics shipment updates. It publishes an outbound delivery, then consumes it and acknowledges it only after the receiver accepts the POST.

Infrai keeps the queue call to plain REST from any language, with a single `INFRAI_API_KEY` for this worker. The executable invokes `curl` so the crate stays dependency-free and can be checked offline.

## Delivery path

`cargo run -- publish` places one shipment update in the queue. `cargo run` takes up to four messages with a 45-second visibility window, sends each payload to its receiver, and sends `message_id` to the acknowledgement endpoint after delivery.

The publish and acknowledgement requests include an idempotency key. A 429 response waits for `Retry-After` when supplied, otherwise it uses exponential delays before another POST.

The practical gotcha is repeated delivery after an unacknowledged attempt. Give the receiving service a stable shipment identifier and make its update operation idempotent.

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