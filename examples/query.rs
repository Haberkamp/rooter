use gpui::prelude::*;
use gpui::{App, Application, Context, Entity, Window, WindowOptions, div};
use rooter::{NavLink, RouteContext, Router, RouterConfig};

fn routes() -> RouterConfig {
    RouterConfig::new()
        .route("/", home)
        .route("/search", search)
        .route("/{*rest}", not_found)
}

fn home() -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .gap_2()
        .child("Query strings are extra location state. They do not change which route matches.")
        .child("Open Search, then try the query links. The search route stays the same.")
}

fn search(route: RouteContext) -> impl IntoElement {
    let query = route.query("q");
    let page = route.optional_query_as::<u64>("page").ok().flatten();

    div()
        .flex()
        .flex_col()
        .gap_2()
        .child(match query {
            Some(query) => format!("Searching for {query}."),
            None => "Enter a search term.".to_owned(),
        })
        .child(match page {
            Some(page) => format!("Showing page {page}."),
            None => "No page query is set.".to_owned(),
        })
        .child(
            "NavLink active matching uses the path only, so every search link stays active here.",
        )
}

fn not_found() -> impl IntoElement {
    div().child("This example only has Home and Search.")
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
                        NavLink::to("/search")
                            .when_active(None, |link| link.underline())
                            .child("Search"),
                    )
                    .child(
                        NavLink::to("/search?q=rooter")
                            .when_active(None, |link| link.underline())
                            .child("Search Rooter"),
                    )
                    .child(
                        NavLink::to("/search?q=hello%20world&page=2")
                            .when_active(None, |link| link.underline())
                            .child("Page 2"),
                    ),
            )
            .child(format!("Location: {location}"))
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
