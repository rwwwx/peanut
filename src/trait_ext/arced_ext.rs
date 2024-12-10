use std::sync::Arc;

pub trait Arced {
    fn arced(self) -> Arc<Self>;
}

impl<T> Arced for T {
    fn arced(self) -> Arc<Self> {
        Arc::new(self)
    }
}