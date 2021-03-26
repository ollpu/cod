use crate::{Rc, NodeClone};
use std::any::TypeId;

/// Replicates Rc::downcast, except for our custom trait.
/// Relies on any::TypeId for correctness.
pub(crate) fn downcast_rc<T: NodeClone>(rc: Rc<dyn NodeClone>) -> Option<Rc<T>> {
    if (&*rc).type_id() == TypeId::of::<T>() {
        let ptr = Rc::into_raw(rc);
        let ptr: *const T = ptr as *const T;
        unsafe { Some(Rc::from_raw(ptr)) }
    } else {
        None
    }
}
