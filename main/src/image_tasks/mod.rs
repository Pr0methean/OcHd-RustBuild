use std::cmp::Ordering;
use std::fmt::{Debug, Formatter};
use std::hash::{Hash, Hasher};
use std::ops::{Deref, DerefMut};
use std::sync::Arc;
use lazy_static::lazy_static;
use lockfree_object_pool::{LinearObjectPool, LinearOwnedReusable};
use resvg::tiny_skia::{Color, Pixmap};
use crate::TILE_SIZE;

pub mod color;
pub mod repaint;
pub mod from_svg;
pub mod animate;
pub mod stack;
pub mod png_output;
pub mod task_spec;
pub mod make_semitransparent;

lazy_static! {
static ref PIXMAP_POOL: Arc<LinearObjectPool<Pixmap>> = Arc::new(LinearObjectPool::new(
    || Pixmap::new(*TILE_SIZE, *TILE_SIZE).expect("Failed to allocate a Pixmap for pool"),
    |_| {} // no reset needed if using allocate_pixmap_for_overwrite
));
}

pub enum MaybeFromPool<T> {
    FromPool {
        reusable: LinearOwnedReusable<T>,
    },
    NotFromPool(T)
}

impl <T> Deref for MaybeFromPool<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        match self {
            MaybeFromPool::FromPool { reusable, .. } => reusable.deref(),
            MaybeFromPool::NotFromPool(value) => value
        }
    }
}

impl <T> DerefMut for MaybeFromPool<T> where T: Clone {
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self {
            MaybeFromPool::FromPool { reusable, .. } => reusable.deref_mut(),
            MaybeFromPool::NotFromPool(value) => value
        }
    }
}

impl <T> Hash for MaybeFromPool<T> where T: Hash {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.deref().hash(state)
    }
}

impl <T> PartialEq for MaybeFromPool<T> where T: PartialEq {
    fn eq(&self, other: &Self) -> bool {
        self.deref().eq(other.deref())
    }
}

impl <T> Eq for MaybeFromPool<T> where T: Eq {}

impl <T> PartialOrd for MaybeFromPool<T> where T: PartialOrd {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.deref().partial_cmp(other.deref())
    }
}

impl <T> Ord for MaybeFromPool<T> where T: Ord {
    fn cmp(&self, other: &Self) -> Ordering {
        self.deref().cmp(other.deref())
    }
}

impl Clone for MaybeFromPool<Pixmap> {
    fn clone(&self) -> Self {
        let mut clone = allocate_pixmap_for_overwrite(self.width(), self.height());
        clone.data_mut().copy_from_slice(self.data());
        clone
    }
}

impl <T> Debug for MaybeFromPool<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            MaybeFromPool::NotFromPool {..} => "NotFromPool",
            MaybeFromPool::FromPool {..} => "FromPool"
        })
    }
}

pub fn allocate_pixmap_for_overwrite(width: u32, height: u32) -> MaybeFromPool<Pixmap> {
    if width == *TILE_SIZE && height == *TILE_SIZE {
        MaybeFromPool::FromPool { reusable: PIXMAP_POOL.pull_owned() }
    } else {
        MaybeFromPool::NotFromPool(Pixmap::new(width, height).unwrap())
    }
}

pub fn allocate_pixmap_empty(width: u32, height: u32) -> MaybeFromPool<Pixmap> {
    if width == *TILE_SIZE && height == *TILE_SIZE {
        let mut reusable = PIXMAP_POOL.pull_owned();
        reusable.fill(Color::TRANSPARENT);
        MaybeFromPool::FromPool { reusable }
    } else {
        MaybeFromPool::NotFromPool(Pixmap::new(width, height)
            .expect("Failed to allocate a Pixmap outside pool"))
    }
}
