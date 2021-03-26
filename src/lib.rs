use std::ops::{Deref, DerefMut};
use std::any::Any;
use std::fmt::Debug;
use std::fmt;

pub use cod_node_derive::Node;

mod id;
mod context;
mod danger_zone;

pub use id::ID;
use id::new_id;

use context::{CONTEXT, Context, PollReason, Replacement, IDMapUpdate};

use danger_zone::downcast_rc;

/// Can be changed to Arc later. However, the design is not thread-aware
/// when mutating. So appropriate !Send/!Syncs need to be defined before changing.
pub use std::rc::Rc as Rc;
pub use std::rc::Weak as Weak;
pub use im_rc as im;


/// !!! should not derive Clone, needs special behavior for deep clones.
/// -> currently broken
#[derive(Clone, Debug)]
pub struct Header {
    id: ID,
    parent_id: Option<ID>,
}

impl Header {
    pub fn new() -> Self {
        Header {
            id: new_id(),
            parent_id: None
        }
    }
}

impl Default for Header {
    fn default() -> Self {
        Self::new()
    }
}

pub trait Node: 'static {
    fn header(&self) -> &Header;
    fn header_mut(&mut self) -> &mut Header;

    /// Optional: You may implement this method for your struct if it does something special.
    ///
    /// For example, you would want to do this if `.clone()` does not actually clone
    /// all the `Child` instances in the struct.
    /// 
    /// Cod will use this when updating the ancestors of a node that was mutated.
    ///
    /// The implementaion should find the `Child` instance which corresponds to the
    /// given ID, and call `.poll_mut()` on it. You should not do anything else
    /// with the `Child`s, doing so will **`panic!`**.
    /// 
    /// If you do implement this method, also make sure to implement `implements_poll_child`
    /// such that it returns true if you want it to be used on `self` specifically.
    fn poll_child_mut(&mut self, _id: ID) { }
    fn implements_poll_child(&self) -> bool { false }

    /// Optional: You may implement this method for your struct if it does something special.
    /// This includes:
    ///
    ///  - `.clone()` does not actually clone all the `Child` instances in the struct.
    ///    (also implement `poll_child` in this case)
    ///  - The struct contains a lot of fields which are expensive to copy and drop.
    ///  - The struct does not safely fit on the stack. (TODO: there are likely other issues with this)
    ///
    /// Cod will use this when removing nodes from the tree, to find the children of this
    /// node. If the implementation is not specialized, Cod will instead clone and then 
    /// immediately drop the struct to determine the children.
    ///
    /// The implementation should call `.poll()` on each `Child` instance it contains
    /// (not recursively!).
    ///
    /// If you do implement this method, also make sure to implement `implements_poll_all`
    /// such that it returns true if you want it to be used on `self` specifically.
    /// In addition, you should implement `poll_all_mut`.
    fn poll_all(&self) { }
    /// Optional: See [`poll_all`]. This is the mutable version. The implementation should
    /// call `.poll_mut()` on all `Child` instances associated with this node.
    fn poll_all_mut(&mut self) { }
    fn implements_poll_all(&self) -> bool { false }
}

/// This is a wrapper trait for `Node` which enables cloning through dynamic dispatch and RTTI.
/// It will be automatically implemented for any struct that is `Node + Clone`.
pub trait NodeClone: Node + Any {
    fn dyn_clone(&self) -> Rc<dyn NodeClone>;
    /// clone, then immediately drop. used for reflection
    fn cod(&self);
}

impl<T: Node + Clone> NodeClone for T {
    fn dyn_clone(&self) -> Rc<dyn NodeClone> {
        Rc::new(self.clone())
    }
    fn cod(&self) {
        let _ = self.clone();
    }
}

pub struct Child<T: NodeClone> {
    inner_ref: Rc<T>,
    inner_id: ID,
}

impl<T: NodeClone + Clone> Child<T> {
    pub fn with_parent<P: Node>(parent: &P, node: T) -> Self {
        Self::with_parent_id(parent.header().id, node)
    }

    pub fn with_parent_header(parent_header: &Header, node: T) -> Self {
        Self::with_parent_id(parent_header.id, node)
    }

    pub fn with_parent_id(parent_id: ID, mut node: T) -> Self {
        node.header_mut().parent_id = Some(parent_id);
        let inner_id = node.header().id;
        let rc = Rc::new(node);
        let child = Self {
            inner_ref: rc.clone(),
            inner_id
        };
        CONTEXT.with(|c| {
            Context::poll(c, PollReason::Construct, inner_id, rc);
        });
        child
    }



    /// TODO. avoid new clone if child has already been accessed during this mutation session.
    pub fn make_mut(&mut self) -> MakeMutRef<'_, T> {
        CONTEXT.with(|c| {
            if Context::mutation_session_active(c) {
                // let the context handle cloning (special stuff needs to happen)
                if let Some(new_ref) =
                    Context::poll_mut(c, PollReason::MakeMutPre, self.inner_id, Rc::clone(&self.inner_ref)) {
                    self.inner_ref = new_ref;
                }
            } else {
                Rc::make_mut(&mut self.inner_ref);
            }
        });
        MakeMutRef {
            child: self
        }
    }

    pub fn get_ref(&self) -> Rc<T> {
        Rc::clone(&self.inner_ref)
    }

    pub fn get_id(&self) -> ID {
        self.inner_id
    }

    pub fn poll(&self) {
        CONTEXT.with(|c| {
            Context::poll(c, PollReason::Manual, self.inner_id, Rc::clone(&self.inner_ref));
        });
    }

    pub fn poll_mut(&mut self) {
        CONTEXT.with(|c| {
            if let Some(new_ref) =
                Context::poll_mut(c, PollReason::ManualMut, self.inner_id, Rc::clone(&self.inner_ref)) {
                self.inner_ref = new_ref;
            }
        });
    }
}

impl<T: NodeClone> Deref for Child<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.inner_ref
    }
}

impl<T: NodeClone> Clone for Child<T> {
    fn clone(&self) -> Self {
        let mut child = Self {
            inner_ref: Rc::clone(&self.inner_ref),
            inner_id: self.inner_id,
        };
        CONTEXT.with(|c| {
            if let Some(new_ref) =
                Context::poll_mut(c, PollReason::Clone, child.inner_id, Rc::clone(&child.inner_ref)) {
                child.inner_ref = new_ref;
            }
        });
        child
    }
}

impl<T: NodeClone> Drop for Child<T> {
    fn drop(&mut self) {
        if !std::thread::panicking() {
            CONTEXT.with(|c| {
                Context::poll(c, PollReason::Drop, self.inner_id, Rc::clone(&self.inner_ref));
            });
        }
    }
}

pub struct MakeMutRef<'a, T: NodeClone> {
    child: &'a mut Child<T>
}

impl<'a, T: NodeClone> Deref for MakeMutRef<'a, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.child.inner_ref
    }
}

impl<'a, T: NodeClone> DerefMut for MakeMutRef<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        // Will not panic because the Child is mutably borrowed and
        // the Rc was made unique upon creation of self
        Rc::get_mut(&mut self.child.inner_ref).unwrap()
    }
}

impl<'a, T: NodeClone> Drop for MakeMutRef<'a, T> {
    fn drop(&mut self) {
        CONTEXT.with(|c| {
            Context::poll(c, PollReason::MakeMutPost, self.child.inner_id, Rc::clone(&self.child.inner_ref));
        });
    }
}

impl<T: NodeClone + Debug> Debug for Child<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&*(self.inner_ref), f)
    }
}

/// One state of the application.
/// States can be cloned freely and cloning is persistent, so it is very cheap.
///
/// R is the type of the root node.
#[derive(Clone)]
pub struct State<R: NodeClone + Clone> {
    root: Rc<R>,
    id_lookup: im::HashMap<ID, Weak<dyn NodeClone>>,
}

impl<R: NodeClone + Clone> State<R> {
    /// Calls a closure that constructs the tree. No existing nodes can be moved in,
    /// they all have to be created during the execution of this closure and on the same
    /// thread.
    pub fn construct<F: FnOnce() -> R>(construct: F) -> Self {
        CONTEXT.with(|c| {
            Context::begin_mutate(c);
        });
        let root = Rc::new(construct());
        let mut state = Self {
            root: Rc::clone(&root),
            id_lookup: im::HashMap::new(),
        };
        CONTEXT.with(|c| {
            state.apply_updates(Context::end_mutate(c));
        });
        state.id_lookup.insert(root.header().id, Rc::downgrade(&root) as Weak<dyn NodeClone>);
        state
    }

    /// Due to implementation details, this has to clone the root and all its
    /// children.
    pub fn new(root: &R) -> Self {
        CONTEXT.with(|c| {
            Context::begin_mutate(c);
        });
        // this initiates a deep clone because mutation context is active
        let root = Rc::new(root.clone());
        let mut state = Self {
            root: Rc::clone(&root),
            id_lookup: im::HashMap::new(),
        };
        CONTEXT.with(|c| {
            state.apply_updates(Context::end_mutate(c));
        });
        state.id_lookup.insert(root.header().id, Rc::downgrade(&root) as Weak<dyn NodeClone>);
        state
    }

    pub fn get_mut<'a, T: NodeClone + Clone>(&'a mut self, mut node: Rc<T>) -> MutRef<'a, R, T> {
        Rc::make_mut(&mut node);
        CONTEXT.with(|c| {
            Context::begin_mutate(c);
        });
        MutRef {
            state: self,
            node
        }
    }

    pub fn ref_from_id(&self, id: ID) -> Option<Rc<dyn NodeClone>> {
        Weak::upgrade(self.id_lookup.get(&id)?)
    }

    pub fn root(&self) -> &R {
        &self.root
    }

    pub fn root_ref(&self) -> Rc<R> {
        Rc::clone(&self.root)
    }

    fn apply_updates(&mut self, updates: impl Iterator<Item=IDMapUpdate>) {
        for update in updates {
            match update {
                IDMapUpdate::Set(id, new_ref) => {
                    self.id_lookup.insert(id, new_ref);
                },
                IDMapUpdate::Erase(id) => {
                    self.id_lookup.remove(&id);
                },
            }
        }
    }
}

pub struct MutRef<'a, R: NodeClone + Clone, T: NodeClone> {
    state: &'a mut State<R>,
    node: Rc<T>,
}

impl<'a, R: NodeClone + Clone, T: NodeClone> Deref for MutRef<'a, R, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.node
    }
}

impl<'a, R: NodeClone + Clone, T: NodeClone> DerefMut for MutRef<'a, R, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        // Will not panic because the node Rc is mutably borrowed and
        // made unique upon creation of self.
        Rc::get_mut(&mut self.node).unwrap()
    }
}

impl<'a, R: NodeClone + Clone, T: NodeClone> Drop for MutRef<'a, R, T> {
    fn drop(&mut self) {
        CONTEXT.with(|c| {
            self.state.apply_updates(Context::end_mutate(c));
        });
        self.state.id_lookup.insert(self.node.header().id, Rc::downgrade(&self.node) as Weak<dyn NodeClone>);
        let mut prev_node = Rc::clone(&self.node) as Rc<dyn NodeClone>;
        while let Some(parent_id) = prev_node.header().parent_id {
            let parent = Weak::upgrade(self.state.id_lookup.get(&parent_id).unwrap()).unwrap();
            CONTEXT.with(|c| {
                Context::set_replacement(c,
                    Replacement { id: prev_node.header().id, replace_with: Rc::clone(&prev_node) as Rc<dyn NodeClone> }
                );
            });
            prev_node = parent.dyn_clone();
            CONTEXT.with(|c| {
                if !Context::finish_replacement(c) {
                    panic!("Cod: Could not find associated `Child` while traversing up")
                }
            });
        }
        CONTEXT.with(|c| {
            self.state.apply_updates(Context::end_replacement(c));
        });
        self.state.root = downcast_rc(prev_node).unwrap();
    }
}

