use std::cmp::Ordering;
use std::fmt::{Debug, Formatter};
use std::hash::{Hash, Hasher};
use std::ops::{Deref, DerefMut};
use lazy_static::lazy_static;
use lockfree_object_pool::{LinearObjectPool, LinearReusable};
use log::info;
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
pub mod upscale;

lazy_static! {
    static ref PIXMAP_POOL: LinearObjectPool<Pixmap> = LinearObjectPool::new(
        || {
            info!("Allocating a Pixmap for pool");
            Pixmap::new(*TILE_SIZE, *TILE_SIZE).expect("Failed to allocate a Pixmap for pool")
        },
        |_| {} // no reset needed if using allocate_pixmap_for_overwrite
    );
}

pub fn prewarm_pixmap_pool() {
    PIXMAP_POOL.pull();
}

pub enum MaybeFromPool<T: 'static> {
    FromPool {
        reusable: LinearReusable<'static, T>,
    },
    NotFromPool(T)
}

impl <T> MaybeFromPool<T> where T: Clone {
    pub fn unwrap_or_clone(self) -> T {
        match self {
            MaybeFromPool::FromPool { reusable } => reusable.deref().to_owned(),
            MaybeFromPool::NotFromPool(inner) => inner
        }
    }
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
        let width = self.width();
        let height = self.height();
        info!("Copying a Pixmap of size {}x{}", width, height);
        let mut clone = allocate_pixmap_for_overwrite(width, height);
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
        info!("Borrowing a Pixmap from pool");
        MaybeFromPool::FromPool { reusable: PIXMAP_POOL.pull() }
    } else {
        info!("Allocating a Pixmap outside pool (not required empty) for unusual size {}x{}",
            width, height);
        MaybeFromPool::NotFromPool(Pixmap::new(width, height).unwrap())
    }
}

pub fn allocate_pixmap_empty(width: u32, height: u32) -> MaybeFromPool<Pixmap> {
    if width == *TILE_SIZE && height == *TILE_SIZE {
        info!("Borrowing and clearing a Pixmap from pool");
        let mut reusable = PIXMAP_POOL.pull();
        reusable.fill(Color::TRANSPARENT);
        MaybeFromPool::FromPool { reusable }
    } else {
        info!("Allocating a Pixmap outside pool (required empty) for unusual size {}x{}",
            width, height);
        MaybeFromPool::NotFromPool(Pixmap::new(width, height)
            .expect("Failed to allocate a Pixmap outside pool"))
    }
}
