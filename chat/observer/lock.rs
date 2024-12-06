use std::{
    ops::{Deref, DerefMut},
    sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard},
};

use crate::{
    shared::{Shared, SharedReadGuard, SharedReadLock},
    state::ObservableState,
    subscriber::Subscriber,
};

pub trait Lock {
    type RwLock<T>;
    type Shared<T>: Deref<Target = T>;
    type SharedReadGuard<'a, T>
    where
        T: 'a;
    type SubscriberState<S>;
    type RwLockReadGuard<'a, T: 'a>: Deref<Target = T>;
    type RwLockWriteGuard<'a, T>
    where
        T: 'a;
    fn new_rwlock<T>(value: T) -> Self::RwLock<T>;
    fn read_noblock<T>(lock: &Self::RwLock<T>) -> Self::RwLockReadGuard<'_, T>;

    fn new_shared<T>(value: T) -> Self::Shared<T>;
    fn shared_read_count<T>(shared: &Self::Shared<T>) -> usize;
    fn shared_into_inner<T>(shared: Self::Shared<T>) -> Arc<Self::RwLock<T>>;
}

pub enum SyncLock {}

impl Lock for SyncLock {
    type RwLock<T> = RwLock<T>;
    type Shared<T> = Shared<T>;
    type SharedReadGuard<'a, T>
        = SharedReadGuard<'a, T>
    where
        T: 'a;
    type SubscriberState<S> = SharedReadLock<ObservableState<S>>;
    type RwLockWriteGuard<'a, T>
        = RwLockWriteGuard<'a, T>
    where
        T: 'a;
    type RwLockReadGuard<'a, T: 'a> = RwLockReadGuard<'a, T>;

    fn new_rwlock<T>(value: T) -> Self::RwLock<T> {
        Self::RwLock::new(value)
    }
    fn read_noblock<T>(lock: &Self::RwLock<T>) -> Self::RwLockReadGuard<'_, T> {
        lock.try_read().unwrap()
    }

    fn new_shared<T>(value: T) -> Self::Shared<T> {
        Self::Shared::new(value)
    }
    fn shared_read_count<T>(shared: &Self::Shared<T>) -> usize {
        Self::Shared::read_count(shared)
    }
    fn shared_into_inner<T>(shared: Self::Shared<T>) -> Arc<Self::RwLock<T>> {
        Self::Shared::into_inner(shared)
    }
}

#[must_use]
pub struct Next<'a, T, L: Lock = SyncLock> {
    subscriber: &'a mut Subscriber<T, L>,
}

impl<'a, T> Next<'a, T> {
    fn new(subscriber: &'a mut Subscriber<T>) -> Self {
        Self { subscriber }
    }
}

// impl<T: Clone> Future for Next<'_, T> {
//     type Output = Option<T>;

//     fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
//         self.subscriber.poll_next_ref(cx).map(opt_guard_to_owned)
//     }
// }

// fn opt_guard_to_owned<T: Clone>(value: Option<ObservableReadGuard<'_, T>>) -> Option<T> {
//     value.map(|guard| guard.to_owned())
// }

// #[cfg(feature = "async-lock")]
// pub enum AsyncLock {}

// #[cfg(feature = "async-lock")]
// impl Lock for AsyncLock {
//     type RwLock<T> = async_lock::RwLock<T>;
//     type SubscriberState<S> = async_state::AsyncSubscriberState<S>;
// }
