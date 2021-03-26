use std::vec::Vec;

use cod::Node;

#[derive(Node, Clone, Debug)]
struct A {
    header: cod::Header,
    some_data: i32,
    child: cod::Child<B>,
    // optional_child: Option<cod::Child<B>>,
    // child_list: Vec<cod::Child<B>>,
}

#[derive(Node, Clone, Debug)]
struct B {
    header: cod::Header,
    data: Vec<i32>,
}

fn main() {
    let state1 = cod::State::construct(|| {
        let header = cod::Header::new();
        A {
            header: header.clone(),
            some_data: 15,
            child: cod::Child::with_parent_header(&header, B { header: Default::default(), data: [1, 3].to_vec() }),
            // optional_child: None,
            // child_list: vec![],
        }
    });
    println!("Initial state:");
    println!("{:#?}", state1.root());
    println!("Performing mutation:");
    let mut state2 = state1.clone();
    {
        let mut b = state2.get_mut(state2.root().child.get_ref());
        b.data.push(123);
    }
    println!("{:#?}", state2.root());
    println!("Old state still accesible:");
    println!("{:#?}", state1.root());
}
