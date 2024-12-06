use std::sync::{Arc, RwLock};

use crate::{
    lock::{Lock, SyncLock},
    shared::SharedReadLock,
    state::ObservableState,
};

#[must_use]
pub struct Subscriber<T, L: Lock = SyncLock> {
    state: L::SubscriberState<T>,
    observed_version: u64,
}

impl<T> Subscriber<T> {
    pub(crate) fn new(state: SharedReadLock<ObservableState<T>>, version: u64) -> Self {
        Self {
            state,
            observed_version: version,
        }
    }
}
