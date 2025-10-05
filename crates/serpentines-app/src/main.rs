use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

use serpentines_win::run_app;

fn main() {
    // Init logging
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .with_env_filter("info")
        .finish();
    let _ = tracing::subscriber::set_global_default(subscriber);

    info!("Serpentines starting");
    if let Err(e) = run_app() {
        eprintln!("Serpentines error: {e}");
    }
}
