use crate::{
    lock::{Lock, SyncLock},
    shared::{Shared, SharedObservable},
    state::ObservableState,
    subscriber::Subscriber,
};
use std::{hash::Hash, mem, ptr};

pub struct Observable<T, L: Lock = SyncLock> {
    state: L::Shared<ObservableState<T>>,
}

impl<T> Observable<T> {
    #[must_use]
    pub fn new(value: T) -> Self {
        let state = Shared::new(ObservableState::new(value));
        Self::from_inner(state)
    }

    /// Obtain a new subscriber.
    ///
    /// Calling `.next().await` or `.next_ref().await` on the returned
    /// subscriber only resolves once the inner value has been updated again
    /// after the call to `subscribe`.
    ///
    /// See [`subscribe_reset`][Self::subscribe_reset] if you want to obtain a
    /// subscriber that immediately yields without any updates.
    pub fn subscribe(this: &Self) -> Subscriber<T> {
        Subscriber::new(Shared::get_read_lock(&this.state), this.state.version())
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
    pub fn subscribe_reset(this: &Self) -> Subscriber<T> {
        Subscriber::new(Shared::get_read_lock(&this.state), 0)
    }

    /// Get a reference to the inner value.
    ///
    /// Usually, you don't need to call this function since `Observable<T>`
    /// implements `Deref`. Use this if you want to pass the inner value to a
    /// generic function where the compiler can't infer that you want to have
    /// the `Observable` dereferenced otherwise.
    pub fn get(this: &Self) -> &T {
        this.state.get()
    }

    /// Set the inner value to the given `value`, notify subscribers and return
    /// the previous value.
    pub fn set(this: &mut Self, value: T) -> T {
        Shared::lock(&mut this.state).set(value)
    }

    /// Set the inner value to the given `value` if it doesn't compare equal to
    /// the existing value.
    ///
    /// If the inner value is set, subscribers are notified and
    /// `Some(previous_value)` is returned. Otherwise, `None` is returned.
    pub fn set_if_not_eq(this: &mut Self, value: T) -> Option<T>
    where
        T: PartialEq,
    {
        Shared::lock(&mut this.state).set_if_not_eq(value)
    }

    /// Set the inner value to the given `value` if it has a different hash than
    /// the existing value.
    ///
    /// If the inner value is set, subscribers are notified and
    /// `Some(previous_value)` is returned. Otherwise, `None` is returned.
    pub fn set_if_hash_not_eq(this: &mut Self, value: T) -> Option<T>
    where
        T: Hash,
    {
        Shared::lock(&mut this.state).set_if_hash_not_eq(value)
    }

    /// Set the inner value to a `Default` instance of its type, notify
    /// subscribers and return the previous value.
    ///
    /// Shorthand for `Observable::set(this, T::default())`.
    pub fn take(this: &mut Self) -> T
    where
        T: Default,
    {
        Self::set(this, T::default())
    }

    /// Update the inner value and notify subscribers.
    ///
    /// Note that even if the inner value is not actually changed by the
    /// closure, subscribers will be notified as if it was. Use
    /// [`update_if`][Self::update_if] if you want to conditionally mutate the
    /// inner value.
    pub fn update(this: &mut Self, f: impl FnOnce(&mut T)) {
        Shared::lock(&mut this.state).update(f);
    }

    /// Maybe update the inner value and notify subscribers if it changed.
    ///
    /// The closure given to this function must return `true` if subscribers
    /// should be notified of a change to the inner value.
    pub fn update_if(this: &mut Self, f: impl FnOnce(&mut T) -> bool) {
        Shared::lock(&mut this.state).update_if(f);
    }
}

impl<T, L: Lock> Observable<T, L> {
    pub(crate) fn from_inner(state: L::Shared<ObservableState<T>>) -> Self {
        Self { state }
    }

    /// Get the number of subscribers.
    ///
    /// Be careful when using this. The result is only reliable if it is exactly
    /// `0`, as otherwise it could be incremented right after your call to this
    /// function, before you look at its result or do anything based on that.
    #[must_use]
    pub fn subscriber_count(this: &Self) -> usize {
        L::shared_read_count(&this.state)
    }

    /// Convert this unique `Observable` into a [`SharedObservable`].
    ///
    /// Any subscribers created for `self` remain valid.
    pub fn into_shared(this: Self) -> SharedObservable<T, L> {
        // Destructure `this` without running `Drop`.
        let state = unsafe { ptr::read(&this.state) };
        mem::forget(this);

        let rwlock = L::shared_into_inner(state);
        SharedObservable::from_inner(rwlock)
    }
}

impl<T, L> Default for Observable<T, L>
where
    T: Default,
    L: Lock,
{
    fn default() -> Self {
        let shared = L::new_shared(ObservableState::new(T::default()));
        Self::from_inner(shared)
    }
}

// // Note: No DerefMut because all mutating must go through inherent methods that
// // notify subscribers
// impl<T> ops::Deref for Observable<T> {
//     type Target = T;

//     fn deref(&self) -> &Self::Target {
//         self.state.get()
//     }
// }

impl<T, L: Lock> Drop for Observable<T, L> {
    fn drop(&mut self) {
        self.state.close();
    }
}
