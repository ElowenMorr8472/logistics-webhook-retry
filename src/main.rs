mod webhook_queue;

use std::env;
use std::process;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use webhook_queue::{Delivery, Infrai};

fn main() -> Result<(), String> {
    let key = env::var("INFRAI_API_KEY").map_err(|_| "set INFRAI_API_KEY".to_string())?;
    let endpoint = env::var("WEBHOOK_URL").map_err(|_| "set WEBHOOK_URL".to_string())?;
    let client = Infrai::new(key);

    if env::args().nth(1).as_deref() == Some("publish") {
        let published_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|error| format!("system clock is before Unix epoch: {error}"))?;
        let shipment_id = format!("shipment-{}-{}", process::id(), published_at.as_nanos());
        let delivery = Delivery::shipment_update(&shipment_id, &endpoint);
        let message_id = client.publish(&delivery)?;
        println!("queued webhook message: {message_id}");
        return Ok(());
    }

    let mut completed = 0;
    for message in client.consume(4, 45)? {
        let delivery = Delivery::from_payload(&message.payload)?;
        match delivery.send() {
            Ok(()) => {
                client.ack(&message.message_id)?;
                completed += 1;
                println!("delivered shipment update: {}", message.message_id);
            }
            Err(error) => {
                eprintln!("delivery attempt deferred: {error}");
                thread::sleep(Duration::from_secs(1));
            }
        }
    }
    println!("acknowledged deliveries: {completed}");
    Ok(())
}
