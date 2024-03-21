use std::borrow::{Borrow, BorrowMut};
use std::cmp::Ordering;
use std::fmt::{Debug, Display, Formatter};
use std::hash::{Hash, Hasher};
use std::mem::{replace, size_of, size_of_val};
use std::ops::{Deref, DerefMut};
use std::sync::{Arc};

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

impl<'a, UnsizedType: ?Sized + Clone, SizedType: Clone> DerefMut for Arcow<'a, UnsizedType, SizedType>
    where SizedType: BorrowMut<UnsizedType>{
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self {
            Arcow::Cloning(value) => {
                return (*value).borrow_mut();
            }
            Arcow::Borrowing(borrow) => {
                let arc = Arc::new((*borrow).clone());
                let _ = replace(self, Arcow::SharingRef(arc));
                return self.deref_mut();
            }
            Arcow::SharingRef(arc) => Arc::make_mut(arc)
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
    where SizedType: Borrow<UnsizedType> {
    pub fn cloning_from(value: SizedType) -> Self {
        Arcow::Cloning(value)
    }
}

impl<'a, UnsizedType: ?Sized, SizedType: Clone> Arcow<'a, UnsizedType, SizedType>
    where SizedType: Borrow<UnsizedType>, Arc<UnsizedType>: From<SizedType> {
    pub fn borrowing_from(value: &'a UnsizedType) -> Self {
        Arcow::Borrowing(value)
    }

    // The multiplier of 8 is based on the size of CPU cache lines, but Clippy thinks we're
    // converting a size from bytes to bits.
    #[allow(clippy::manual_bits)]
    const ARC_THRESHOLD: usize = 8 * size_of::<usize>();

    pub fn sharing_ref(value: SizedType) -> Self {
        Arcow::SharingRef(value.into())
    }

    pub fn from_owned(value: SizedType) -> Self {
        if size_of_val(&value) > Self::ARC_THRESHOLD {
            Self::sharing_ref(value)
        } else {
            Self::cloning_from(value)
        }
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
