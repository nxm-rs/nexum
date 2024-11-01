use std::{
    cell::{Ref, RefCell, RefMut, UnsafeCell},
    rc::Rc,
};

// Define a generic Singleton wrapper with `UnsafeCell` to encapsulate `Rc<RefCell<Option<T>>>`
pub struct Singleton<T> {
    inner: UnsafeCell<Rc<RefCell<Option<T>>>>,
}

impl<T> Singleton<T> {
    // Creates a new Singleton
    pub fn new() -> Self {
        Singleton {
            inner: UnsafeCell::new(Rc::new(RefCell::new(None))),
        }
    }

    // Initializes the singleton value if not already set, allowing `Some(value)` or `None`
    pub fn initialize(&self, value: Option<T>) -> Result<(), &'static str> {
        unsafe {
            let rc_refcell = &*self.inner.get();
            let mut opt = rc_refcell.borrow_mut();
            if opt.is_some() {
                Err("Already initialized")
            } else {
                *opt = value; // Set the inner Option to the provided value, which could be None or Some(value)
                Ok(())
            }
        }
    }

    // Retrieves a reference to the inner `Option<T>` within the singleton
    pub fn get(&self) -> Ref<Option<T>> {
        unsafe {
            // Borrow the RefCell and map it to directly return a reference to the Option<T>
            Ref::map((&*self.inner.get()).borrow(), |inner| inner)
        }
    }

    // Retrieves a mutable reference to the inner `Option<T>` within the singleton
    pub fn get_mut(&self) -> RefMut<Option<T>> {
        unsafe {
            let rc_refcell = &*self.inner.get();
            rc_refcell.borrow_mut() // Directly return the mutable reference to Option<T>
        }
    }
}

// Implement `Sync` for `Singleton` safely in a single-threaded WASM context
unsafe impl<T> Sync for Singleton<T> {}
unsafe impl<T> Send for Singleton<T> {}
