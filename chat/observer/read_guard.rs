use crate::{
    lock::{Lock, SyncLock},
    state::ObservableState,
};
use derive_tools::Deref;

#[derive(Debug, Deref)]
pub struct ObservableReadGuard<'a, T: 'a, L: Lock = SyncLock> {
    inner: L::SharedReadGuard<'a, ObservableState<T>>,
}

impl<'a, T: 'a, L: Lock> ObservableReadGuard<'a, T, L> {
    pub(crate) fn new(inner: L::SharedReadGuard<'a, ObservableState<T>>) -> Self {
        Self { inner }
    }
}
