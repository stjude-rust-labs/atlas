use std::ops::RangeInclusive;

type Key<K> = RangeInclusive<K>;

pub struct IntervalTree<K, V> {
    root: Option<Box<Node<K, V>>>,
}

impl<K, V> IntervalTree<K, V>
where
    K: Clone + Ord,
{
    pub fn insert(&mut self, key: Key<K>, value: V) {
        self.root = set(self.root.take(), key, value);
    }

    pub fn find(&self, key: RangeInclusive<K>) -> Find<'_, K, V> {
        let nodes = self.root.iter().map(|root| root.as_ref()).collect();
        Find::new(nodes, key)
    }
}

impl<K, V> Default for IntervalTree<K, V> {
    fn default() -> Self {
        Self { root: None }
    }
}

fn set<K, V>(mut root: Option<Box<Node<K, V>>>, key: Key<K>, value: V) -> Option<Box<Node<K, V>>>
where
    K: Clone + Ord,
{
    if let Some(node) = root.take() {
        Some(insert(node, key, value))
    } else {
        Some(Box::new(Node::new(key, value)))
    }
}

fn insert<K, V>(mut root: Box<Node<K, V>>, key: Key<K>, value: V) -> Box<Node<K, V>>
where
    K: Clone + Ord,
{
    if key.start() <= root.key.start() {
        root.left = set(root.left.take(), key, value);
    } else if key.start() > root.key.start() {
        root.right = set(root.right.take(), key, value);
    }

    update_height(&mut root);
    update_max(&mut root);

    balance(root)
}

fn update_height<K, V>(root: &mut Node<K, V>) {
    let left_height = height(root.left.as_deref());
    let right_height = height(root.right.as_deref());
    root.height = left_height.max(right_height) + 1;
}

fn height<K, V>(root: Option<&Node<K, V>>) -> u32 {
    root.map_or(0, |node| node.height)
}

fn update_max<K, V>(root: &mut Node<K, V>)
where
    K: Clone + Ord,
{
    root.max = root.key.end().clone();

    if let Some(ref left) = root.left {
        if left.max > root.max {
            root.max = left.max.clone();
        }
    }

    if let Some(ref right) = root.right {
        if right.max > root.max {
            root.max = right.max.clone();
        }
    }
}

enum BalanceFactor {
    LeftHeavy,
    Balanced,
    RightHeavy,
}

fn balance<K, V>(root: Box<Node<K, V>>) -> Box<Node<K, V>>
where
    K: Clone + Ord,
{
    match balance_factor(&root) {
        BalanceFactor::LeftHeavy => balance_left_heavy_tree(root),
        BalanceFactor::Balanced => root,
        BalanceFactor::RightHeavy => balance_right_heavy_tree(root),
    }
}

fn balance_factor<K, V>(root: &Node<K, V>) -> BalanceFactor {
    let left_height = height(root.left.as_deref());
    let right_height = height(root.right.as_deref());

    if left_height > right_height && left_height - right_height >= 2 {
        BalanceFactor::LeftHeavy
    } else if left_height < right_height && right_height - left_height >= 2 {
        BalanceFactor::RightHeavy
    } else {
        BalanceFactor::Balanced
    }
}

fn balance_left_heavy_tree<K, V>(mut root: Box<Node<K, V>>) -> Box<Node<K, V>>
where
    K: Clone + Ord,
{
    let left = root.left.take().expect("invalid tree state");

    if height(left.left.as_deref()) < height(left.right.as_deref()) {
        let new_left = rotate_left(left);
        root.left = Some(new_left);
        update_height(&mut root);
        update_max(&mut root);
    } else {
        root.left = Some(left);
    }

    rotate_right(root)
}

fn balance_right_heavy_tree<K, V>(mut root: Box<Node<K, V>>) -> Box<Node<K, V>>
where
    K: Clone + Ord,
{
    let right = root.right.take().expect("invalid tree state");

    if height(right.left.as_deref()) > height(right.right.as_deref()) {
        let new_right = rotate_right(right);
        root.right = Some(new_right);
        update_height(&mut root);
        update_max(&mut root);
    } else {
        root.right = Some(right);
    }

    rotate_left(root)
}

fn rotate_left<K, V>(mut root: Box<Node<K, V>>) -> Box<Node<K, V>>
where
    K: Clone + Ord,
{
    let mut new_root = root.right.take().expect("invalid tree state");

    root.right = new_root.left.take();
    update_height(&mut root);
    update_max(&mut root);

    new_root.left = Some(root);
    update_height(&mut new_root);
    update_max(&mut new_root);

    new_root
}

fn rotate_right<K, V>(mut root: Box<Node<K, V>>) -> Box<Node<K, V>>
where
    K: Clone + Ord,
{
    let mut new_root = root.left.take().expect("invalid tree state");

    root.left = new_root.right.take();
    update_height(&mut root);
    update_max(&mut root);

    new_root.right = Some(root);
    update_height(&mut new_root);
    update_max(&mut new_root);

    new_root
}

struct Node<K, V> {
    key: Key<K>,
    value: V,
    max: K,
    height: u32,
    left: Option<Box<Node<K, V>>>,
    right: Option<Box<Node<K, V>>>,
}

impl<K, V> Node<K, V>
where
    K: Clone,
{
    fn new(key: Key<K>, value: V) -> Self {
        let max = key.end().clone();

        Self {
            key,
            value,
            max,
            height: 1,
            left: None,
            right: None,
        }
    }
}

pub struct Find<'t, K, V> {
    nodes: Vec<&'t Node<K, V>>,
    key: RangeInclusive<K>,
}

impl<'t, K, V> Find<'t, K, V> {
    fn new(nodes: Vec<&'t Node<K, V>>, key: RangeInclusive<K>) -> Self {
        Self { nodes, key }
    }
}

impl<'t, K, V> Iterator for Find<'t, K, V>
where
    K: Ord,
{
    type Item = (&'t RangeInclusive<K>, &'t V);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let node = self.nodes.pop()?;

            if *self.key.start() > node.max {
                continue;
            }

            if let Some(left) = node.left.as_deref() {
                self.nodes.push(left);
            }

            if self.key.end() < node.key.start() {
                continue;
            }

            if let Some(right) = node.right.as_deref() {
                self.nodes.push(right);
            }

            if intersects(&self.key, &node.key) {
                return Some((&node.key, &node.value));
            }
        }
    }
}

fn intersects<K>(a: &RangeInclusive<K>, b: &RangeInclusive<K>) -> bool
where
    K: Ord,
{
    a.start() <= b.end() && b.start() <= a.end()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build_tree() -> IntervalTree<i32, i32> {
        //             15..=18
        //           /         \
        //     5..=8           17..=19
        //    /     \          /     \
        // 4..=8   7..=10  16..=22  21..=24
        let mut tree = IntervalTree::default();

        tree.insert(17..=19, 0);
        tree.insert(5..=8, 1);
        tree.insert(21..=24, 2);
        tree.insert(4..=8, 3);
        tree.insert(15..=18, 4);
        tree.insert(7..=10, 5);
        tree.insert(16..=22, 6);

        tree
    }

    #[test]
    fn test_find() {
        let tree = build_tree();
        let actual: Vec<_> = tree.find(7..=20).collect();

        let expected = [
            (&(15..=18), &4),
            (&(17..=19), &0),
            (&(16..=22), &6),
            (&(5..=8), &1),
            (&(7..=10), &5),
            (&(4..=8), &3),
        ];

        assert_eq!(actual, expected);
    }
}
