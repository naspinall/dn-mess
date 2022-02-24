use super::tree::Tree;
use crate::messages::packets::ResourceRecord;

pub struct ZoneTree {
    tree: Tree<Zone>,
}

pub struct Zone {
    pub label: String,
    time_to_live: usize,
    records: Vec<ResourceRecord>,
}

impl ZoneTree {
    fn new() -> ZoneTree {
        ZoneTree { tree: Tree::new() }
    }

    fn find_zone(&self, domain: String) -> Option<&Zone> {
        // Split on labels
        let labels = domain.split('.');

        let mut search_id = self.tree.get_root_id()?;

        for label in labels.into_iter() {
            if let Some(found_node) = self
                .tree
                .iter_children(search_id)
                .find(|node| node.data.label.eq(label))
            {
                // We have found the label, continue
                search_id = found_node.id;
                continue;
            }

            // We are done, return records
            break;
        }

        return Some(&self.tree.get_node(search_id)?.data);
    }
}

mod tests {

    use super::*;

    #[test]
    fn test_find_zone() {
        // Set this domain into the tree
        let tree = ZoneTree::new();
    }
}
