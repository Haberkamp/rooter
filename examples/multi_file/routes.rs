use rooter::RouterConfig;

use crate::pages;

pub fn routes() -> RouterConfig {
    RouterConfig::new()
        .route("/", |_, _| pages::home::page())
        .route("/about", |_, _| pages::about::page())
        .route("/{*rest}", |_, _| pages::not_found::page())
}
