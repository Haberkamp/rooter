use gpui::prelude::*;
use gpui::{App, Application, Context, Entity, Window, WindowOptions, div};
use rooter::{NavLink, Router, RouterConfig};

fn routes() -> RouterConfig {
    RouterConfig::new()
        .route("/", home)
        .route("/journal", journal)
        .route("/archive", archive)
        .route("/settings", settings)
}

fn home() -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .gap_2()
        .child("Links push a new history entry.")
        .child("Back and Forward move through that stack.")
}

fn journal() -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .gap_2()
        .child("This journal page was pushed onto the stack.")
        .child("Use Replace with archive to overwrite this entry instead of pushing another one.")
}

fn archive() -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .gap_2()
        .child("This archive page replaced the current history entry.")
        .child("Back skips the page that was replaced.")
}

fn settings() -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .gap_2()
        .child("Navigating here after Back discards the Forward stack.")
        .child("Forward will stay unavailable until you go back again.")
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
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let location = self.router.read(cx).location().to_owned();
        let can_go_back = self.router.read(cx).can_go_back();
        let can_go_forward = self.router.read(cx).can_go_forward();
        let router = self.router.clone();
        let replace_router = self.router.clone();

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
                    .child(
                        NavLink::to("/")
                            .when_active(None, |link| link.underline())
                            .child("Home"),
                    )
                    .child(
                        NavLink::to("/journal")
                            .when_active(None, |link| link.underline())
                            .child("Journal"),
                    )
                    .child(
                        NavLink::to("/settings")
                            .when_active(None, |link| link.underline())
                            .child("Settings"),
                    ),
            )
            .child(
                div()
                    .flex()
                    .gap_4()
                    .child(
                        div()
                            .id("back")
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.router.update(cx, |router, cx| router.back(cx));
                            }))
                            .child(if can_go_back { "Back" } else { "Back (empty)" }),
                    )
                    .child(
                        div()
                            .id("forward")
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.router.update(cx, |router, cx| router.forward(cx));
                            }))
                            .child(if can_go_forward {
                                "Forward"
                            } else {
                                "Forward (empty)"
                            }),
                    )
                    .child(
                        div()
                            .id("replace-archive")
                            .on_click(move |_, _, cx| {
                                replace_router
                                    .update(cx, |router, cx| router.replace("/archive", cx));
                            })
                            .child("Replace with archive"),
                    )
                    .child(
                        div()
                            .id("open-current")
                            .on_click(move |_, _, cx| {
                                router.update(cx, |router, cx| router.navigate("/journal", cx));
                            })
                            .child("Open journal again"),
                    ),
            )
            .child(format!(
                "Location: {location}. Back: {}. Forward: {}.",
                if can_go_back { "yes" } else { "no" },
                if can_go_forward { "yes" } else { "no" }
            ))
            .child(self.router.clone())
    }
}

fn main() {
    Application::new().run(|cx: &mut App| {
        cx.activate(true);
        cx.open_window(WindowOptions::default(), |window, cx| {
            cx.new(|cx| AppView::new(window, cx))
        })
        .unwrap();
    });
}
