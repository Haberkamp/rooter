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

```sh
cargo run --example simple
cargo run --example multi_file
cargo run --example nested
cargo run --example layouts
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

## Navigation behavior

`navigate` and `replace` no-op (no history change, no `NavigationEvent`) when
the path normalizes to the current location. `back` and `forward` no-op at the
ends of the stack. `Router::url` returns `UrlError` for an unknown name or a
missing path parameter; extra parameters become query string entries.

`navigate_window` / `replace_window` / `back_window` / `forward_window` return
`false` only when the window has no attached router. They return `true` even
when the underlying method no-ops.

## Layouts and stateful chrome

`.layout()` wraps every route in the current group. Put `Outlet` where the child
route should render. Nested groups stack layouts from the inside out.

Layout and page factories run again every time the router renders. They should
return elements, not create long-lived state. If chrome needs to remember
anything across navigations (a collapsed sidebar, a filter, scroll position),
create a GPUI `Entity` once on the window root and clone it into the layout:

```rust
struct AppView {
    router: Entity<Router>,
}

impl AppView {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let sidebar = cx.new(|_| Sidebar { collapsed: false });
        Self {
            router: Router::attach(window, cx, routes(sidebar)),
        }
    }
}

fn routes(sidebar: Entity<Sidebar>) -> RouterConfig {
    RouterConfig::new().group("/dashboard", |routes| {
        routes.layout(move || dashboard(sidebar.clone()))
    })
}

fn dashboard(sidebar: Entity<Sidebar>) -> impl IntoElement {
    div().child(sidebar).child(Outlet::new())
}
```

The layout still rebuilds each frame. The `Sidebar` entity does not, so its
fields survive leaving `/dashboard` and coming back. Creating that entity
inside the layout factory would reset it on every render.

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
