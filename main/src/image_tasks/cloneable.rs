use std::borrow::Borrow;
use crate::anyhoo;
use log::info;
use replace_with::replace_with_and_return;
use std::cmp::Ordering;
use std::fmt::{Debug, Display, Formatter};
use std::hash::{Hash, Hasher};
use std::ops::{Deref, DerefMut};
use std::sync::{Arc, Mutex};

#[derive(Clone, Debug, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub struct CloneableError {
    message: Name,
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

#[derive(Debug)]
pub enum Arcow<'a, UnsizedType: ?Sized, SizedType: Clone + 'a>
where SizedType: Borrow<UnsizedType>{
    SharingRef(Arc<UnsizedType>),
    Cloning(SizedType),
    Borrowing(&'a UnsizedType),
}

pub type CloneableResult<UnsizedType, SizedType> = Result<Arcow<'static, UnsizedType, SizedType>, CloneableError>;
pub type Name = Arcow<'static, str, String>;
pub type SimpleArcow<T> = Arcow<'static, T, T>;

impl<'a, UnsizedType: ?Sized, SizedType: Clone> Clone for Arcow<'a, UnsizedType, SizedType>
where SizedType: Borrow<UnsizedType>{
    fn clone(&self) -> Self {
        match self {
            Arcow::SharingRef(arc) => Arcow::SharingRef(arc.clone()),
            Arcow::Borrowing(borrow) => Arcow::Borrowing(borrow),
            Arcow::Cloning(value) => Arcow::Cloning(value.clone())
        }
    }
}

impl<'a, UnsizedType: ?Sized, SizedType: Clone> Deref for Arcow<'a, UnsizedType, SizedType>
    where SizedType: Borrow<UnsizedType>{
    type Target = UnsizedType;

    fn deref(&self) -> &Self::Target {
        match self {
            Arcow::SharingRef(arc) => arc,
            Arcow::Borrowing(borrow) => borrow,
            Arcow::Cloning(value) => (*value).borrow()
        }
    }
}

impl<UnsizedType: ?Sized, SizedType: Clone> Display for Arcow<'_, UnsizedType, SizedType>
    where SizedType: Borrow<UnsizedType>, for<'a> &'a UnsizedType: Display {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.deref().to_string())
    }
}

impl<UnsizedType: ?Sized, SizedType: Clone> PartialEq for Arcow<'_, UnsizedType, SizedType>
    where SizedType: Borrow<UnsizedType>, for<'a> &'a UnsizedType: PartialEq {
    fn eq(&self, other: &Self) -> bool {
        if let Arcow::SharingRef(arc) = self && let Arcow::SharingRef(other_arc) = other
                && Arc::ptr_eq(arc, other_arc) {
            return true;
        }
        self.deref() == other.deref()
    }
}

impl<'a, UnsizedType: ?Sized + Eq, SizedType: Clone> Eq for Arcow<'a, UnsizedType, SizedType>
    where SizedType: Borrow<UnsizedType>{}

impl<'a, UnsizedType: ?Sized + PartialOrd, SizedType: Clone> PartialOrd for Arcow<'a, UnsizedType, SizedType>
    where SizedType: Borrow<UnsizedType>{
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if let Arcow::SharingRef(arc) = self && let Arcow::SharingRef(other_arc) = other
                && Arc::ptr_eq(arc, other_arc) {
            return Some(Ordering::Equal);
        }
        self.deref().partial_cmp(other.deref())
    }
}

impl<'a, UnsizedType: ?Sized + PartialEq + Ord, SizedType: Clone> Ord for Arcow<'a, UnsizedType, SizedType>
    where SizedType: Borrow<UnsizedType>{
    fn cmp(&self, other: &Self) -> Ordering {
        self.partial_cmp(other).unwrap()
    }
}

impl<UnsizedType: ?Sized, SizedType: Clone> Hash for Arcow<'_, UnsizedType, SizedType>
    where SizedType: Borrow<UnsizedType>, for<'a> &'a UnsizedType: Hash {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.deref().hash(state)
    }
}

impl From<String> for Name {
    fn from(value: String) -> Self {
        Arcow::SharingRef(value.into())
    }
}

impl From<&'static str> for Name {
    fn from(value: &'static str) -> Self {
        Arcow::Borrowing(value)
    }
}

impl<'a, UnsizedType: ?Sized, SizedType: Clone> Arcow<'a, UnsizedType, SizedType>
    where SizedType: Borrow<UnsizedType>, Arc<UnsizedType>: From<SizedType> {
    pub fn borrowing_from(value: &'a UnsizedType) -> Self {
        Arcow::Borrowing(value)
    }

    pub fn sharing_ref_to(value: SizedType) -> Self {
        Arcow::SharingRef(value.into())
    }
}

impl<'a, UnsizedType: ?Sized, SizedType: Clone> Arcow<'a, UnsizedType, SizedType>
    where SizedType: Borrow<UnsizedType> {
    pub fn cloning_from(value: &SizedType) -> Self {
        Arcow::Cloning(value.clone())
    }
}

impl<'a, UnsizedType, SizedType: Clone> Arcow<'a, UnsizedType, SizedType>
    where SizedType: Borrow<UnsizedType> + From<UnsizedType>, UnsizedType: Clone + From<SizedType> {

    pub fn consume<R, T: FnOnce(UnsizedType) -> R>(self, action: T) -> R {
        match self {
            Arcow::SharingRef(arc) => action(Arc::unwrap_or_clone(arc)),
            Arcow::Cloning(value) => action(value.into()),
            Arcow::Borrowing(borrow) => action(borrow.clone())
        }
    }
}

pub type LazyTaskFunction<UnsizedType, SizedType>
    = Box<dyn FnOnce() -> Result<Arcow<'static, UnsizedType, SizedType>, CloneableError> + Send>;

pub enum CloneableLazyTaskState<UnsizedType: ?Sized + 'static, SizedType: Clone + 'static>
    where SizedType: Borrow<UnsizedType>
{
    Upcoming { function: LazyTaskFunction<UnsizedType, SizedType> },
    Finished { result: CloneableResult<UnsizedType, SizedType> },
}

#[derive(Clone, Debug)]
pub struct CloneableLazyTask<UnsizedType: ?Sized + 'static, SizedType: Clone + 'static>
    where SizedType: Borrow<UnsizedType>
{
    pub name: Name,
    state: Arc<Mutex<CloneableLazyTaskState<UnsizedType, SizedType>>>,
}

impl<UnsizedType: ?Sized, SizedType: Clone + 'static> Display for CloneableLazyTask<UnsizedType, SizedType>
    where SizedType: Borrow<UnsizedType>
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.name)
    }
}

impl<UnsizedType: ?Sized, SizedType: Clone + 'static> Debug for CloneableLazyTaskState<UnsizedType, SizedType>
    where SizedType: Borrow<UnsizedType>
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

impl<UnsizedType: ?Sized, SizedType: Clone + 'static> CloneableLazyTask<UnsizedType, SizedType>
    where SizedType: Borrow<UnsizedType>
{
    pub fn new<U>(name: U, base: LazyTaskFunction<UnsizedType, SizedType>) -> Self
    where
        U: Into<Name>,
    {
        CloneableLazyTask {
            name: name.into(),
            state: Arc::new(Mutex::new(CloneableLazyTaskState::Upcoming {
                function: base,
            })),
        }
    }

    pub fn new_immediate_ok<IntoName: Into<Name>>
        (name: IntoName, result: Arcow<'static, UnsizedType, SizedType>) -> Self
    {
        CloneableLazyTask {
            name: name.into(),
            state: Arc::new(Mutex::new(CloneableLazyTaskState::Finished {
                result: Ok(result),
            })),
        }
    }

    /// Consumes this particular copy of the task and returns the result. Trades off readability and
    /// maintainability to maximize the chance of avoiding unnecessary copies.
    pub fn into_result(self) -> CloneableResult<UnsizedType, SizedType> {
        match Arc::try_unwrap(self.state) {
            Ok(exclusive_state) => {
                // We're the last referent to this Lazy, so we don't need to clone anything.
                match exclusive_state.into_inner() {
                    Ok(state) => match state {
                        CloneableLazyTaskState::Upcoming { function } => {
                            info!("Starting task {}", self.name);
                            let result = function();
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
                            let result = function().map(Arcow::from);
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
