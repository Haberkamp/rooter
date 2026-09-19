use gpui::{AnyElement, App, IntoElement, Window};

pub(crate) type RouteFactory = Box<dyn Fn(&mut Window, &mut App) -> AnyElement>;

pub struct RouterConfig {
    pub(crate) routes: Vec<(String, RouteFactory)>,
    matcher: matchit::Router<usize>,
}

impl RouterConfig {
    pub fn new() -> Self {
        Self {
            routes: Vec::new(),
            matcher: matchit::Router::new(),
        }
    }

    pub fn route<E>(
        mut self,
        path: impl Into<String>,
        factory: impl Fn(&mut Window, &mut App) -> E + 'static,
    ) -> Self
    where
        E: IntoElement,
    {
        let path = path.into();
        let index = self.routes.len();
        self.matcher
            .insert(path.clone(), index)
            .unwrap_or_else(|error| panic!("invalid or conflicting route `{path}`: {error}"));
        self.routes.push((
            path,
            Box::new(move |window, cx| factory(window, cx).into_any_element()),
        ));
        self
    }

    pub(crate) fn match_index(&self, path: &str) -> Option<usize> {
        self.matcher.at(path).ok().map(|matched| *matched.value)
    }
}

impl Default for RouterConfig {
    fn default() -> Self {
        Self::new()
    }
}
