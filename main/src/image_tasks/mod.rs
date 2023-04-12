
use std::fmt::{Debug, Formatter};
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};
use std::sync::{Arc};

use lazy_static::lazy_static;
use lockfree_object_pool::{LinearObjectPool, LinearOwnedReusable};
use tiny_skia::{Color, Pixmap};

use crate::image_tasks::MaybeFromPool::{FromPool, NotFromPool};


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
    || Pixmap::new(*TILE_SIZE, *TILE_SIZE).unwrap(),
    |pixmap| pixmap.fill(Color::TRANSPARENT)
));
}

pub enum MaybeFromPool<'a, T> {
    FromPool {
        reusable: LinearOwnedReusable<T>,
        phantom: &'a PhantomData<T>
    },
    NotFromPool(T)
}

impl <'a, T> Deref for MaybeFromPool<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        match self {
            FromPool { reusable, .. } => reusable.deref(),
            NotFromPool(value) => value
        }
    }
}

impl <'a, T> DerefMut for MaybeFromPool<'a, T> where T: Clone {
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self {
            FromPool { reusable, .. } => reusable.deref_mut(),
            NotFromPool(value) => value
        }
    }
}

impl <'a> Clone for MaybeFromPool<'a, Pixmap> {
    fn clone(&self) -> Self {
        let mut pixmap = allocate_pixmap(self.width(), self.height());
        self.deref().clone_into(pixmap.deref_mut());
        pixmap
    }
}

impl <'a, T> Debug for MaybeFromPool<'a, T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            NotFromPool {..} => "NotFromPool",
            FromPool {..} => "FromPool"
        })
    }
}

lazy_static! {
    static ref PIXMAP_PHANTOM: PhantomData<Pixmap> = PhantomData::default();
}

pub fn allocate_pixmap<'a>(width: u32, height: u32) -> MaybeFromPool<'a, Pixmap> {
    if width == *TILE_SIZE && height == *TILE_SIZE {
        FromPool { reusable: PIXMAP_POOL.pull_owned(), phantom: &PIXMAP_PHANTOM }
    } else {
        NotFromPool(Pixmap::new(width, height).unwrap())
    }
}
