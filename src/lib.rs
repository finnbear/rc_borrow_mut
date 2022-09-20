#![feature(alloc_layout_extra)]
#![feature(pointer_byte_offsets)]
#![feature(layout_for_ptr)]
#![feature(cell_update)]

use std::fmt;
use std::fmt::{Debug, Formatter};
use std::ops::{Deref, DerefMut};
use std::rc::Rc;

pub trait RcBorrowMut<T: ?Sized> {
    /// Mutably borrows the contents of the Rc.
    ///
    /// # Panics
    ///
    /// If there are other strong references.
    fn borrow_mut(me: &mut Self) -> BorrowRefMut<T> {
        Self::try_borrow_mut(me).unwrap()
    }

    /// Mutably borrows the contents of the Rc.
    ///
    /// Succeeds if the argument is the only strong reference.
    fn try_borrow_mut(me: &mut Self) -> Result<BorrowRefMut<T>, OtherStrongReferencesExist>;
}

pub struct OtherStrongReferencesExist;

impl Debug for OtherStrongReferencesExist {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str("Cannot borrow mutably, other strong references exist.")
    }
}

/// A mutable handle to the contents of an `Rc`.
pub struct BorrowRefMut<'a, T: ?Sized> {
    inner: &'a mut Rc<T>,
}

impl<T: ?Sized + fmt::Debug> fmt::Debug for BorrowRefMut<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        (**self).fmt(f)
    }
}

impl<T: ?Sized + fmt::Display> fmt::Display for BorrowRefMut<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        (**self).fmt(f)
    }
}

impl<T: ?Sized> Deref for BorrowRefMut<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        let raw = Rc::as_ptr(&self.inner);
        unsafe { &*raw }
    }
}

impl<T: ?Sized> DerefMut for BorrowRefMut<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        let raw = Rc::as_ptr(&self.inner);
        unsafe { &mut *(raw as *mut T) }
    }
}

impl<T: ?Sized> Drop for BorrowRefMut<'_, T> {
    fn drop(&mut self) {
        let raw = Rc::as_ptr(&self.inner);
        unsafe {
            let rc_box = hack::raw_to_rc_box(raw);
            (&*rc_box).strong.update(|x| {
                debug_assert_eq!(x, 0);
                x + 1
            });
        }
    }
}

impl<T: ?Sized> RcBorrowMut<T> for Rc<T> {
    fn try_borrow_mut(me: &mut Self) -> Result<BorrowRefMut<T>, OtherStrongReferencesExist> {
        debug_assert_ne!(Rc::strong_count(me), 0);
        if Rc::strong_count(me) > 1 {
            return Err(OtherStrongReferencesExist);
        }

        unsafe {
            let raw = Rc::as_ptr(me);
            let rc_box = hack::raw_to_rc_box(raw);
            (&*rc_box).strong.update(|x| {
                debug_assert_eq!(x, 1);
                x - 1
            });

            Ok(BorrowRefMut { inner: me })
        }
    }
}

mod hack {
    use core::alloc::Layout;
    use std::cell::Cell;
    use std::mem::align_of_val_raw;

    #[repr(C)]
    pub struct RcBox<T: ?Sized> {
        pub strong: Cell<usize>,
        _weak: Cell<usize>,
        _value: T,
    }

    pub unsafe fn raw_to_rc_box<T: ?Sized>(ptr: *const T) -> *mut RcBox<T> {
        let offset = data_offset(ptr);

        // Reverse the offset to find the original RcBox.
        ptr.byte_sub(offset) as *mut RcBox<T>
    }

    unsafe fn data_offset<T: ?Sized>(ptr: *const T) -> usize {
        data_offset_align(align_of_val_raw(ptr))
    }

    #[inline]
    fn data_offset_align(align: usize) -> usize {
        let layout = Layout::new::<RcBox<()>>();
        layout.size() + layout.padding_needed_for(align)
    }
}

#[cfg(test)]
mod tests {
    use crate::RcBorrowMut;
    use std::cell::Cell;
    use std::rc::Rc;

    #[test]
    fn mutate() {
        let mut rc = Rc::new(0);
        let weak = Rc::downgrade(&Rc::clone(&rc));
        let mut mutable = Rc::borrow_mut(&mut rc);
        *mutable += 1;
        assert!(weak.upgrade().is_none());
        assert_eq!(format!("{} {:?}", mutable, mutable), "1 1");
        *mutable += 1;
        drop(mutable);
        assert_eq!(*weak.upgrade().unwrap(), 2);
    }

    #[test]
    fn drop_weak() {
        let mut rc = Rc::new(0);
        let weak = Rc::downgrade(&rc);
        let mut mutable = Rc::borrow_mut(&mut rc);
        *mutable += 1;
        assert!(weak.upgrade().is_none());
        drop(weak);
        *mutable += 1;
        drop(mutable);
    }

    #[test]
    #[should_panic]
    fn panic() {
        let mut rc = Rc::new(0);
        let _rc2 = Rc::clone(&rc);
        let _ = Rc::borrow_mut(&mut rc);
    }

    #[test]
    fn unsize() {
        let mut rc: Rc<[i32]> = vec![0, 2, 1].into();
        let mut mutable = Rc::borrow_mut(&mut rc);
        mutable.sort();
        drop(mutable);
        assert_eq!(*rc, vec![0, 1, 2]);
    }

    #[test]
    fn dropper() {
        struct Dropper {
            dead: Cell<bool>,
            dummy: i32,
        }

        impl Drop for Dropper {
            fn drop(&mut self) {
                assert!(self.dead.get());
            }
        }

        let mut rc: Rc<Dropper> = Rc::new(Dropper {
            dead: Cell::new(false),
            dummy: 0,
        });
        let weak = Rc::downgrade(&rc);
        let mut mutable = Rc::borrow_mut(&mut rc);
        mutable.dummy += 1;
        drop(weak);
        drop(mutable);
        rc.dead.set(true);
        drop(rc);
    }
}
