use std::{cell::RefCell, rc::Rc};

struct Tree {
    root: Rc<Node>,
}

struct Node {
    name: String,
    children: Vec<Rc<Node>>,
}

impl Tree {
    fn find_zone(&self, domain: String) -> &Node {
        // Node we are using to search
        let mut search_node = &self.root;

        // Split the labels on period, reverse to start from the end
        let labels = domain.split('.').rev();

        for label in labels {
            for child in search_node.children.iter() {
                if label == child.name {
                    search_node = child;
                }
            }
        }

        search_node
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_zone() {
        let tree = Tree {
            root: Rc::new(Node {
                name: "com".to_string(),
                children: vec![Rc::new(Node {
                    name: "example".to_string(),
                    children: vec![],
                })],
            }),
        };

        let found_node = tree.find_zone("example.com.".to_string());

        assert_eq!(found_node.name, "example".to_string());
    }
}
