# rooter

A small, window-scoped router for [GPUI](https://www.gpui.rs/).

Each window owns its own router and navigation state. Routes are configured once,
and `NavLink` automatically uses the router attached to its window.

## Example

```rust
use gpui::prelude::*;
use gpui::{App, Context, Entity, Window, div};
use rooter::{NavLink, Router, RouterConfig};

fn routes() -> RouterConfig {
    RouterConfig::new()
        .route("/", |_, _| div().child("Home"))
        .route("/about", |_, _| div().child("About"))
        .route("/{*rest}", |_, _| div().child("Page not found"))
}

struct AppView {
    router: Entity<Router>,
}

impl AppView {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            router: Router::attach(window, cx, routes()),
        }
    }
}

impl Render for AppView {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .child(NavLink::to("/").child("Home"))
            .child(NavLink::to("/about").child("About"))
            .child(self.router.clone())
    }
}
```

See `examples/simple.rs` for a complete application and
`examples/multi_file/` for an application with one file per page.
`examples/nested.rs` demonstrates recursive groups, group index routes,
dynamic segments, and scoped catch-alls.
`examples/layouts.rs` demonstrates nested layouts, `Outlet`, and stateful chrome.
`examples/prefetch.rs` demonstrates route loaders, a 2s artificial delay, and hover prefetch.

```sh
cargo run --example simple
cargo run --example multi_file
cargo run --example nested
cargo run --example layouts
cargo run --example prefetch
```

## Compatibility

This crate tracks **gpui 0.2.x**. GPUI is still evolving; a GPUI breaking
release may require a breaking rooter release.

Minimum supported Rust: **1.88** (see `rust-version` in `Cargo.toml`).

## Versioning

rooter stays on **0.x** until both this API and GPUI settle. During 0.x:

- Breaking public API changes bump the **minor** version (0.1 → 0.2).
- Compatible fixes and additions bump the **patch** version.

`matchit` and `regex` are implementation details and are not re-exported.

See [CHANGELOG.md](CHANGELOG.md) for released changes.

## Development

Run the tests:

```sh
cargo test --all-targets
```

Check formatting:

```sh
cargo fmt --check
```

Run the linter:

```sh
cargo clippy --all-targets -- -D warnings
```

## License

Licensed under the MIT License. See [LICENSE](LICENSE).
