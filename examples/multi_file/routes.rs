use rooter::RouterConfig;

use crate::pages;

pub fn routes() -> RouterConfig {
    RouterConfig::new()
        .route("/", pages::home::page)
        .route("/about", pages::about::page)
        .route("/{*rest}", pages::not_found::page)
}
