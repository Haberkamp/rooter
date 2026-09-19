use gpui::div;
use gpui::prelude::*;

pub fn page() -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .gap_2()
        .child("There is no page registered for this route.")
        .child("Choose Home or About from the navigation above.")
}
