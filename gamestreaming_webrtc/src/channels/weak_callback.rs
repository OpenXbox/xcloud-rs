use core::future::Future;
use std::pin::Pin;
use std::sync::Arc;

pub trait WeakAsyncCallback<T, F> {
    fn with_weak_async_callback(arc: &Arc<T>, callback: F) -> Self;
}

impl<T, F, G> WeakAsyncCallback<T, F>
    for Box<dyn FnMut() -> Pin<Box<dyn Future<Output = ()> + Send + 'static>> + Send + Sync>
where
    F: FnMut(Arc<T>) -> G + Send + Sync + 'static + Clone,
    T: Send + Sync + 'static,
    G: Future<Output = ()> + Send,
{
    fn with_weak_async_callback(arc: &Arc<T>, callback: F) -> Self {
        Box::new({
            let weak = Arc::downgrade(arc);
            move || {
                let mut callback = callback.clone();
                if let Some(arc) = weak.upgrade() {
                    Box::pin(async move { callback(arc).await })
                } else {
                    Box::pin(async move {})
                }
            }
        })
    }
}

impl<T, F, G, T1> WeakAsyncCallback<T, F>
    for Box<dyn FnMut(T1) -> Pin<Box<dyn Future<Output = ()> + Send + 'static>> + Send + Sync>
where
    F: FnMut(Arc<T>, T1) -> G + Send + Sync + 'static + Clone,
    T: Send + Sync + 'static,
    T1: Send + 'static,
    G: Future<Output = ()> + Send,
{
    fn with_weak_async_callback(arc: &Arc<T>, callback: F) -> Self {
        Box::new({
            let weak = Arc::downgrade(arc);
            move |arg1| {
                let mut callback = callback.clone();
                if let Some(arc) = weak.upgrade() {
                    Box::pin(async move { callback(arc, arg1).await })
                } else {
                    Box::pin(async move {})
                }
            }
        })
    }
}

impl<T, F, G, T1, T2> WeakAsyncCallback<T, F>
    for Box<dyn FnMut(T1, T2) -> Pin<Box<dyn Future<Output = ()> + Send + 'static>> + Send + Sync>
where
    F: FnMut(Arc<T>, T1, T2) -> G + Send + Sync + 'static + Clone,
    T: Send + Sync + 'static,
    T1: Send + 'static,
    T2: Send + 'static,
    G: Future<Output = ()> + Send,
{
    fn with_weak_async_callback(arc: &Arc<T>, callback: F) -> Self {
        Box::new({
            let weak = Arc::downgrade(arc);
            move |arg1, arg2| {
                let mut callback = callback.clone();
                if let Some(arc) = weak.upgrade() {
                    Box::pin(async move { callback(arc, arg1, arg2).await })
                } else {
                    Box::pin(async move {})
                }
            }
        })
    }
}