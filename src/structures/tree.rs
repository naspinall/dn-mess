use std::collections::HashMap;

pub struct Node<T> {
    pub data: T,

    // Own ID
    pub id: usize,

    // Sibling references
    pub next_sibling: Option<usize>,
    pub previous_sibling: Option<usize>,

    // Children references
    pub first_child: Option<usize>,
    pub last_child: Option<usize>,
}

pub struct Tree<T> {
    nodes: HashMap<usize, Node<T>>,
    current_index: usize,
    root: Option<usize>,
}

pub struct ChildIterator<'tree, T> {
    id: usize,
    nodes: &'tree HashMap<usize, Node<T>>,
}

impl<'tree, T> ChildIterator<'tree, T> {
    fn new(id: usize, nodes: &'tree HashMap<usize, Node<T>>) -> ChildIterator<'tree, T> {
        ChildIterator { id, nodes }
    }
}

impl<'tree, T> Iterator for ChildIterator<'tree, T> {
    fn next(&mut self) -> Option<&'tree Node<T>> {
        let current_node = self.nodes.get(&self.id)?;
        let next_node_id = current_node.next_sibling?;

        // Set id to next id to continue iteration
        self.id = next_node_id;

        // Return next node
        self.nodes.get(&next_node_id)
    }

    type Item = &'tree Node<T>;
}

impl<T> Tree<T> {
    pub fn new() -> Tree<T> {
        Tree {
            nodes: HashMap::new(),
            current_index: 0,
            root: None,
        }
    }

    pub fn add(&mut self, data: T) -> usize {
        let id = self.next_index();

        if self.root.is_none() {
            self.root = Some(id)
        }

        let node = Node {
            data,
            id,
            next_sibling: None,
            previous_sibling: None,
            first_child: None,
            last_child: None,
        };

        self.nodes.insert(id, node);

        id
    }

    fn next_index(&mut self) -> usize {
        self.current_index += 1;
        self.current_index
    }

    pub fn add_child(&mut self, parent_id: usize, id: usize) {
        // Get parent node
        let parent = match self.get_node_mut(parent_id) {
            Some(node) => node,
            None => return,
        };

        let previous_last_child = parent.last_child;

        // Set new child to last child
        parent.last_child = Some(id);

        // If there is no first child, add id as the first child
        if parent.first_child.is_none() {
            parent.first_child = Some(id)
        }

        // Get last child
        if let Some(child_id) = previous_last_child {
            // Add sibling to the last child
            self.add_sibling(child_id, id);
        }
    }

    pub fn add_sibling(&mut self, sibling_id: usize, id: usize) {
        // Get the sibling
        let sibling = match self.get_node_mut(sibling_id) {
            Some(node) => node,
            None => return,
        };

        // Set old next, to new previous
        sibling.previous_sibling = sibling.next_sibling;

        // Next sibling is the new sibling
        sibling.next_sibling = Some(id);
    }

    pub fn get_node(&self, id: usize) -> Option<&Node<T>> {
        self.nodes.get(&id)
    }

    pub fn get_node_mut(&mut self, id: usize) -> Option<&mut Node<T>> {
        self.nodes.get_mut(&id)
    }

    pub fn get_first_child(&self, id: usize) -> Option<&Node<T>> {
        let child_id = self.get_node(id)?.first_child?;
        self.get_node(child_id)
    }

    pub fn get_first_child_mut(&mut self, id: usize) -> Option<&mut Node<T>> {
        let child_id = self.get_node(id)?.first_child?;
        self.get_node_mut(child_id)
    }

    pub fn iter_children(&self, id: usize) -> ChildIterator<T> {
        ChildIterator::new(id, &self.nodes)
    }

    pub fn get_root_id(&self) -> Option<usize> {
        self.root
    }
}

mod tests {

    use super::*;

    #[test]
    fn test_add_nodes() {
        let mut tree = Tree::new();

        let value = "Hello".to_string();

        let id = tree.add(value);

        let root = tree.get_node(tree.root.unwrap());

        assert_eq!(tree.root.unwrap(), id);
        assert_eq!(root.unwrap().data, "Hello".to_string());
    }

    #[test]
    fn test_add_child() {
        let mut tree = Tree::new();

        let value = "Hello".to_string();

        let root_id = tree.add(value);

        let mut children = vec![];

        // Add 100 children to the root
        for i in 0..100 {
            let value = "Hello".to_string();
            let id = tree.add(value);

            children.push(id);
            tree.add_child(root_id, id)
        }

        tree.iter_children(root_id)
            .for_each(|node| assert!(children.iter().position(|id| id == &node.id).is_some()))
    }
}
