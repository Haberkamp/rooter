use gpui::prelude::*;
use gpui::{App, Application, Context, Entity, Window, WindowOptions, div};
use rooter::{NavLink, Router, RouterConfig};

fn routes() -> RouterConfig {
    RouterConfig::new()
        .route("/", home)
        .route("/about", about)
        .route("/{*rest}", not_found)
}

fn home() -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .gap_2()
        .child("Welcome to the rooter example.")
        .child("Use the links above to move between pages.")
}

fn about() -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .gap_2()
        .child("rooter is a small, window-scoped router for GPUI.")
        .child("Each window keeps its own route and navigation state.")
}

fn not_found() -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .gap_2()
        .child("The requested route is not part of this example.")
        .child("Use the navigation above to return to a known page.")
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
            .size_full()
            .flex()
            .flex_col()
            .gap_4()
            .p_4()
            .bg(gpui::white())
            .text_color(gpui::black())
            .child(
                div()
                    .flex()
                    .gap_4()
                    .child(NavLink::to("/").child(div().child("Home")))
                    .child(NavLink::to("/about").child(div().child("About"))),
            )
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
