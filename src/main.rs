mod app;
mod events;
mod ui;
mod util;

use crate::app::state::App;
use hypixel::HypixelClient;
use std::error::Error;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // The bazaar endpoint is keyless. One client is shared by the initial load
    // and every background refresh; clones reuse its connection pool.
    let client = HypixelClient::builder()
        .timeout(Duration::from_secs(10))
        .retry_on_rate_limit(2)
        .build();

    let initial = client.skyblock_bazaar().await?;
    let mut app = App::new(client, initial);

    events::run::run_app(&mut app).await?;
    Ok(())
}
