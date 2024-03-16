use std::cmp::Ordering;
use std::fmt::{Debug, Formatter};
use std::hash::{Hash, Hasher};
use std::ops::{Deref, DerefMut};
use lockfree_object_pool::{LinearObjectPool, LinearReusable};
use log::info;
use once_cell::sync::Lazy;
use resvg::tiny_skia::{Color, IntSize, Pixmap};
use crate::{GRID_SIZE, TILE_SIZE};

pub mod color;
pub mod repaint;
pub mod from_svg;
pub mod animate;
pub mod stack;
pub mod png_output;
pub mod task_spec;
pub mod make_semitransparent;
pub mod upscale;
pub(crate) mod cloneable;

fn new_uninit_pixmap(width: u32, height: u32) -> Pixmap {
    let data_len = 4 * (width as usize) * (height as usize);
    let mut data = Vec::with_capacity(data_len);
    unsafe {
        data.set_len(data_len);
    }
    Pixmap::from_vec(data, IntSize::from_wh(width, height).unwrap())
        .unwrap_or_else(|| panic!("Failed to allocate a {}x{} pixmap", width, height))
}

static TILE_SIZE_PIXMAP_POOL: Lazy<LinearObjectPool<Pixmap>> = Lazy::new(|| LinearObjectPool::new(
        || {
            info!("Allocating a tile-size Pixmap for pool");
            new_uninit_pixmap(*TILE_SIZE, *TILE_SIZE)
        },
        |_| {} // no reset needed if using allocate_pixmap_for_overwrite
    )
);
static GRID_SIZE_PIXMAP_POOL: Lazy<LinearObjectPool<Pixmap>> = Lazy::new(|| LinearObjectPool::new(
    || {
        info!("Allocating a grid-size Pixmap for pool");
        new_uninit_pixmap(*GRID_SIZE, *GRID_SIZE)
    },
    |_| {} // no reset needed if using allocate_pixmap_for_overwrite
));

pub fn prewarm_pixmap_pool() {
    GRID_SIZE_PIXMAP_POOL.pull();
    if GRID_SIZE != *TILE_SIZE {
        TILE_SIZE_PIXMAP_POOL.pull();
    }
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
    if width == GRID_SIZE && height == GRID_SIZE {
        info!("Borrowing a grid-size Pixmap from pool");
        MaybeFromPool::FromPool { reusable: GRID_SIZE_PIXMAP_POOL.pull() }
    } else if width == *TILE_SIZE && height == *TILE_SIZE {
        info!("Borrowing a tile-size Pixmap from pool");
        MaybeFromPool::FromPool { reusable: TILE_SIZE_PIXMAP_POOL.pull() }
    } else {
        info!("Allocating a Pixmap outside pool (not required empty) for unusual size {}x{}",
            width, height);
        MaybeFromPool::NotFromPool(new_uninit_pixmap(width, height))
    }
}

pub fn allocate_pixmap_empty(width: u32, height: u32) -> MaybeFromPool<Pixmap> {
    if width == GRID_SIZE && height == GRID_SIZE {
        info!("Borrowing and clearing a grid-size Pixmap from pool");
        let mut reusable = GRID_SIZE_PIXMAP_POOL.pull();
        reusable.fill(Color::TRANSPARENT);
        MaybeFromPool::FromPool { reusable }
    } else if width == *TILE_SIZE && height == *TILE_SIZE {
        info!("Borrowing and clearing a tile-size Pixmap from pool");
        let mut reusable = TILE_SIZE_PIXMAP_POOL.pull();
        reusable.fill(Color::TRANSPARENT);
        MaybeFromPool::FromPool { reusable }
    } else {
        info!("Allocating a Pixmap outside pool (required empty) for unusual size {}x{}",
            width, height);
        MaybeFromPool::NotFromPool(Pixmap::new(width, height)
            .expect("Failed to allocate a Pixmap outside pool"))
    }
}
