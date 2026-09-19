use gpui::prelude::*;
use gpui::{App, Application, Context, Entity, Window, WindowOptions, div};
use rooter::{NavLink, Router, RouterConfig};

fn routes() -> RouterConfig {
    RouterConfig::new()
        .route("/", |_, _| home())
        .route("/about", |_, _| about())
        .route("/{*rest}", |_, _| not_found())
}

fn home() -> impl IntoElement {
    div().child("Home")
}

fn about() -> impl IntoElement {
    div().child("About")
}

fn not_found() -> impl IntoElement {
    div().child("Page not found")
}

struct HelloWorld {
    router: Entity<Router>,
}

impl HelloWorld {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            router: Router::attach(window, cx, routes()),
        }
    }
}

impl Render for HelloWorld {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .child(NavLink::to("/").child(div().child("Home")))
            .child(NavLink::to("/about").child(div().child("About")))
            .child(self.router.clone())
    }
}

fn main() {
    Application::new().run(|cx: &mut App| {
        cx.activate(true);
        cx.open_window(WindowOptions::default(), |window, cx| {
            cx.new(|cx| HelloWorld::new(window, cx))
        })
        .unwrap();
    });
}
