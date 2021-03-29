
use crate::{Header, Node, Child, State};

#[derive(Clone)]
struct TestNode {
    header: Header,
    data: i32,
    child: Option<Child<TestNode>>,
    second_child: Option<Child<TestNode>>,
}

impl TestNode {
    fn new(data: i32, child: Option<TestNode>) -> TestNode {
        let header = Header::new();
        TestNode {
            data,
            child: child.map(|c| Child::with_parent(&header, c)),
            second_child: None,
            header,
        }
    }
}

// can't use the derive inside the crate
impl Node for TestNode {
    fn header(&self) -> &Header { &self.header }
    fn header_mut(&mut self) -> &mut Header { &mut self.header }
}

#[test]
fn deep_copy() {
    let structure = TestNode::new(1, Some(TestNode::new(2, Some(TestNode::new(3, None)))));
    let state1 = State::new(&structure);
    let mut state2 = state1.clone();
    {
        let mut root = state2.get_mut(state2.root_ref());
        root.second_child = Some(root.child.as_ref().unwrap().clone());
    }
    assert_eq!(state2.root().second_child.as_ref().unwrap().data, 2);
    assert_eq!(state2.root().second_child.as_ref().unwrap().child.as_ref().unwrap().data, 3);
    // deep copy should change ids
    assert_ne!(state2.root().child.as_ref().unwrap().header.id, state2.root().second_child.as_ref().unwrap().header.id);
}
