use gpui::prelude::*;
use gpui::{App, Application, Context, Entity, Window, WindowOptions, div};
use rooter::{NavLink, NavigationEvent, Router, RouterConfig};

fn routes() -> RouterConfig {
    RouterConfig::new()
        .route("/", home)
        .route("/journal", journal)
        .route("/archive", archive)
        .route("/private", private)
        .guard(|| rooter::GuardResult::Redirect("/login".into()))
        .route("/login", login)
}

fn home() -> impl IntoElement {
    div().child("Navigate, go back, replace, or open the guarded page to emit events.")
}

fn journal() -> impl IntoElement {
    div().child("This page was pushed onto the stack.")
}

fn archive() -> impl IntoElement {
    div().child("This page replaced the current history entry.")
}

fn private() -> impl IntoElement {
    div().child("This page is redirected away by a guard.")
}

fn login() -> impl IntoElement {
    div().child("The guard replaced the attempted location with /login.")
}

struct AppView {
    router: Entity<Router>,
    events: Vec<NavigationEvent>,
}

impl AppView {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let router = Router::attach(window, cx, routes());
        cx.subscribe(&router, |this, _, event, cx| {
            this.events.push(event.clone());
            cx.notify();
        })
        .detach();
        Self {
            router,
            events: Vec::new(),
        }
    }
}

impl Render for AppView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
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
                        NavLink::to("/private")
                            .when_active(None, |link| link.underline())
                            .child("Private"),
                    ),
            )
            .child(
                div()
                    .flex()
                    .gap_4()
                    .child(
                        div()
                            .id("back")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.router.update(cx, |router, cx| router.back(cx));
                            }))
                            .child("Back"),
                    )
                    .child(
                        div()
                            .id("forward")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.router.update(cx, |router, cx| router.forward(cx));
                            }))
                            .child("Forward"),
                    )
                    .child(
                        div()
                            .id("replace-archive")
                            .on_click(move |_, _, cx| {
                                replace_router
                                    .update(cx, |router, cx| router.replace("/archive", cx));
                            })
                            .child("Replace with archive"),
                    ),
            )
            .child(self.router.clone())
            .child("Caught events:")
            .child(if self.events.is_empty() {
                div().child("None yet.")
            } else {
                div().flex().flex_col().gap_1().children(
                    self.events
                        .iter()
                        .enumerate()
                        .map(|(index, event)| {
                            div().child(format!(
                                "{}. {:?} {} -> {}",
                                index + 1,
                                event.kind,
                                event.from,
                                event.to
                            ))
                        })
                        .collect::<Vec<_>>(),
                )
            })
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
