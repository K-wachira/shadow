use serde_json::Value;

/// A node in the JSON tree
#[derive(Debug, Clone)]
pub struct TreeNode {
    pub key: String,
    pub value: NodeValue,
    pub depth: usize,
    pub expanded: bool,
}

#[derive(Debug, Clone)]
pub enum NodeValue {
    Object(Vec<TreeNode>),
    Array(Vec<TreeNode>),
    Leaf(String),
}

impl TreeNode {
    pub fn from_json(key: String, value: &Value, depth: usize, expanded: bool) -> Self {
        let node_value = match value {
            Value::Object(map) => {
                let children = map
                    .iter()
                    .map(|(k, v)| TreeNode::from_json(k.clone(), v, depth + 1, expanded))
                    .collect();
                NodeValue::Object(children)
            }
            Value::Array(arr) => {
                let children = arr
                    .iter()
                    .enumerate()
                    .map(|(i, v)| TreeNode::from_json(format!("[{}]", i), v, depth + 1, expanded))
                    .collect();
                NodeValue::Array(children)
            }
            Value::String(s) => NodeValue::Leaf(format!("\"{}\"", s)),
            Value::Number(n) => NodeValue::Leaf(n.to_string()),
            Value::Bool(b) => NodeValue::Leaf(b.to_string()),
            Value::Null => NodeValue::Leaf("null".to_string()),
        };

        TreeNode {
            key,
            value: node_value,
            depth,
            expanded, // all levels closed by default
        }
    }

    pub fn is_expandable(&self) -> bool {
        matches!(self.value, NodeValue::Object(_) | NodeValue::Array(_))
    }
}

/// A flattened row ready for rendering
#[derive(Debug, Clone)]
pub struct FlatRow {
    pub depth: usize,
    pub key: String,
    pub display: RowDisplay,
    pub path: Vec<usize>,
}

#[derive(Debug, Clone)]
pub enum RowDisplay {
    Expandable { expanded: bool, child_count: usize, is_object: bool },
    Leaf(String),
}

/// The stateful tree — stored inside MessageKind::MemoryTree
#[derive(Debug, Clone)]
pub struct JsonTree {
    pub roots: Vec<TreeNode>,
    pub flat: Vec<FlatRow>,
    pub cursor: usize,
    pub scroll: usize,
}

impl JsonTree {
    pub fn from_value(value: &Value, expanded: bool) -> Self {
        let roots = match value {
            Value::Object(map) => map
                .iter()
                .map(|(k, v)| TreeNode::from_json(k.clone(), v, 0, expanded))
                .collect(),
            _ => vec![TreeNode::from_json("root".into(), value, 0, expanded)],
        };

        let mut tree = JsonTree { roots, flat: vec![], cursor: 0, scroll: 0 };
        tree.rebuild_flat();
        tree
    }

    pub fn rebuild_flat(&mut self) {
        self.flat.clear();
        for (i, root) in self.roots.iter().enumerate() {
            collect_flat(root, &[i], &mut self.flat);
        }
    }

    pub fn move_up(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
        }
    }

    pub fn move_down(&mut self) {
        if self.cursor + 1 < self.flat.len() {
            self.cursor += 1;
        }
    }

    pub fn toggle_current(&mut self) {
        let path = self.flat[self.cursor].path.clone();
        if let Some(node) = get_node_mut(&mut self.roots, &path) {
            if node.is_expandable() {
                node.expanded = !node.expanded;
            }
        }
        self.rebuild_flat();
        self.cursor = self.cursor.min(self.flat.len().saturating_sub(1));
    }

    /// Returns a copyable string for the currently selected row
    pub fn selected_value(&self) -> Option<String> {
        self.flat.get(self.cursor).and_then(|row| match &row.display {
            RowDisplay::Leaf(v) => Some(format!("{}: {}", row.key, v)),
            _ => None,
        })
    }

    pub fn adjust_scroll(&mut self, viewport_height: usize) {
        if self.cursor < self.scroll {
            self.scroll = self.cursor;
        } else if self.cursor >= self.scroll + viewport_height {
            self.scroll = self.cursor - viewport_height + 1;
        }
    }
}

fn collect_flat(node: &TreeNode, path: &[usize], flat: &mut Vec<FlatRow>) {
    let display = match &node.value {
        NodeValue::Object(children) => RowDisplay::Expandable {
            expanded: node.expanded,
            child_count: children.len(),
            is_object: true,
        },
        NodeValue::Array(children) => RowDisplay::Expandable {
            expanded: node.expanded,
            child_count: children.len(),
            is_object: false,
        },
        NodeValue::Leaf(s) => RowDisplay::Leaf(s.clone()),
    };

    flat.push(FlatRow { depth: node.depth, key: node.key.clone(), display, path: path.to_vec() });

    if node.expanded {
        let children = match &node.value {
            NodeValue::Object(c) | NodeValue::Array(c) => c,
            NodeValue::Leaf(_) => return,
        };
        for (i, child) in children.iter().enumerate() {
            let mut child_path = path.to_vec();
            child_path.push(i);
            collect_flat(child, &child_path, flat);
        }
    }
}

fn get_node_mut<'a>(roots: &'a mut Vec<TreeNode>, path: &[usize]) -> Option<&'a mut TreeNode> {
    if path.is_empty() {
        return None;
    }
    let mut node = roots.get_mut(path[0])?;
    for &idx in &path[1..] {
        node = match &mut node.value {
            NodeValue::Object(c) | NodeValue::Array(c) => c.get_mut(idx)?,
            NodeValue::Leaf(_) => return None,
        };
    }
    Some(node)
}