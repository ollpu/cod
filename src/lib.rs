use std::collections::HashMap as NonPersistentMap;

mod id;
mod session;

pub use id::ID;
use id::new_id;

/// Can be changed to Arc later. However, the design is not thread-aware
/// when mutating. So appropriate !Send/!Syncs need to be defined before changing.
pub type Rc = std::rc::Rc;
pub type Weak = std::rc::Weak;

pub struct Header {
    id: ID,
    parent_id: Option<ID>,
}

pub trait Node {
    fn header(&self) -> &Header;
    fn header_mut(&mut self) -> &mut Header;

    /// Optional: You may implement this method for your struct if it does something special.
    ///
    /// For example, you would want to do this if `.clone()` does not actually clone
    /// all the `Child` instances in the struct.
    /// 
    /// Cou will use this when updating the ancestors of a node that was mutated.
    ///
    /// The implementaion should find the `Child` instance which corresponds to the
    /// given ID, and call `.poll()` on it.
    /// 
    /// If you do implement this method, also make sure to implement `implements_poll_child`
    /// such that it returns true if you want it to be used on `self` specifically.
    fn poll_child(&self, _id: ID) { }
    fn implements_poll_child(&self) -> bool { false }

    /// Optional: You may implement this method for your struct if it does something special.
    /// This includes:
    ///
    ///  - `.clone()` does not actually clone all the `Child` instances in the struct.
    ///    (also implement `poll_child` in this case)
    ///  - `drop` does not actually drop all the `Child` instances in the struct.
    ///  - The struct contains a lot of fields which are expensive to copy and drop.
    ///  - The struct does not safely fit on the stack. (TODO: there are likely other issues with this)
    ///
    /// Cou will use this when removing nodes from the tree, to find the children of this
    /// node. If the implementation is not specialized, Cou will instead clone and then 
    /// immediately drop the struct to determine the children.
    ///
    /// The implementation should call `.poll()` on each `Child` instance it contains
    /// (not recursively!).
    ///
    /// If you do implement this method, also make sure to implement `implements_poll_all`
    /// such that it returns true if you want it to be used on `self` specifically.
    fn poll_all(&self) { }
    fn implements_poll_all(&self) -> bool { false }
}

/// This is a wrapper trait for Node which enables cloning through dynamic dispatch.
/// It will be automatically implemented for any struct that is `Node + Clone`.
pub trait NodeClone: Node {
    fn dyn_clone(&self) -> Rc<dyn NodeClone>;
}

impl<T: Node + Clone> NodeClone for T {
    fn dyn_clone(&self) -> Rc<dyn NodeClone> {
        Rc::new(self.clone())
    }
}

pub struct Child<T: NodeClone> {
    inner_ref: Rc<T>,
    inner_id: ID,
}

impl Child<T> {
    fn new<P: Node>(parent: &P, mut node: T) -> Self {
        node.header_mut().parent_id = Some(parent.header().id);
        let inner_id = node.id;
        let rc = Rc::new(node);
        todo!(); // store the new reference somewhere thread-local if a session is active
        Self {
            inner_ref: rc
            inner_id
        }
    }

    /// Does not clone the child if the reference is unique. This means
    /// this method can be called repeatedly during the same [`State::get_mut`] session wihtout
    /// creating unnecessary clones.
    ///
    /// **`panic!`**s if used on a non-unique reference outside of a mutating session.
    fn make_mut(&mut self) -> &mut T {
        if let Some(node) = self.inner_ref.get_mut() {
            node
        } else {
            self.inner_ref = Rc::new(<self.inner_ref as &NodeClone>::clone());
            todo!(); // store the new reference somewhere thread-local as Weak
            self.inner_ref.get_mut().unwrap()
        }
    }

    fn get_ref(&self) -> Rc<T> {
        self.inner_ref.clone()
    }

    fn get_id(&self) -> ID {
        self.inner_id
    }
}

impl<T> Deref for Child<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        *(self.inner_ref)
    }
}

impl<T> Drop for Child<T> {
    fn drop(&mut self) {
        todo!() // mark removal in thread-local if session is active
    }
}

/// One state of the application.
/// States can be cloned freely and cloning is persistent, so it is very cheap.
///
/// R is the type of the root node.
#[derive(Clone)]
pub struct State<R: NodeClone> {
    root: Rc<R>,
    id_lookup: im::HashMap<ID, Weak<dyn NodeClone>>,
}

impl<R> State<R> {
    /// Calls a closure that constructs the tree. No existing nodes can be moved in,
    /// they all have to be created during the execution of this closure and on the same
    /// thread.
    fn construct<M: StateManager<R>, F: FnOnce() -> R>(manager: Weak<M>, construct: F) -> Rc<Self> {
        // TODO: start normal mutation session?
        let id_lookup = im::HashMap::new();
        todo!();
        let root = construct();
        // end mutation session
        Self {
            root,
            id_lookup,
        }
    }

    /// Due to implementation details, this has to clone the root and all its
    /// children.
    fn new<M: StateManager<R>>(manager: Weak<M>, root: &R) -> Self {
        // TODO: start creation session to collect all IDs
        todo!();
        let root = root.clone();
        Self {
            manager: manager as Rc<dyn StateManager<R>>,
            root,
        }
    }

    fn get_mut<T: NodeClone>(&mut self, node: &mut Rc<T>) -> MutRef<'_, R, T> {
        // TODO: start mutation session & do other stuff
        todo!();

    }

    fn ref_from_id(&self, id: ID) -> Option<Rc<dyn NodeClone>> {
        self.id_lookup.get(id)?.upgrade()
    }

    fn root(&self) -> &R {
        *(self.root)
    }

    fn root_ref(&self) -> Rc<R> {
        self.root.clone()
    }
}

pub struct MutRef<'a, R: NodeClone, T: NodeClone> {
    state: &'a mut State<R>,
    node: &'a mut Rc<T>,
}

impl<'a, R, T> !Send for MutRef {}
impl<'a, R, T> !Sync for MutRef {}

impl<'a, R, T> Deref for MutRef<'a, R, T> {
    type Self::Target = T;
    fn deref(&self) -> &Self::Target {
        *(self.node)
    }
}

impl<'a, R, T> DerefMut for MutRef<'a, R, T> {
    type Self::Target = T;
    fn deref(&mut self) -> &mut Self::Target {
        // Will not panic because the node Rc is mutably borrowed and
        // made unique upon creation of self.
        self.node.get_mut().unwrap()
    }
}

impl<'a, R, T> Drop for MutRef<'a, R, T> {
    fn drop(&mut self) {
        // TODO: end mutation session
        todo!()
    }
}

