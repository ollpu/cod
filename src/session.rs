//! Session defines thread-local context in order to affect what happens in Clone
//! and Drop, among other operations involving tree mutation.

enum SessionStatus {
    Inactive,
    Creation,
    Mutation
}

/// Used when a `Child` is dropped. The whole subtree needs to be cleaned up.
/// Used in both creation and mutation sessions.
/// Any `Child` that is polled or cloned while this is active should perform `cleanup()`
/// in order to continue the traversal down.
enum RemovalStatus {
    Inactive,
    Removing
}

enum IDMapUpdate {
    Set(ID, Weak<dyn crate::NodeClone>),
    Remove(ID),
}
struct IDMapDiff {
    updates: Vec<IDMapUpdate>,
}
