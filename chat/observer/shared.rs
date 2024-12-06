use std::{
    hash::Hash,
    sync::{
        Arc, LockResult, PoisonError, RwLock, RwLockReadGuard, RwLockWriteGuard, TryLockError,
        TryLockResult, Weak,
    },
};

use derive_tools::*;

use crate::{
    lock::{Lock, SyncLock},
    read_guard::ObservableReadGuard,
    state::ObservableState,
    subscriber::Subscriber,
};

#[derive(Debug, Default)]
pub struct Shared<T: ?Sized>(Arc<RwLock<T>>);

impl<T> Shared<T> {
    pub fn new(data: T) -> Self {
        Self(Arc::new(RwLock::new(data)))
    }

    pub fn unwrap(this: Self) -> Result<T, Self> {
        match Arc::try_unwrap(this.0) {
            Ok(rwlock) => Ok(rwlock.into_inner().unwrap()),
            Err(arc) => Err(Self(arc)),
        }
    }
}

impl<T: ?Sized> std::ops::Deref for Shared<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        Shared::get(self)
    }
}

impl<T: ?Sized> Shared<T> {
    #[track_caller]
    pub fn get(this: &Self) -> &T {
        Self::try_get(this).unwrap()
    }

    pub fn try_get(this: &Self) -> LockResult<&T> {
        match this.0.read() {
            Ok(read_guard) => Ok(unsafe { readguard_into_ref(read_guard) }),
            Err(err) => Err(poison_error_map(err, |read_guard| unsafe {
                readguard_into_ref(read_guard)
            })),
        }
    }

    /// Lock this `Shared` to be able to mutate it, blocking the current thread
    /// until the operation succeeds.
    pub fn lock(this: &mut Self) -> SharedWriteGuard<'_, T> {
        SharedWriteGuard(this.0.write().unwrap())
    }

    /// Get a [`SharedReadLock`] for accessing the same resource read-only from
    /// elsewhere.
    pub fn get_read_lock(this: &Self) -> SharedReadLock<T> {
        SharedReadLock(this.0.clone())
    }

    /// Attempt to create a `Shared` from its internal representation,
    /// `Arc<RwLock<T>>`.
    ///
    /// This returns `Ok(_)` only if there are no further references (including
    /// weak references) to the inner `RwLock` since otherwise, `Shared`s
    /// invariant of being the only instance that can mutate the inner value
    /// would be broken.
    pub fn try_from_inner(rwlock: Arc<RwLock<T>>) -> Result<Self, Arc<RwLock<T>>> {
        if Arc::strong_count(&rwlock) == 1 && Arc::weak_count(&rwlock) == 0 {
            Ok(Self(rwlock))
        } else {
            Err(rwlock)
        }
    }

    /// Turns this `Shared` into its internal representation, `Arc<RwLock<T>>`.
    pub fn into_inner(this: Self) -> Arc<RwLock<T>> {
        this.0
    }

    /// Gets the number of associated [`SharedReadLock`]s.
    pub fn read_count(this: &Self) -> usize {
        Arc::strong_count(&this.0) - 1
    }

    /// Gets the number of associated [`WeakReadLock`]s.
    pub fn weak_count(this: &Self) -> usize {
        Arc::weak_count(&this.0)
    }
}

/// SAFETY: Only allowed for a read guard obtained from the inner value of a
/// `Shared`. Transmuting lifetime here, this is okay because the resulting
/// reference's borrows this, which is the only `Shared` instance that could
/// mutate the inner value (you can not have two `Shared`s that reference the
/// same inner value) and the other references that can exist to the inner value
/// are only allowed to read as well.
unsafe fn readguard_into_ref<'a, T: ?Sized + 'a>(guard: RwLockReadGuard<'a, T>) -> &'a T {
    let reference: &T = &guard;
    &*(reference as *const T)
}

#[derive(Debug, Clone)]
pub struct SharedReadLock<T: ?Sized>(Arc<RwLock<T>>);

impl<T: ?Sized> SharedReadLock<T> {
    /// Lock this `SharedReadLock`, blocking the current thread until the
    /// operation succeeds.
    pub fn lock(&self) -> SharedReadGuard<'_, T> {
        SharedReadGuard(self.0.read().unwrap())
    }

    /// Try to lock this `SharedReadLock`.
    ///
    /// If the value is currently locked for writing through the corresponding
    /// `Shared` instance or the lock was poisoned, returns [`TryLockError`].
    pub fn try_lock(&self) -> TryLockResult<SharedReadGuard<'_, T>> {
        self.0
            .try_read()
            .map(SharedReadGuard)
            .map_err(|err| try_lock_error_map(err, SharedReadGuard))
    }

    /// Create a new [`WeakReadLock`] pointer to this allocation.
    pub fn downgrade(&self) -> WeakReadLock<T> {
        WeakReadLock(Arc::downgrade(&self.0))
    }

    /// Upgrade a `SharedReadLock` to `Shared`.
    ///
    /// This only return `Ok(_)` if there are no other references (including a
    /// `Shared`, or weak references) to the inner value, since otherwise it
    /// would be possible to have multiple `Shared`s for the same inner value
    /// alive at the same time, which would violate `Shared`s invariant of
    /// being the only reference that is able to mutate the inner value.
    pub fn try_upgrade(self) -> Result<Shared<T>, Self> {
        if Arc::strong_count(&self.0) == 1 && Arc::weak_count(&self.0) == 0 {
            Ok(Shared(self.0))
        } else {
            Err(self)
        }
    }

    /// Create a `SharedReadLock` from its internal representation,
    /// `Arc<RwLock<T>>`.
    ///
    /// You can use this to create a `SharedReadLock` from a shared `RwLock`
    /// without ever using `Shared`, if you want to expose an API where there is
    /// a value that can be written only from inside one module or crate, but
    /// outside users should be allowed to obtain a reusable lock for reading
    /// the inner value.
    pub fn from_inner(rwlock: Arc<RwLock<T>>) -> Self {
        Self(rwlock)
    }

    /// Attempt to turn this `SharedReadLock` into its internal representation,
    /// `Arc<RwLock<T>>`.
    ///
    /// This returns `Ok(_)` only if there are no further references (including
    /// a `Shared`, or weak references) to the inner value, since otherwise
    /// it would be possible to have a `Shared` and an `Arc<RwLock<T>>` for
    /// the same inner value alive at the same time, which would violate
    /// `Shared`s invariant of being the only reference that is able to
    /// mutate the inner value.
    pub fn try_into_inner(self) -> Result<Arc<RwLock<T>>, Self> {
        if Arc::strong_count(&self.0) == 1 && Arc::weak_count(&self.0) == 0 {
            Ok(self.0)
        } else {
            Err(self)
        }
    }
}

#[derive(Clone, Debug)]
pub struct WeakReadLock<T: ?Sized>(Weak<RwLock<T>>);

impl<T: ?Sized> WeakReadLock<T> {
    /// Attempt to upgrade the `WeakReadLock` into a `SharedReadLock`, delaying
    /// dropping of the inner value if successful.
    ///
    /// Returns `None` if the inner value has already been dropped.
    pub fn upgrade(&self) -> Option<SharedReadLock<T>> {
        Weak::upgrade(&self.0).map(SharedReadLock)
    }
}

#[derive(Deref, Debug)]
pub struct SharedReadGuard<'a, T: ?Sized>(RwLockReadGuard<'a, T>);

impl<'a, T: ?Sized + 'a> SharedReadGuard<'a, T> {
    /// Create a `SharedReadGuard` from its internal representation,
    /// `RwLockReadGuard<'a, T>`.
    pub fn from_inner(guard: RwLockReadGuard<'a, T>) -> Self {
        Self(guard)
    }
}

#[derive(Deref, DerefMut, Debug)]
pub struct SharedWriteGuard<'a, T: ?Sized>(RwLockWriteGuard<'a, T>);

impl<'a, T: ?Sized> SharedWriteGuard<'a, T> {
    /// Create a `SharedWriteGuard` from its internal representation,
    /// `RwLockWriteGuard<'a, T>`.
    pub fn from_inner(guard: RwLockWriteGuard<'a, T>) -> Self {
        Self(guard)
    }
}

#[derive(Debug)]
pub struct SharedObservable<T, L: Lock = SyncLock> {
    state: Arc<L::RwLock<ObservableState<T>>>,
    /// Ugly hack to track the amount of clones of this observable,
    /// *excluding subscribers*.
    _num_clones: Arc<()>,
}

impl<T> SharedObservable<T> {
    /// Create a new `SharedObservable` with the given initial value.
    #[must_use]
    pub fn new(value: T) -> Self {
        Self::from_inner(Arc::new(std::sync::RwLock::new(ObservableState::new(
            value,
        ))))
    }

    /// Obtain a new subscriber.
    ///
    /// Calling `.next().await` or `.next_ref().await` on the returned
    /// subscriber only resolves once the inner value has been updated again
    /// after the call to `subscribe`.
    ///
    /// See [`subscribe_reset`][Self::subscribe_reset] if you want to obtain a
    /// subscriber that immediately yields without any updates.
    pub fn subscribe(&self) -> Subscriber<T> {
        let version = self.state.read().unwrap().version();
        Subscriber::new(SharedReadLock::from_inner(Arc::clone(&self.state)), version)
    }

    /// Obtain a new subscriber that immediately yields.
    ///
    /// `.subscribe_reset()` is equivalent to `.subscribe()` with a subsequent
    /// call to [`.reset()`][Subscriber::reset] on the returned subscriber.
    ///
    /// In contrast to [`subscribe`][Self::subscribe], calling `.next().await`
    /// or `.next_ref().await` on the returned subscriber before updating the
    /// inner value yields the current value instead of waiting. Further calls
    /// to either of the two will wait for updates.
    pub fn subscribe_reset(&self) -> Subscriber<T> {
        Subscriber::new(SharedReadLock::from_inner(Arc::clone(&self.state)), 0)
    }

    /// Get a clone of the inner value.
    pub fn get(&self) -> T
    where
        T: Clone,
    {
        self.state.read().unwrap().get().clone()
    }

    /// Lock the inner with shared read access, blocking the current thread
    /// until the lock can be acquired.
    ///
    /// While the returned read guard is alive, nobody can update the inner
    /// value. If you want to update the value based on the previous value, do
    /// **not** use this method because it can cause races with other clones of
    /// the same `SharedObservable`. Instead, call of of the `update_` methods,
    /// or if that doesn't fit your use case, call [`write`][Self::write]
    /// and update the value through the write guard it returns.
    pub fn read(&self) -> ObservableReadGuard<'_, T> {
        ObservableReadGuard::new(SharedReadGuard::from_inner(self.state.read().unwrap()))
    }

    /// Attempts to acquire shared read access to the inner value.
    ///
    /// See [`RwLock`s documentation](https://doc.rust-lang.org/std/sync/struct.RwLock.html#method.try_read)
    /// for details.
    pub fn try_read(&self) -> TryLockResult<ObservableReadGuard<'_, T>> {
        match self.state.try_read() {
            Ok(guard) => Ok(ObservableReadGuard::new(SharedReadGuard::from_inner(guard))),
            Err(TryLockError::Poisoned(e)) => Err(TryLockError::Poisoned(PoisonError::new(
                ObservableReadGuard::new(SharedReadGuard::from_inner(e.into_inner())),
            ))),
            Err(TryLockError::WouldBlock) => Err(TryLockError::WouldBlock),
        }
    }

    /// Lock the inner with exclusive write access, blocking the current thread
    /// until the lock can be acquired.
    ///
    /// This can be used to set a new value based on the existing value. The
    /// returned write guard dereferences (immutably) to the inner type, and has
    /// associated functions to update it.
    pub fn write(&self) -> ObservableWriteGuard<'_, T> {
        ObservableWriteGuard::new(self.state.write().unwrap())
    }

    /// Attempts to acquire exclusive write access to the inner value.
    ///
    /// See [`RwLock`s documentation](https://doc.rust-lang.org/std/sync/struct.RwLock.html#method.try_write)
    /// for details.
    pub fn try_write(&self) -> TryLockResult<ObservableWriteGuard<'_, T>> {
        match self.state.try_write() {
            Ok(guard) => Ok(ObservableWriteGuard::new(guard)),
            Err(TryLockError::Poisoned(e)) => Err(TryLockError::Poisoned(PoisonError::new(
                ObservableWriteGuard::new(e.into_inner()),
            ))),
            Err(TryLockError::WouldBlock) => Err(TryLockError::WouldBlock),
        }
    }

    /// Set the inner value to the given `value`, notify subscribers and return
    /// the previous value.
    pub fn set(&self, value: T) -> T {
        self.state.write().unwrap().set(value)
    }

    /// Set the inner value to the given `value` if it doesn't compare equal to
    /// the existing value.
    ///
    /// If the inner value is set, subscribers are notified and
    /// `Some(previous_value)` is returned. Otherwise, `None` is returned.
    pub fn set_if_not_eq(&self, value: T) -> Option<T>
    where
        T: PartialEq,
    {
        self.state.write().unwrap().set_if_not_eq(value)
    }

    /// Set the inner value to the given `value` if it has a different hash than
    /// the existing value.
    ///
    /// If the inner value is set, subscribers are notified and
    /// `Some(previous_value)` is returned. Otherwise, `None` is returned.
    pub fn set_if_hash_not_eq(&self, value: T) -> Option<T>
    where
        T: Hash,
    {
        self.state.write().unwrap().set_if_hash_not_eq(value)
    }

    /// Set the inner value to a `Default` instance of its type, notify
    /// subscribers and return the previous value.
    ///
    /// Shorthand for `observable.set(T::default())`.
    pub fn take(&self) -> T
    where
        T: Default,
    {
        self.set(T::default())
    }

    /// Update the inner value and notify subscribers.
    ///
    /// Note that even if the inner value is not actually changed by the
    /// closure, subscribers will be notified as if it was. Use
    /// [`update_if`][Self::update_if] if you want to conditionally mutate the
    /// inner value.
    pub fn update(&self, f: impl FnOnce(&mut T)) {
        self.state.write().unwrap().update(f);
    }

    /// Maybe update the inner value and notify subscribers if it changed.
    ///
    /// The closure given to this function must return `true` if subscribers
    /// should be notified of a change to the inner value.
    pub fn update_if(&self, f: impl FnOnce(&mut T) -> bool) {
        self.state.write().unwrap().update_if(f);
    }
}

impl<T, L: Lock> SharedObservable<T, L> {
    pub(crate) fn from_inner(state: Arc<L::RwLock<ObservableState<T>>>) -> Self {
        Self {
            state,
            _num_clones: Arc::new(()),
        }
    }

    /// Get the number of `SharedObservable` clones.
    ///
    /// This always returns at least `1` since `self` is included in the count.
    ///
    /// Be careful when using this. The result is only reliable if it is exactly
    /// `1`, as otherwise it could be incremented right after your call to this
    /// function, before you look at its result or do anything based on that.
    #[must_use]
    pub fn observable_count(&self) -> usize {
        Arc::strong_count(&self._num_clones)
    }

    /// Get the number of subscribers.
    ///
    /// Be careful when using this. The result can change right after your call
    /// to this function, before you look at its result or do anything based
    /// on that.
    #[must_use]
    pub fn subscriber_count(&self) -> usize {
        self.strong_count() - self.observable_count()
    }

    /// Get the number of strong references to the inner value.
    ///
    /// Every clone of the `SharedObservable` and every associated `Subscriber`
    /// holds a reference, so this is the sum of all clones and subscribers.
    /// This always returns at least `1` since `self` is included in the count.
    ///
    /// Equivalent to `ob.observable_count() + ob.subscriber_count()`.
    ///
    /// Be careful when using this. The result is only reliable if it is exactly
    /// `1`, as otherwise it could be incremented right after your call to this
    /// function, before you look at its result or do anything based on that.
    #[must_use]
    pub fn strong_count(&self) -> usize {
        Arc::strong_count(&self.state)
    }

    /// Get the number of weak references to the inner value.
    ///
    /// Weak references are created using [`downgrade`][Self::downgrade] or by
    /// cloning an existing weak reference.
    #[must_use]
    pub fn weak_count(&self) -> usize {
        Arc::weak_count(&self.state)
    }

    /// Create a new [`WeakObservable`] reference to the same inner value.
    pub fn downgrade(&self) -> WeakObservable<T, L> {
        WeakObservable {
            state: Arc::downgrade(&self.state),
            _num_clones: Arc::downgrade(&self._num_clones),
        }
    }
}

impl<T, L: Lock> Clone for SharedObservable<T, L> {
    fn clone(&self) -> Self {
        Self {
            state: self.state.clone(),
            _num_clones: self._num_clones.clone(),
        }
    }
}

impl<T, L> Default for SharedObservable<T, L>
where
    T: Default,
    L: Lock,
{
    fn default() -> Self {
        let rwlock = L::new_rwlock(ObservableState::new(T::default()));
        Self::from_inner(Arc::new(rwlock))
    }
}

impl<T, L: Lock> Drop for SharedObservable<T, L> {
    fn drop(&mut self) {
        // Only close the state if there are no other clones of this
        // `SharedObservable`.
        if Arc::strong_count(&self._num_clones) == 1 {
            // If there are no other clones, obtaining a read lock can't fail.
            L::read_noblock(&self.state).close();
        }
    }
}
pub struct WeakObservable<T, L: Lock = SyncLock> {
    state: Weak<L::RwLock<ObservableState<T>>>,
    _num_clones: Weak<()>,
}

impl<T, L: Lock> WeakObservable<T, L> {
    /// Attempt to upgrade the `WeakObservable` into a `SharedObservable`.
    ///
    /// Returns `None` if the inner value has already been dropped.
    pub fn upgrade(&self) -> Option<SharedObservable<T, L>> {
        let state = Weak::upgrade(&self.state)?;
        let _num_clones = Weak::upgrade(&self._num_clones)?;
        Some(SharedObservable { state, _num_clones })
    }
}

#[derive(Debug, Deref)]
pub struct ObservableWriteGuard<'a, T: 'a, L: Lock = SyncLock> {
    inner: L::RwLockWriteGuard<'a, ObservableState<T>>,
}

impl<'a, T: 'a, L: Lock> ObservableWriteGuard<'a, T, L> {
    fn new(inner: L::RwLockWriteGuard<'a, ObservableState<T>>) -> Self {
        Self { inner }
    }
}

fn poison_error_map<T, U>(error: PoisonError<T>, f: impl FnOnce(T) -> U) -> PoisonError<U> {
    let inner = error.into_inner();
    PoisonError::new(f(inner))
}

fn try_lock_error_map<T, U>(error: TryLockError<T>, f: impl FnOnce(T) -> U) -> TryLockError<U> {
    match error {
        TryLockError::Poisoned(err) => TryLockError::Poisoned(poison_error_map(err, f)),
        TryLockError::WouldBlock => TryLockError::WouldBlock,
    }
}
