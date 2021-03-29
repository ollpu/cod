//! Context defines thread-local context in order to affect what happens in Clone
//! and Drop, among other operations involving tree mutation.

use std::cell::RefCell;
use crate::{NodeClone, ID, Rc, Weak};
use crate::danger_zone::downcast_rc;
use crate::id::new_id;

thread_local! {
    pub(crate) static CONTEXT: RefCell<Context> = Default::default();
}

#[derive(Default)]
pub(crate) struct Context {
    status: ContextStatus,
    updates: Vec<IDMapUpdate>,
    /// FIXME: is never shrinked
    deep_copy_id_stack: Vec<ID>,
}

#[derive(Clone)]
enum ContextStatus {
    Inactive,
    Mutation(TraversalStatus),
    Propagation(Replacement, bool)
}

impl Default for ContextStatus {
    fn default() -> ContextStatus {
        ContextStatus::Inactive
    }
}

/// Traversal occurs when a `Child` is dropped or cloned (meaning a deep copy should be made).
/// Both require recursively traversing the whole subtree of that `Child`.
/// Cod has exclusive control of the traversal, and only one can be in progress at a time.
///
/// Implemented as a DFS on the execution stack (there's not much other options because
/// we cannot go back to update a `Child` pointer after it has been polled through clone).
/// This is fine, since `Drop` on complete deallocation would do that anyways.
#[derive(Clone, Copy)]
enum TraversalStatus {
    Inactive,
    DeepCopy,
    Removal,
}

#[derive(Clone)]
pub(crate) struct Replacement {
    pub(crate) id: ID,
    pub(crate) replace_with: Rc<dyn NodeClone>,
}

pub(crate) enum IDMapUpdate {
    Set(ID, Weak<dyn NodeClone>),
    Erase(ID),
}

/// Why [`Context::poll`] was called
#[derive(Clone, Copy)]
pub(crate) enum PollReason {
    Construct,
    DeepCopy(ID),
    Clone,
    Drop,
    MakeMutPre,
    MakeMutPost,
    Manual,
    ManualMut,
}

impl Context {
    pub(crate) fn poll<T: NodeClone>(context: &RefCell<Self>, reason: PollReason, node: Rc<T>) {
        // poll is quite massive, so I assume dynamic dispatch is a net positive
        let changed = Context::poll_dyn(context, reason, node as Rc<dyn NodeClone>);
        assert!(changed.is_none());
    }

    pub(crate) fn poll_mut<T: NodeClone>(context: &RefCell<Self>, reason: PollReason, node: Rc<T>)
        -> Option<Rc<T>> {
        Context::poll_dyn(context, reason, node as Rc<dyn NodeClone>).map(|n| downcast_rc(n).unwrap())
    }

    pub(crate) fn poll_dyn(context: &RefCell<Self>, reason: PollReason, node: Rc<dyn NodeClone>)
        -> Option<Rc<dyn NodeClone>> {
        let id = node.header().id;
        // clone not strictly necessary for all arms, can be improved
        let status = context.borrow().status.clone();
        match status {
            ContextStatus::Inactive => None,
            ContextStatus::Mutation(traversal) => {
                match traversal {
                    TraversalStatus::Inactive => {
                        match reason {
                            PollReason::Construct => {
                                // register new node
                                context.borrow_mut().node_map_update(id, &node);
                                None
                            },
                            PollReason::DeepCopy(parent_id) => {
                                // initiate deep copy changing parent
                                {
                                    let mut context = context.borrow_mut();
                                    context.status = ContextStatus::Mutation(TraversalStatus::DeepCopy);
                                    context.deep_copy_id_stack.clear();
                                    context.deep_copy_id_stack.push(parent_id);
                                }
                                let new_node = Context::poll_dyn(context, reason, node);
                                context.borrow_mut().status = ContextStatus::Mutation(TraversalStatus::Inactive);
                                new_node
                            },
                            PollReason::Clone => {
                                // initiate deep copy without changing parent
                                {
                                    let mut context = context.borrow_mut();
                                    context.status = ContextStatus::Mutation(TraversalStatus::DeepCopy);
                                    context.deep_copy_id_stack.clear();
                                }
                                let new_node = Context::poll_dyn(context, reason, node);
                                context.borrow_mut().status = ContextStatus::Mutation(TraversalStatus::Inactive);
                                new_node
                            },
                            PollReason::Drop => {
                                // initiate recursive removal
                                context.borrow_mut().status = ContextStatus::Mutation(TraversalStatus::Removal);
                                let new_node = Context::poll_dyn(context, reason, node);
                                context.borrow_mut().status = ContextStatus::Mutation(TraversalStatus::Inactive);
                                new_node
                            },
                            PollReason::MakeMutPre => {
                                // temporarily deactivate the context
                                context.borrow_mut().status = ContextStatus::Inactive;
                                // clone the node (clones will now not cause polls)
                                let new_node = node.dyn_clone();
                                // reactivate context
                                context.borrow_mut().status = ContextStatus::Mutation(TraversalStatus::Inactive);
                                // do not store map update yet. MakeMutPost handles that.
                                Some(new_node)
                            },
                            PollReason::MakeMutPost => {
                                context.borrow_mut().node_map_update(id, &node);
                                None
                            }
                            _ => panic!("Cod: `poll()` or `poll_mut()` called in an unexpected context")
                        }
                    },
                    TraversalStatus::DeepCopy => {
                        match reason {
                            PollReason::Clone | PollReason::ManualMut => {
                                let mut new_node;
                                let cloned_id = new_id();
                                context.borrow_mut().deep_copy_id_stack.push(cloned_id);
                                if node.implements_poll_all() {
                                    // temporarily deactivate the context
                                    context.borrow_mut().status = ContextStatus::Inactive;
                                    // clone the node (clones will now not cause polls)
                                    new_node = node.dyn_clone();
                                    // reactivate context
                                    context.borrow_mut().status = ContextStatus::Mutation(TraversalStatus::DeepCopy);
                                    // recurse. inner_ref is now unique so does not panic
                                    Rc::get_mut(&mut new_node).unwrap().poll_all_mut();
                                } else {
                                    // recurse directly, clones will cause polls
                                    new_node = node.dyn_clone();
                                }
                                context.borrow_mut().deep_copy_id_stack.pop();
                                let mut header = Rc::get_mut(&mut new_node).unwrap().header_mut();
                                header.id = cloned_id;
                                // parent_id not changed on the topmost node of the deep copy
                                if let Some(parent_id) = context.borrow_mut().deep_copy_id_stack.last() {
                                    header.parent_id = Some(*parent_id);
                                }
                                // store map update
                                context.borrow_mut().node_map_update(cloned_id, &new_node);
                                Some(new_node)
                            },
                            PollReason::Manual => {
                                panic!("Cod: `poll()` called when `poll_mut()` was expected")
                            },
                            _ => panic!()
                        }
                    },
                    TraversalStatus::Removal => {
                        match reason {
                            PollReason::Clone | PollReason::Manual => {
                                if node.implements_poll_all() {
                                    node.poll_all();
                                } else {
                                    // namesake of this crate: Clone on Drop
                                    // let me explain.
                                    // we are cleaning up the node pointed to by `child`, but in order
                                    // to do that, we need to iterate over its subtree.
                                    // we abuse Clone as a form of reflection to find the children
                                    // of this node.
                                    node.cod();
                                }
                                // store map removal
                                context.borrow_mut().node_map_erase(id);
                                None
                            },
                            PollReason::Drop => None,
                            _ => panic!()
                        }
                    },
                }
            },
            ContextStatus::Propagation(Replacement { id: target_id, replace_with }, found) => {
                // TODO: early exit if already found? feels dubious
                if id == target_id {
                    match reason {
                        PollReason::Clone | PollReason::ManualMut => {
                            if found {
                                panic!("Cod: The same ID was found in multiple `Child`s, state is corrupted")
                            }
                            match &mut context.borrow_mut().status {
                                ContextStatus::Propagation(_, ref mut found) => {
                                    *found = true;
                                },
                                _ => ()
                            }
                            Some(replace_with.clone())
                        },
                        PollReason::Manual => {
                            panic!("Cod: `poll()` called when `poll_mut()` was expected")
                        }
                        _ => panic!()
                    }
                } else {
                    // no match here
                    None
                }
            },
        }
    }

    fn node_map_update(&mut self, id: ID, node: &Rc<dyn NodeClone>) {
        self.updates.push(IDMapUpdate::Set(id, Rc::downgrade(node)));
    }

    fn node_map_erase(&mut self, id: ID) {
        self.updates.push(IDMapUpdate::Erase(id));
    }

    pub(crate) fn begin_mutate(context: &RefCell<Self>) {
        let mut context = context.borrow_mut();
        assert!(matches!(context.status, ContextStatus::Inactive));
        context.status = ContextStatus::Mutation(TraversalStatus::Inactive);
    }

    pub(crate) fn mutation_session_active(context: &RefCell<Self>) -> bool {
        matches!(context.borrow().status, ContextStatus::Mutation(_))
    }

    pub(crate) fn end_mutate(context: &RefCell<Self>) -> impl Iterator<Item=IDMapUpdate> {
        let mut context = context.borrow_mut();
        assert!(matches!(context.status, ContextStatus::Mutation(TraversalStatus::Inactive)));
        context.status = ContextStatus::Inactive;
        let updates = std::mem::take(&mut context.updates);
        updates.into_iter()
    }

    pub(crate) fn set_replacement(context: &RefCell<Self>, replacement: Replacement) {
        let mut context = context.borrow_mut();
        assert!(matches!(context.status, ContextStatus::Inactive));
        context.status = ContextStatus::Propagation(replacement, false);
    }

    pub(crate) fn finish_replacement(context: &RefCell<Self>) -> bool {
        let mut context = context.borrow_mut();
        let result = match context.status {
            ContextStatus::Propagation(_, found) => found,
            _ => panic!()
        };
        context.status = ContextStatus::Inactive;
        result
    }

    pub(crate) fn end_replacement(context: &RefCell<Self>) -> impl Iterator<Item=IDMapUpdate> {
        let mut context = context.borrow_mut();
        assert!(matches!(context.status, ContextStatus::Inactive));
        let updates = std::mem::take(&mut context.updates);
        updates.into_iter()
    }
}

