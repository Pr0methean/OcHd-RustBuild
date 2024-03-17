use crate::anyhoo;
use log::info;
use replace_with::replace_with_and_return;
use std::cmp::Ordering;
use std::fmt::{Debug, Display, Formatter};
use std::hash::{Hash, Hasher};
use std::ops::{Deref, DerefMut};
use std::sync::{Arc, Mutex};

pub type CloneableResult<T> = Result<Arc<T>, CloneableError>;

#[derive(Clone, Debug, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub struct CloneableError {
    message: Arcow<'static, str>,
}

impl<T> From<T> for CloneableError
where
    T: ToString,
{
    fn from(value: T) -> Self {
        CloneableError {
            message: value.to_string().into(),
        }
    }
}

#[derive(Debug, Eq, Ord)]
pub enum Arcow<'a, T: ?Sized> {
    Owned(Arc<T>),
    Borrowed(&'a T),
}

impl<'a, T: ?Sized> Clone for Arcow<'a, T> {
    fn clone(&self) -> Self {
        match self {
            Arcow::Owned(arc) => Arcow::Owned(arc.clone()),
            Arcow::Borrowed(borrow) => Arcow::Borrowed(borrow),
        }
    }
}

impl<'a, T: ?Sized> Deref for Arcow<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        match self {
            Arcow::Owned(arc) => arc,
            Arcow::Borrowed(borrow) => borrow,
        }
    }
}

impl<'a, T: Display + ?Sized> Display for Arcow<'a, T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.deref().to_string())
    }
}

impl<'a, T: Eq + ?Sized> PartialEq for Arcow<'a, T> {
    fn eq(&self, other: &Self) -> bool {
        self.deref() == other.deref()
    }
}

impl<'a, T: Eq + Ord + ?Sized> PartialOrd for Arcow<'a, T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.deref().cmp(other.deref()))
    }
}

impl<'a, T: Hash + ?Sized> Hash for Arcow<'a, T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.deref().hash(state)
    }
}

impl<'a, T: ?Sized> From<&'a T> for Arcow<'a, T> {
    fn from(value: &'a T) -> Self {
        Arcow::Borrowed(value)
    }
}

impl<'a, T> From<T> for Arcow<'a, T> {
    fn from(value: T) -> Self {
        Arcow::Owned(value.into())
    }
}

impl<'a> From<String> for Arcow<'a, str> {
    fn from(value: String) -> Self {
        Arcow::<'a, str>::Owned(value.into())
    }
}

impl<'a, T> From<Vec<T>> for Arcow<'a, [T]> {
    fn from(value: Vec<T>) -> Self {
        Arcow::Owned(value.into_boxed_slice().into())
    }
}

pub type LazyTaskFunction<T> = Box<dyn FnOnce() -> Result<Box<T>, CloneableError> + Send>;

pub enum CloneableLazyTaskState<T>
where
    T: ?Sized,
{
    Upcoming { function: LazyTaskFunction<T> },
    Finished { result: CloneableResult<T> },
}

#[derive(Clone, Debug)]
pub struct CloneableLazyTask<T>
where
    T: ?Sized,
{
    pub name: Arcow<'static, str>,
    state: Arc<Mutex<CloneableLazyTaskState<T>>>,
}

impl<T> Display for CloneableLazyTask<T>
where
    T: ?Sized,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.name)
    }
}

impl<T> Debug for CloneableLazyTaskState<T>
where
    T: ?Sized,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            CloneableLazyTaskState::Upcoming { .. } => f.write_str("Upcoming"),
            CloneableLazyTaskState::Finished { result } => match result {
                Ok(..) => f.write_str("Ok"),
                Err(error) => f.write_fmt(format_args!("Error({})", error.message)),
            },
        }
    }
}

impl<T> CloneableLazyTask<T>
where
    T: ?Sized,
{
    pub fn new<U>(name: U, base: LazyTaskFunction<T>) -> CloneableLazyTask<T>
    where
        U: Into<Arcow<'static, str>>,
    {
        CloneableLazyTask {
            name: name.into(),
            state: Arc::new(Mutex::new(CloneableLazyTaskState::Upcoming {
                function: base,
            })),
        }
    }
}

impl<T> CloneableLazyTask<T> {
    pub fn new_immediate_ok<U>(name: U, result: T) -> CloneableLazyTask<T>
    where
        U: Into<Arcow<'static, str>>,
    {
        CloneableLazyTask {
            name: name.into(),
            state: Arc::new(Mutex::new(CloneableLazyTaskState::Finished {
                result: Ok(Arc::new(result)),
            })),
        }
    }

    /// Consumes this particular copy of the task and returns the result. Trades off readability and
    /// maintainability to maximize the chance of avoiding unnecessary copies.
    pub fn into_result(self) -> CloneableResult<T> {
        match Arc::try_unwrap(self.state) {
            Ok(exclusive_state) => {
                // We're the last referent to this Lazy, so we don't need to clone anything.
                match exclusive_state.into_inner() {
                    Ok(state) => match state {
                        CloneableLazyTaskState::Upcoming { function } => {
                            info!("Starting task {}", self.name);
                            let result: CloneableResult<T> = function().map(Arc::from);
                            info!("Finished task {}", self.name);
                            info!("Unwrapping the only reference to {}", self.name);
                            result
                        }
                        CloneableLazyTaskState::Finished { result } => {
                            info!("Unwrapping the last reference to {}", self.name);
                            result
                        }
                    },
                    Err(e) => Err(e.into()),
                }
            }
            Err(shared_state) => match shared_state.lock() {
                Ok(mut locked_state) => replace_with_and_return(
                    locked_state.deref_mut(),
                    || CloneableLazyTaskState::Finished {
                        result: Err(anyhoo!("replace_with_and_return_failed")),
                    },
                    |exec_state| match exec_state {
                        CloneableLazyTaskState::Upcoming { function } => {
                            info!("Starting task {}", self.name);
                            let result: CloneableResult<T> = function().map(Arc::from);
                            info!("Finished task {}", self.name);
                            info!(
                                "Unwrapping one of {} references to {} after computing it",
                                Arc::strong_count(&shared_state),
                                self.name
                            );
                            (
                                result.to_owned(),
                                CloneableLazyTaskState::Finished { result },
                            )
                        }
                        CloneableLazyTaskState::Finished { result } => {
                            info!(
                                "Unwrapping one of {} references to {}",
                                Arc::strong_count(&shared_state),
                                self.name
                            );
                            (
                                result.to_owned(),
                                CloneableLazyTaskState::Finished { result },
                            )
                        }
                    },
                ),
                Err(e) => Err(e.into()),
            },
        }
    }
}
