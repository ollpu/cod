
mod diff;
mod mutation_manager;

pub use diff::vec::VecDiffer;
pub use mutation_manager::{
    MutationManager, MutationEvent, ConfigureObserver, ObservationEvent, mutate,
    configure_observer
};

use cod::Rc;

#[derive(Debug, Clone)]
pub struct UpdateEvent<T>(pub Rc<T>);

impl<T> PartialEq for UpdateEvent<T> {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

