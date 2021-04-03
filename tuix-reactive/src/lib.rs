
mod diff;
mod mutation_manager;

pub use diff::*;
pub use mutation_manager::{
    MutationManager, MutationEvent, ConfigureObserver, ObservationEvent, mutate,
    configure_observer
};

use cod::Rc;

#[derive(Clone)]
pub enum UpdateEvent<T> {
    /// Updated data and whether animations should be performed as a result of this update.
    /// 
    /// The animation field is only set to `true` when the update is caused by user interaction,
    /// not on an undo state change or initial build for example.
    Update(Rc<T>, bool),
    /// A Remove variant is only sent when an animation should be performed, otherwise the widget
    /// would be removed directly. The receiving widget is responsible for removing itself
    /// eventually -- immediately if it does not intend to animate.
    ///
    /// TODO: Possibly make this easier to opt-out of with some changes to tuix.
    Remove,
}

#[derive(Clone, PartialEq, Debug)]
#[non_exhaustive]
pub enum AnimationRequest {
    Appear,
}

impl<T> PartialEq for UpdateEvent<T> {
    fn eq(&self, _other: &Self) -> bool { false }
}
use std::fmt;
impl<T> fmt::Debug for UpdateEvent<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UpdateEvent::Update(node, animate) => {
                f.debug_tuple("Updated")
                    .field(&Rc::as_ptr(node))
                    .field(&animate)
                    .finish()
            },
            UpdateEvent::Remove => {
                f.debug_tuple("Removed").finish()
            }
        }
    }
}

