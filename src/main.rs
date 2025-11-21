mod api;
mod app;
mod events;
mod ui;
mod util;

use crate::app::state::App;

#[tokio::main]
async fn main() -> Result<(), api::client::ApiError> {
    let initial = api::client::fetch_bazaar().await?;
    let mut app = App::new(initial);

    // Map io::Error into ApiError::Io with `?` thanks to From<std::io::Error> above
    events::run::run_app(&mut app).await?;
    Ok(())
}
