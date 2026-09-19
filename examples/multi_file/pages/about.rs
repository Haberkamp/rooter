use gpui::div;
use gpui::prelude::*;

pub fn page() -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .gap_2()
        .child("rooter provides simple, window-scoped routing for GPUI.")
        .child("The route table is kept separate from the individual pages.")
}
