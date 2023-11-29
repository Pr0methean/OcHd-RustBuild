use std::sync::{Arc, Mutex};
use std::fmt::{Debug, Display, Formatter};
use log::info;
use replace_with::{replace_with_and_return};
use std::ops::{Deref, DerefMut};
use crate::{anyhoo};

pub type CloneableResult<T> = Result<Arc<T>, CloneableError>;

#[derive(Clone, Debug, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub struct CloneableError {
    message: Arc<str>
}

impl <T> From<T> for CloneableError where T: ToString {
    fn from(value: T) -> Self {
        CloneableError {message: value.to_string().into()}
    }
}

#[derive(Clone)]
pub enum ArcowStr<'a> {
    Owned(Arc<str>),
    Borrowed(&'a str)
}

impl <'a> Deref for ArcowStr<'a> {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        match self {
            ArcowStr::Owned(arc) => &*arc,
            ArcowStr::Borrowed(borrow) => borrow
        }
    }
}

impl <'a> Debug for ArcowStr<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.deref())
    }
}


impl <'a> Display for ArcowStr<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.deref())
    }
}

impl <'a> From<&'a str> for ArcowStr<'a> {
    fn from(value: &'a str) -> Self {
        ArcowStr::Borrowed(value)
    }
}

impl <'a> From<&'a ArcowStr<'a>> for ArcowStr<'a> {
    fn from(value: &'a ArcowStr<'a>) -> Self {
        value.clone()
    }
}

impl <'a> From<String> for ArcowStr<'a> {
    fn from(value: String) -> Self {
        ArcowStr::Owned(value.into())
    }
}

pub type LazyTaskFunction<T> = Box<dyn FnOnce() -> Result<Box<T>, CloneableError> + Send>;

pub enum CloneableLazyTaskState<T> where T: ?Sized {
    Upcoming {
        function: LazyTaskFunction<T>,
    },
    Finished {
        result: CloneableResult<T>
    }
}

#[derive(Clone,Debug)]
pub struct CloneableLazyTask<T> where T: ?Sized {
    pub name: ArcowStr<'static>,
    state: Arc<Mutex<CloneableLazyTaskState<T>>>
}

impl <T> Display for CloneableLazyTask<T> where T: ?Sized {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.name)
    }
}

impl <T> Debug for CloneableLazyTaskState<T> where T: ?Sized {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            CloneableLazyTaskState::Upcoming { .. } => {
                f.write_str("Upcoming")
            },
            CloneableLazyTaskState::Finished { result } => {
                match result {
                    Ok(..) => f.write_str("Ok"),
                    Err(error) => f.write_fmt(
                        format_args!("Error({})", error.message))
                }
            }
        }
    }
}

impl <T> CloneableLazyTask<T> where T: ?Sized {
    pub fn new<U>(name: U, base: LazyTaskFunction<T>) -> CloneableLazyTask<T>
        where U: Into<ArcowStr<'static>> {
        CloneableLazyTask {
            name: name.into(),
            state: Arc::new(Mutex::new(CloneableLazyTaskState::Upcoming {
                function: base
            }))
        }
    }
}

impl <T> CloneableLazyTask<T> {
    pub fn new_immediate_ok<U>(name: U, result: T) -> CloneableLazyTask<T>
        where U: Into<ArcowStr<'static>> {
        CloneableLazyTask {
            name: name.into(),
            state: Arc::new(Mutex::new(CloneableLazyTaskState::Finished {
                result: Ok(Arc::new(result))
            }))
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
                        },
                        CloneableLazyTaskState::Finished { result } => {
                            info!("Unwrapping the last reference to {}", self.name);
                            result
                        },
                    }
                    Err(e) => Err(e.into())
                }
            }
            Err(shared_state) => {
                match shared_state.lock() {
                    Ok(mut locked_state) => {
                        replace_with_and_return(
                            locked_state.deref_mut(),
                            || CloneableLazyTaskState::Finished {
                                result: Err(anyhoo!("replace_with_and_return_failed"))
                            },
                            |exec_state| {
                                match exec_state {
                                    CloneableLazyTaskState::Upcoming { function} => {
                                        info! ("Starting task {}", self.name);
                                        let result: CloneableResult<T> = function().map(Arc::from);
                                        info! ("Finished task {}", self.name);
                                        info!("Unwrapping one of {} references to {} after computing it",
                                            Arc::strong_count(&shared_state), self.name);
                                        (result.to_owned(), CloneableLazyTaskState::Finished { result })
                                    },
                                    CloneableLazyTaskState::Finished { result } => {
                                        info!("Unwrapping one of {} references to {}",
                                            Arc::strong_count(&shared_state), self.name);
                                        (result.to_owned(), CloneableLazyTaskState::Finished { result })
                                    },
                                }
                            }
                        )
                    }
                    Err(e) => Err(e.into())
                }
            }
        }
    }
}
