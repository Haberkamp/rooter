use std::any::Any;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::{Duration, Instant};

use gpui::{App, Window};

use crate::config::RouteContext;

/// How long a loaded route resource is reused before the loader runs again.
///
/// Used when [`Router::prefetch`](crate::Router::prefetch) / [`NavLink::prefetch`](crate::NavLink::prefetch)
/// do not set a duration. Matches Inertia's 30s prefetch default.
pub const DEFAULT_CACHE_FOR: Duration = Duration::from_secs(30);

/// Cache lifetime passed to [`Router::prefetch`](crate::Router::prefetch) and
/// [`NavLink::prefetch`](crate::NavLink::prefetch).
///
/// A [`Duration`] is used as-is. [`None`] uses [`DEFAULT_CACHE_FOR`].
pub trait IntoCacheFor {
    fn into_cache_for(self) -> Duration;
}

/// Target for [`Router::invalidate`](crate::Router::invalidate).
///
/// [`Invalidate::all`] drops every cached resource. [`Invalidate::path`] drops
/// that location. [`Invalidate::named`] with params builds the same URL as
/// [`Router::url`](crate::Router::url). A named route without required params
/// drops every cached location that matches that route.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Invalidate {
    kind: InvalidateKind,
    params: Vec<(String, String)>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum InvalidateKind {
    All,
    Path(String),
    Named(String),
}

impl Invalidate {
    pub fn all() -> Self {
        Self {
            kind: InvalidateKind::All,
            params: Vec::new(),
        }
    }

    pub fn path(path: impl Into<String>) -> Self {
        Self {
            kind: InvalidateKind::Path(path.into()),
            params: Vec::new(),
        }
    }

    pub fn named(name: impl Into<String>) -> Self {
        Self {
            kind: InvalidateKind::Named(name.into()),
            params: Vec::new(),
        }
    }

    pub fn param(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.params.push((key.into(), value.into()));
        self
    }

    pub(crate) fn kind(&self) -> &InvalidateKind {
        &self.kind
    }

    pub(crate) fn params(&self) -> &[(String, String)] {
        &self.params
    }
}

impl IntoCacheFor for Duration {
    fn into_cache_for(self) -> Duration {
        self
    }
}

impl IntoCacheFor for Option<Duration> {
    fn into_cache_for(self) -> Duration {
        self.unwrap_or(DEFAULT_CACHE_FOR)
    }
}

pub(crate) type ErasedValue = Arc<dyn Any + Send + Sync>;
pub(crate) type LoaderFuture =
    Pin<Box<dyn Future<Output = Result<ErasedValue, ErasedValue>> + 'static>>;
pub(crate) type RouteLoader =
    std::rc::Rc<dyn Fn(RouteContext, &mut Window, &mut App) -> LoaderFuture>;

#[derive(Clone)]
pub(crate) enum StoredResource {
    Loading,
    Ready {
        value: ErasedValue,
        expires_at: Instant,
    },
    Error {
        error: ErasedValue,
        expires_at: Instant,
    },
}

impl StoredResource {
    pub(crate) fn is_fresh(&self, now: Instant) -> bool {
        match self {
            Self::Loading => true,
            Self::Ready { expires_at, .. } | Self::Error { expires_at, .. } => *expires_at > now,
        }
    }

    pub(crate) fn downcast<T, E>(&self) -> Resource<T, E>
    where
        T: Send + Sync + 'static,
        E: Send + Sync + 'static,
    {
        match self {
            Self::Loading => Resource::loading(),
            Self::Ready { value, .. } => Resource {
                inner: ResourceInner::Ready(value.clone().downcast::<T>().unwrap_or_else(|_| {
                    panic!("route resource type does not match the page `Resource`")
                })),
            },
            Self::Error { error, .. } => Resource {
                inner: ResourceInner::Error(error.clone().downcast::<E>().unwrap_or_else(|_| {
                    panic!("route resource error type does not match the page `Resource`")
                })),
            },
        }
    }
}

/// Snapshot of a route loader.
///
/// Loaders run in the background. Navigation does not wait for them. Prefetch
/// and the matched page share the same slot, so a hover that started a request
/// is the request the page observes after click.
#[derive(Clone)]
pub struct Resource<T, E = String> {
    inner: ResourceInner<T, E>,
}

#[derive(Clone)]
enum ResourceInner<T, E> {
    Loading,
    Ready(Arc<T>),
    Error(Arc<E>),
}

impl<T, E> Resource<T, E> {
    pub(crate) fn loading() -> Self {
        Self {
            inner: ResourceInner::Loading,
        }
    }

    pub fn is_loading(&self) -> bool {
        matches!(self.inner, ResourceInner::Loading)
    }

    pub fn has_error(&self) -> bool {
        matches!(self.inner, ResourceInner::Error(_))
    }

    pub fn get(&self) -> Option<&T> {
        match &self.inner {
            ResourceInner::Ready(value) => Some(value),
            _ => None,
        }
    }

    pub fn error(&self) -> Option<&E> {
        match &self.inner {
            ResourceInner::Error(error) => Some(error),
            _ => None,
        }
    }
}

pub trait LoaderFactory<Kind, T, E>: 'static {
    #[doc(hidden)]
    fn load(
        &self,
        route: RouteContext,
        window: &mut Window,
        cx: &mut App,
    ) -> impl Future<Output = Result<T, E>> + 'static;
}

impl<F, Fut, T, E> LoaderFactory<crate::WithoutContext, T, E> for F
where
    F: Fn() -> Fut + 'static,
    Fut: Future<Output = Result<T, E>> + 'static,
{
    fn load(
        &self,
        _route: RouteContext,
        _window: &mut Window,
        _cx: &mut App,
    ) -> impl Future<Output = Result<T, E>> + 'static {
        self()
    }
}

impl<F, Fut, T, E> LoaderFactory<crate::WithRoute, T, E> for F
where
    F: Fn(RouteContext) -> Fut + 'static,
    Fut: Future<Output = Result<T, E>> + 'static,
{
    fn load(
        &self,
        route: RouteContext,
        _window: &mut Window,
        _cx: &mut App,
    ) -> impl Future<Output = Result<T, E>> + 'static {
        self(route)
    }
}

impl<F, Fut, T, E> LoaderFactory<crate::WithRouteContext, T, E> for F
where
    F: Fn(RouteContext, &mut Window, &mut App) -> Fut + 'static,
    Fut: Future<Output = Result<T, E>> + 'static,
{
    fn load(
        &self,
        route: RouteContext,
        window: &mut Window,
        cx: &mut App,
    ) -> impl Future<Output = Result<T, E>> + 'static {
        self(route, window, cx)
    }
}

pub(crate) fn erase_loader<L, Kind, T, E>(loader: L) -> RouteLoader
where
    L: LoaderFactory<Kind, T, E>,
    T: Send + Sync + 'static,
    E: Send + Sync + 'static,
{
    std::rc::Rc::new(move |route, window, cx| {
        let future = loader.load(route, window, cx);
        Box::pin(async move {
            match future.await {
                Ok(value) => Ok(Arc::new(value) as ErasedValue),
                Err(error) => Err(Arc::new(error) as ErasedValue),
            }
        })
    })
}
