
mod diff;
mod helper;
mod mutation_manager;
mod undo;

pub use diff::*;
pub use helper::*;
pub use mutation_manager::{
    MutationManager, MutationEvent, ConfigureObserver, mutate,
    configure_observer
};
pub use undo::*;

use cod::{ID, Rc, NodeClone};

pub type DynUpdateEvent = UpdateEvent<dyn NodeClone>;

pub enum UpdateEvent<T: ?Sized> {
    /// Updated data and whether animations should be performed as a result of this update.
    /// 
    /// The animation field is only set to `true` when the update is caused by user interaction,
    /// not on an undo state change or initial build for example.
    Update(Rc<T>, bool),
    /// If a remove event is sent by MutationManager, it signifies that the observed node
    /// has been removed, and no further updates will be sent. The receiving widget may
    /// choose to display a placeholder for instance.
    ///
    /// If instead it is sent as manually or by a Differ, the animation field will
    /// always be set to `true`. (The widget is removed directly instead if no animation should
    /// occur.) The widget should initiate an animation, after which
    /// it should remove itself -- immediately if it does not intend to animate.
    ///
    /// TODO: Possibly make this easier to opt-out of with some changes to tuix.  
    Remove(ID, bool),
}

impl<T: NodeClone + Sized> UpdateEvent<T> {
    pub fn into_dyn(self) -> DynUpdateEvent {
        match self {
            UpdateEvent::Update(node, animate) => UpdateEvent::Update(node, animate),
            UpdateEvent::Remove(id, animate) => UpdateEvent::Remove(id, animate),
        }
    }
}

#[derive(Clone, PartialEq, Debug)]
#[non_exhaustive]
pub enum AnimationRequest {
    Appear,
}

/// XXX: PartialEq not implemented for generic UpdateEvent, effectively to make it
/// !tuix::Message, as they should not be sent as events directly.
impl PartialEq for DynUpdateEvent {
    fn eq(&self, _other: &Self) -> bool { false }
}
impl<T: ?Sized> Clone for UpdateEvent<T> {
    fn clone(&self) -> Self {
        match self {
            UpdateEvent::Update(node, animate) => UpdateEvent::Update(Rc::clone(node), *animate),
            UpdateEvent::Remove(id, animate) => UpdateEvent::Remove(*id, *animate),
        }
    }
}
use std::fmt;
impl<T: ?Sized> fmt::Debug for UpdateEvent<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UpdateEvent::Update(node, animate) => {
                f.debug_tuple("Update")
                    .field(&Rc::as_ptr(node))
                    .field(&animate)
                    .finish()
            },
            UpdateEvent::Remove(id, animate) => {
                f.debug_tuple("Remove")
                    .field(&id)
                    .field(&animate)
                    .finish()
            }
        }
    }
}

