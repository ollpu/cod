use crate::{Rc, NodeClone};
use std::any::TypeId;

/// Replicates Rc::downcast, except for our custom trait.
/// Relies on any::TypeId for correctness.
pub fn downcast_rc<T: NodeClone>(rc: Rc<dyn NodeClone>) -> Option<Rc<T>> {
    if (&*rc).type_id() == TypeId::of::<T>() {
        let ptr = Rc::into_raw(rc);
        let ptr: *const T = ptr as *const T;
        // SAFETY: Refer to Rc::downcast. `type_id` should always be defined by the Any
        // trait because that is the only trait with such a method visible here.
        unsafe { Some(Rc::from_raw(ptr)) }
    } else {
        None
    }
}

pub fn downcast_ref<T: NodeClone>(this: &dyn NodeClone) -> Option<&T> {
    if this.type_id() == TypeId::of::<T>() {
        // SAFETY: Refer to <dyn Any>::downcast_ref and above.
        unsafe { Some(& *(this as *const dyn NodeClone as *const T)) }
    } else {
        None
    }
}

pub fn downcast_mut<T: NodeClone>(this: &mut dyn NodeClone) -> Option<&mut T> {
    if this.type_id() == TypeId::of::<T>() {
        // SAFETY: Refer to <dyn Any>::downcast_mut and above.
        unsafe { Some(&mut *(this as *mut dyn NodeClone as *mut T)) }
    } else {
        None
    }
}

