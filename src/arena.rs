#![allow(dead_code)]

use core::str;
use std::{alloc::{alloc, dealloc, Layout}, cell::Cell, fmt::Debug, marker::PhantomData, ops::{Deref, DerefMut}, ptr::{self, copy_nonoverlapping}, slice::{from_raw_parts, from_raw_parts_mut}};

#[inline]
fn array<T>(n: usize) -> Layout {
    Layout::array::<T>(n).unwrap()
}

// let the user of this function enforce the lifetime of the &str returned
#[inline]
unsafe fn ptr_to_string_mut(ptr: *mut u8, len: usize) -> &'static mut str {
    unsafe { str::from_utf8_unchecked_mut(from_raw_parts_mut(ptr, len)) }
}

pub struct Arena<'a> {
    memory: *mut u8,
    top: Cell<usize>, // holds the ptr to the top element. Doesn't need to be derefed so used `usize`
    n: Cell<usize>,
    layout: Layout,
    marker: PhantomData<&'a u8>,
}

impl<'a> Arena<'a> {
    pub fn new(size: usize) -> Self {
        let layout = array::<u8>(size);
        let memory = unsafe { alloc(layout) };
        Arena { 
            memory,
            top: Cell::new(memory as _),
            n: Cell::new(0),
            layout,
            marker: PhantomData,
        }
    }

    #[inline]
    fn advance_by(&self, n: usize) {
        self.n.set(self.n.get() + n);
    }

    unsafe fn alloc_bytes(&self, n: usize) -> *mut u8 {
        if self.n.get() + n > self.layout.size() {
            panic!("Memoryyyyy");
        }
    
        let ptr = unsafe { self.memory.add(self.n.get()) };
        self.advance_by(n);
        self.top.set(ptr as usize);
        ptr
    }
    
    pub fn alloc<T>(&self, item: T) -> &mut T {
        unsafe { 
            let ptr = self.alloc_bytes(size_of::<T>()) as *mut T;
            *ptr = item;
            &mut *ptr
        }
    }

    pub fn alloc_str(&self, str: &str) -> &mut str {
        unsafe { 
            let ptr = self.alloc_bytes(str.len());
            copy_nonoverlapping(str.as_ptr(), ptr, str.len());
            ptr_to_string_mut(ptr, str.len()) 
        }
    }

    unsafe fn realloc<T>(&self, ptr: *const T, old_size: usize, new_size: usize) -> *mut T {
        let old_size = old_size * size_of::<T>();
        let new_size = new_size * size_of::<T>();

        fn inner(me: &Arena, ptr: usize, old_size: usize, new_size: usize) -> usize {
            if me.top.get() == ptr {
                me.advance_by(new_size - old_size);
                return ptr;
            }

            unsafe {
                let new_ptr = me.alloc_bytes(new_size);
                copy_nonoverlapping(ptr as _, new_ptr, old_size);
                me.advance_by(new_size);
                new_ptr as _
            }
        }

        inner(&self, ptr as _, old_size, new_size) as _
    }

    pub fn reset(self) -> Self {
        unsafe { self.memory.write_bytes(0, self.layout.size()); }
        self
    }

    pub fn dump(&self) {
        println!("{:?}", unsafe { from_raw_parts(self.memory, self.layout.size()) });
    }
}

impl Drop for Arena<'_> {
    fn drop(&mut self) {
        unsafe { dealloc(self.memory, self.layout ); }
    }
}

pub struct ArenaVec<'a, T> {
    mem: *mut T,
    len: usize,
    cap: usize,
    arena: &'a Arena<'a> 
}

impl<'a, T> ArenaVec<'a, T> {
    pub fn new(arena: &'a Arena) -> Self {
        let ptr = unsafe { arena.alloc_bytes(size_of::<T>()) };

        ArenaVec {
            mem: ptr as *mut T,
            len: 0,
            cap: 1,
            arena,
        }
    }

    pub fn push(&mut self, item: T) {
        if self.len + 1 > self.cap {
            unsafe { 
                self.mem = self.arena.realloc(self.mem as _, self.cap, self.cap * 2) ;
            }
            self.cap *= 2;
        }

        unsafe { 
            ptr::write(self.mem.add(self.len), item);
        }

        self.len += 1;
    }

    pub fn pop(&mut self) -> Option<T> {
        if self.len == 0 {
            None
        } else {
            self.len -= 1;
            unsafe { Some(ptr::read(self.mem.add(self.len))) }
        }
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.len
    }

    pub fn iter(&self) -> Iter<'a, T> {
        Iter {
            start: self.mem,
            end: unsafe { self.mem.add(self.len()) },
            _iter: PhantomData,
        }
    }
}

impl<'a, T: Debug> Debug for ArenaVec<'a, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.as_ref().fmt(f)
    }
}

impl<'a, T> AsRef<[T]> for ArenaVec<'a, T> {
    fn as_ref(&self) -> &[T] {
        unsafe { from_raw_parts(self.mem, self.len) }
    }
}

pub struct Iter<'a, T> {
    start: *const T,
    end: *const T,
    _iter: PhantomData<&'a T>
}

impl<'a, T> Iterator for Iter<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        let next = unsafe { self.start.add(1) };
        if next > self.end {
            return None;
        }

        unsafe { Some(&*next) }
    }
}

pub struct ArenaBox<'a, T> {
    mem: *mut T,
    _arena: PhantomData<&'a T>,
}

impl<'a, T> ArenaBox<'a, T> {
    pub fn new(arena: &'a Arena, thing: T) -> Self {
        let mem = arena.alloc(thing);
        ArenaBox {
            mem,
            _arena: PhantomData,
        }
    }

    #[inline]
    pub fn into_inner(self) -> T {
        unsafe {ptr::read(self.mem)}
    }
}

impl<'a, T> Deref for ArenaBox<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe {&*self.mem}
    }
}

impl<'a, T> DerefMut for ArenaBox<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe {&mut *self.mem}
    }
}

impl<'a, T: Debug> Debug for ArenaBox<'a, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.deref().fmt(f)
    }
}