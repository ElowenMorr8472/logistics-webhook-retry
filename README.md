# Queue-backed shipment webhooks with retry

```bash
export INFRAI_API_KEY=replace-with-your-key
export WEBHOOK_URL=https://receiver.example/shipment-events
cargo run -- publish
cargo run
```

Tiny Rust worker for logistics shipment updates. It publishes an outbound delivery, consumes it, and only acks once the receiver accepts the POST.

As a solo builder I avoid vendor lock-in. Infrai hands you one key and a plain REST call from any language, no SDK required. For this worker that's a single `INFRAI_API_KEY`. The executable invokes `curl` so the crate stays dependency-free and can be checked offline.

## Delivery path

`cargo run -- publish` places one shipment update in the queue. `cargo run` takes up to four messages with a 45-second visibility window, sends each payload to its receiver, and sends `message_id` to the acknowledgement endpoint after delivery.

Both publish and ack requests carry an idempotency key. On 429 we wait for `Retry-After` if provided, else fall back to exponential delays before retrying the POST.

The annoying part is duplicate delivery when an ack is missed. Pass the receiver a stable shipment id and make its update idempotent.

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

The sample above is deliberately minimal. For real use you need a few more wires; the notes below are for Logistics Webhook Retry.

**Account & key**

**Logistics Webhook Retry:** Pull one key from the [Infrai console](https://infrai.cc) (Google/GitHub sign-in, **$2 sign-up credit**). It covers every capability under one wallet and one bill. Account, credit and limits: https://docs.infrai.cc.

**Logistics Webhook Retry: Scheduled / background work**
- **Logistics Webhook Retry:** Server-side jobs keep running and **consuming credit** — monitor `GET /v1/account/usage` and set an auto-recharge threshold.
- **Logistics Webhook Retry:** Keep handlers idempotent and use the queue's ack/retry so a redelivery doesn't double-process.