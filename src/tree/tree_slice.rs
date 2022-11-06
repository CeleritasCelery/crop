// use std::borrow::Cow;
use std::ops::Range;

// use super::node_internal::{NodeCoordinates, NodeDescendant};
use super::{Leaves, Metric, Node, Summarize};

#[derive(Debug, Clone)]
enum NodeOrSlicedLeaf<'a, const N: usize, L: Summarize> {
    /// No slicing was needed so we can reuse a reference to the original node.
    Whole(&'a Node<N, L>),

    /// We had to slice a leaf, getting an owned value.
    SlicedLeaf(super::Leaf<L>),
}

impl<'a, const N: usize, L: Summarize> NodeOrSlicedLeaf<'a, N, L> {
    fn summary(&self) -> &L::Summary {
        match self {
            Self::Whole(node) => node.summary(),
            Self::SlicedLeaf(leaf) => leaf.summary(),
        }
    }
}

#[derive(Clone)]
pub struct TreeSlice<'a, const FANOUT: usize, Leaf: Summarize> {
    nodes: Vec<NodeOrSlicedLeaf<'a, FANOUT, Leaf>>,
    summary: Leaf::Summary,
}

impl<'a, const FANOUT: usize, Leaf: Summarize> TreeSlice<'a, FANOUT, Leaf> {
    pub(super) fn empty() -> Self {
        Self { nodes: Vec::new(), summary: Leaf::Summary::default() }
        // let empty = Leaf::empty();

        // Self {
        //     summary: empty.summarize(),
        //     kind: SliceKind::SingleLeaf(Cow::Owned(empty)),
        // }
    }

    pub(super) fn from_range_in_node<M>(
        node: &'a Node<FANOUT, Leaf>,
        range: Range<M>,
    ) -> Self
    where
        M: Metric<Leaf>,
    {
        let (nodes, summary) = sumzing(node, range);
        Self { nodes, summary }
        // todo!()

        // match deepest_node_containing_range(node, range) {
        //     Deepest::Leaf(leaf) => Self {
        //         summary: leaf.summarize(),
        //         kind: SliceKind::SingleLeaf(leaf),
        //     },

        //     Deepest::Inode(inode, range) => {
        //         // let (start, end, summary) =
        //         let (nodes, summary) =
        //             inode_get_nodes_summary_in_range(inode, range);

        //         Self {
        //             // kind: SliceKind::SubTree { inode, start, end },
        //             kind: SliceKind::SubTree(nodes),
        //             summary,
        //         }
        //     },
        // }
    }

    /// TODO: docs
    pub fn leaves(&self) -> Leaves<'_, Leaf> {
        todo!()
    }

    /// TODO: docs
    pub fn slice<M>(&self, range: Range<M>) -> TreeSlice<'a, FANOUT, Leaf>
    where
        M: Metric<Leaf>,
    {
        assert!(M::zero() <= range.start);
        assert!(range.start <= range.end);
        assert!(range.end <= M::measure(&self.summary));

        if range.start == range.end {
            return Self::empty();
        }

        if M::measure(&self.summary) == range.end - range.start {
            return self.clone();
        }

        // TODO: don't clone
        let (nodes, summary) = sumzong(self.nodes.clone(), range);
        Self { nodes, summary }

        // todo!()

        // match &self.kind {
        //     SliceKind::SingleLeaf(leaf) => {
        //         let value = M::slice(&*leaf, range);
        //         Self {
        //             summary: value.summarize(),
        //             kind: SliceKind::SingleLeaf(Cow::Owned(value)),
        //         }
        //     },

        //     SliceKind::SubTree(vec) => {
        //         let (sliced, summary) = slice_vec_nodes(&vec, range);
        //         Self { kind: SliceKind::SubTree(sliced), summary }
        //     },
        // }
    }

    /// TODO: docs
    pub fn summary(&self) -> &Leaf::Summary {
        &self.summary
    }
}

fn sumzing<'a, const N: usize, L, M>(
    mut node: &'a Node<N, L>,
    mut range: Range<M>,
) -> (Vec<NodeOrSlicedLeaf<'a, N, L>>, L::Summary)
where
    L: Summarize,
    M: Metric<L>,
{
    let zero = M::zero();

    assert!(zero <= range.start);
    assert!(range.start <= range.end);
    assert!(range.end <= M::measure(node.summary()));

    'outer: loop {
        match node {
            Node::Leaf(leaf) => {
                // If the leaf's size is perfectly equal to the width of the
                // range we return the leaf's value. If not then the range is
                // strictly smaller than the leaf and the latter *must* be
                // sliceable by `M`.

                // TODO: this should be handled in the previous iteration.
                if M::measure(leaf.summary()) == range.end - range.start {
                    return (
                        vec![NodeOrSlicedLeaf::Whole(node)],
                        leaf.summary().clone(),
                    );
                } else {
                    let sliced = M::slice(leaf.value(), range);
                    let summary = sliced.summarize();
                    return (
                        vec![NodeOrSlicedLeaf::SlicedLeaf(
                            super::Leaf::from_value(sliced),
                        )],
                        summary,
                    );
                }
            },

            Node::Internal(inode) => {
                let mut measured = zero;
                for child in inode.children() {
                    let size = M::measure(child.summary());
                    if range.start >= measured && range.end <= measured + size
                    {
                        range.start -= measured;
                        range.end -= measured;
                        node = &*child;
                        continue 'outer;
                    } else {
                        measured += size;
                    }
                }
                // If none of the inode's children fully contained the range
                // then the inode itself must be the deepest node that fully
                // contains the range, so we're done.
                let nodes = inode
                    .children()
                    .iter()
                    .map(|n| NodeOrSlicedLeaf::Whole(&**n));

                return sumzong(nodes, range);
            },
        }
    }
}

fn sumzong<'a, const N: usize, I, L, M>(
    nodes: I,
    range: Range<M>,
) -> (Vec<NodeOrSlicedLeaf<'a, N, L>>, L::Summary)
where
    M: Metric<L>,
    L: Summarize,
    I: IntoIterator<Item = NodeOrSlicedLeaf<'a, N, L>>,
{
    let mut iter = nodes.into_iter();
    let mut measured = M::zero();

    let mut nodes = Vec::new();
    let mut summary = L::Summary::default();

    while let Some(node) = iter.next() {
        let size = M::measure(node.summary());
        if measured + size > range.start {
            nodes_from_start(
                node,
                range.start - measured,
                &mut nodes,
                &mut summary,
                &mut M::zero(),
                &mut false,
            );
            measured += size;
            break;
        } else {
            measured += size;
        }
    }

    while let Some(node) = iter.next() {
        let size = M::measure(node.summary());
        if measured + size >= range.end {
            nodes_to_end(
                node,
                range.end - measured,
                &mut nodes,
                &mut summary,
                &mut M::zero(),
                &mut false,
            );
            break;
        } else {
            summary += node.summary();
            nodes.push(node);
            measured += size;
        }
    }

    (nodes, summary)
}

fn nodes_from_start<'a, const N: usize, L, M>(
    node: NodeOrSlicedLeaf<'a, N, L>,
    start: M,
    vec: &mut Vec<NodeOrSlicedLeaf<'a, N, L>>,
    summary: &mut L::Summary,
    measured: &mut M,
    found_start: &mut bool,
) where
    L: Summarize,
    M: Metric<L>,
{
    match node {
        NodeOrSlicedLeaf::Whole(Node::Leaf(leaf)) => {
            let start = start - *measured;
            let end = M::measure(leaf.summary()); // TODO: remove this
            let sliced = M::slice(leaf.value(), start..end);
            *summary += &sliced.summarize();
            vec.push(NodeOrSlicedLeaf::SlicedLeaf(super::Leaf::from_value(
                sliced, // TODO: don't `summarize` twice
            )));
            *found_start = true;
            return;
        },

        NodeOrSlicedLeaf::SlicedLeaf(ref leaf) => {
            let start = start - *measured;
            let end = M::measure(leaf.summary()); // TODO: remove this
            let sliced = M::slice(leaf.value(), start..end);
            *summary += &sliced.summarize();
            vec.push(NodeOrSlicedLeaf::SlicedLeaf(super::Leaf::from_value(
                sliced, // TODO: don't `summarize` twice
            )));
            *found_start = true;
            return;
        },

        NodeOrSlicedLeaf::Whole(Node::Internal(inode)) => {
            for child in
                inode.children().iter().map(|n| NodeOrSlicedLeaf::Whole(&**n))
            {
                if *found_start {
                    *summary += child.summary();
                    vec.push(child);
                    continue;
                }
                if start == *measured {
                    *summary += child.summary();
                    vec.push(child);
                    *found_start = true;
                    continue;
                }
                let size = M::measure(child.summary());
                if *measured + size > start {
                    nodes_from_start(
                        child,
                        start,
                        vec,
                        summary,
                        measured,
                        found_start,
                    );
                } else {
                    *measured += size;
                }
            }
        },
    };
}

fn nodes_to_end<'a, const N: usize, L, M>(
    node: NodeOrSlicedLeaf<'a, N, L>,
    end: M,
    vec: &mut Vec<NodeOrSlicedLeaf<'a, N, L>>,
    summary: &mut L::Summary,
    measured: &mut M,
    found_end: &mut bool,
) where
    L: Summarize,
    M: Metric<L>,
{
    match node {
        NodeOrSlicedLeaf::Whole(Node::Leaf(leaf)) => {
            let start = M::zero(); // TODO: remove this
            let end = end - *measured;
            let sliced = M::slice(leaf.value(), start..end);
            *summary += &sliced.summarize();
            vec.push(NodeOrSlicedLeaf::SlicedLeaf(super::Leaf::from_value(
                sliced, // TODO: don't `summarize` twice
            )));
            *found_end = true;
            return;
        },

        NodeOrSlicedLeaf::SlicedLeaf(ref leaf) => {
            let start = M::zero(); // TODO: remove this
            let end = end - *measured;
            let sliced = M::slice(leaf.value(), start..end);
            *summary += &sliced.summarize();
            vec.push(NodeOrSlicedLeaf::SlicedLeaf(super::Leaf::from_value(
                sliced, // TODO: don't `summarize` twice
            )));
            *found_end = true;
            return;
        },

        NodeOrSlicedLeaf::Whole(Node::Internal(inode)) => {
            for child in
                inode.children().iter().map(|n| NodeOrSlicedLeaf::Whole(&**n))
            {
                if *found_end {
                    return;
                }
                let size = M::measure(child.summary());
                if end == *measured + size {
                    *summary += child.summary();
                    vec.push(child);
                    *found_end = true;
                    return;
                }
                if *measured + size >= end {
                    nodes_to_end(
                        child, end, vec, summary, measured, found_end,
                    );
                } else {
                    *summary += child.summary();
                    vec.push(child);
                    *measured += size;
                }
            }
        },
    }
}
/*























































*/

//#[derive(Clone)]
//pub struct TreeSlice<'a, const FANOUT: usize, Leaf: Summarize> {
//    kind: SliceKind<'a, FANOUT, Leaf>,
//    summary: Leaf::Summary,
//}

//#[derive(Clone)]
//enum SliceKind<'a, const N: usize, Leaf: Summarize> {
//    /// The slice is fully contained in a single leaf.
//    SingleLeaf(Cow<'a, Leaf>),

//    // TODO: if we end up going w/ this we can re-use this representation for
//    // the single-leaf case as well, no need to have this enum. Also, since we
//    // no longer need the `Cow` on the `Leaf` we can remove the `Clone`
//    // boundary on `Summarize`.
//    /// The slice spans multiple leaves.
//    SubTree(Vec<NodeOrSlicedLeaf<'a, N, Leaf>>),
//    // /// The slice spans multiple leaves.
//    // _SubTree {
//    //     // /// The deepest internal node in the tree that still fully contains the
//    //     // /// interval from which this slice was derived.
//    //     // inode: &'a Inode<N, Leaf>,

//    //     // /// TODO: docs
//    //     // start: NodeDescendant<'a, N, Leaf>,

//    //     // /// TODO: docs
//    //     // end: NodeDescendant<'a, N, Leaf>,
//    // },
//}

///// Enum returned by [`deepest_node_containing_range`].
//enum Deepest<'a, const N: usize, L: Summarize, M: Metric<L>> {
//    /// The deepest node containing the range is a leaf. If the range and the
//    /// leaf are exactly the same size there's no need to slice the leaf and we
//    /// can use a `Cow::Borrowed`. However if the range is smaller that the
//    /// leaf we have to slice it and use an owned value. In that case we use a
//    /// `Cow::Owned`.
//    Leaf(Cow<'a, L>),

//    /// The deepest node containing the range is an internal node. Even though
//    /// we store it as a `Node`, it is guaranteed to be of the `Node::Internal`
//    /// variant. We store it as a `Node` because this value will be passed to
//    /// [`inode_get_summary_start_end_in_range`].
//    ///
//    /// The second item in the tuple is the original range passed to
//    /// [`deepest_node_containing_range`] but shifted to be a valid range of
//    /// the returned inode. In particular, the start and end of the range are
//    /// always less then or equal to the ones of the original range, but
//    /// shifted of the same amount so that this range has the exact same width
//    /// as the original one.
//    Inode(&'a Inode<N, L>, Range<M>),
//}

///// TODO: docs
//fn deepest_node_containing_range<const N: usize, L, M>(
//    mut node: &Node<N, L>,
//    mut range: Range<M>,
//) -> Deepest<'_, N, L, M>
//where
//    L: Summarize,
//    M: Metric<L>,
//{
//    let zero = M::zero();

//    assert!(zero <= range.start);
//    assert!(range.start <= range.end);
//    assert!(range.end <= M::measure(node.summary()));

//    'outer: loop {
//        match node {
//            Node::Leaf(leaf) => {
//                // If the leaf's size is perfectly equal to the width of the
//                // range we return the leaf's value. If not then the range is
//                // strictly smaller than the leaf and the latter *must* be
//                // sliceable by `M`.
//                let value =
//                    if M::measure(leaf.summary()) == range.end - range.start {
//                        Cow::Borrowed(leaf.value())
//                    } else {
//                        Cow::Owned(M::slice(leaf.value(), range))
//                    };
//                return Deepest::Leaf(value);
//            },

//            Node::Internal(inode) => {
//                let mut measured = zero;
//                for child in inode.children() {
//                    let size = M::measure(child.summary());
//                    if range.start >= measured && range.end <= measured + size
//                    {
//                        range.start -= measured;
//                        range.end -= measured;
//                        node = &*child;
//                        continue 'outer;
//                    } else {
//                        measured += size;
//                    }
//                }
//                // If none of the inode's children fully contained the range
//                // then the inode itself must be the deepest node that fully
//                // contains the range, so we're done.
//                return Deepest::Inode(inode, range);
//            },
//        }
//    }
//}

//fn something<'a, const N: usize, I, L, M>(
//    nodes: I,
//    range: Range<M>,
//) -> (Vec<NodeOrSlicedLeaf<'a, N, L>>, L::Summary)
//where
//    M: Metric<L>,
//    L: Summarize,
//    I: IntoIterator<Item = NodeOrSlicedLeaf<'a, N, L>>,
//{
//    let mut iter = nodes.into_iter();
//    let mut measured = M::zero();

//    let mut nodes = Vec::new();
//    let mut summary = L::Summary::default();

//    while let Some(node) = iter.next() {
//        let size = M::measure(node.summary());
//        if measured + size > range.start {
//            nodes_from_start(
//                node,
//                range.start - measured,
//                &mut nodes,
//                &mut summary,
//                &mut M::zero(),
//                &mut false,
//            );
//            measured += size;
//            break;
//        } else {
//            measured += size;
//        }
//    }

//    while let Some(node) = iter.next() {
//        let size = M::measure(node.summary());
//        if measured + size >= range.end {
//            nodes_to_end(
//                node,
//                range.end - measured,
//                &mut nodes,
//                &mut summary,
//                &mut M::zero(),
//                &mut false,
//            );
//            break;
//        } else {
//            summary += node.summary();
//            nodes.push(node);
//            measured += size;
//        }
//    }

//    (nodes, summary)
//}

//fn inode_get_nodes_summary_in_range<const N: usize, L, M>(
//    inode: &Inode<N, L>,
//    range: Range<M>,
//) -> (Vec<NodeOrSlicedLeaf<'_, N, L>>, L::Summary)
//// ) -> (NodeDescendant<'_, N, L>, NodeDescendant<'_, N, L>, L::Summary)
//where
//    L: Summarize,
//    M: Metric<L>,
//{
//    // let mut start_descendant = None;
//    // let mut end_descendant = None;

//    let mut nodes = Vec::new();
//    let mut summary = L::Summary::default();

//    let mut measured = M::zero();
//    let mut iter = inode.children().iter().enumerate();

//    // Loop until we find the child containing the start of the range.
//    while let Some((i, child)) = iter.next() {
//        let size = M::measure(child.summary());
//        if measured + size > range.start {
//            nodes_from_start_rec(
//                child,
//                range.start - measured,
//                &mut nodes,
//                &mut summary,
//                &mut M::zero(),
//                &mut false,
//            );
//            // let (start, summ) = todo_start(
//            //     child,
//            //     NodeCoordinates::<N>::init(i),
//            //     range.start - measured,
//            // );
//            // start_descendant = Some(start);
//            // summary += &summ;
//            measured += size;
//            break;
//        } else {
//            measured += size;
//        }
//    }

//    // Loop until we find the child containing the end of the range.
//    while let Some((i, child)) = iter.next() {
//        let size = M::measure(child.summary());
//        if measured + size >= range.end {
//            nodes_to_end_rec(
//                child,
//                range.end - measured,
//                &mut nodes,
//                &mut summary,
//                &mut M::zero(),
//                &mut false,
//            );
//            // let (end, summ) = todo_end(
//            //     child,
//            //     NodeCoordinates::<N>::init(i),
//            //     range.end - measured,
//            // );
//            // end_descendant = Some(end);
//            // summary += &summ;
//            break;
//        } else {
//            summary += child.summary();
//            nodes.push(NodeOrSlicedLeaf::Whole(&**child));
//            measured += size;
//        }
//    }

//    // (start_descendant.unwrap(), end_descendant.unwrap(), summary)
//    (nodes, summary)
//}

//// fn nodes_from_start<'a, const N: usize, L, M>(
////     root: &'a Node<N, L>,
////     start: M,
//// ) -> (Vec<NodeOrSlicedLeaf<'a, N, L>>, L::Summary)
//// where
////     L: Summarize,
////     M: Metric<L>,
//// {
////     let mut nodes = Vec::new();
////     let mut summary = L::Summary::default();
////     nodes_from_start_rec(
////         root,
////         start,
////         &mut nodes,
////         &mut summary,
////         &mut M::zero(),
////         &mut false,
////     );
////     (nodes, summary)
//// }

//// fn nodes_to_end<'a, const N: usize, L, M>(
////     root: &'a Node<N, L>,
////     start: M,
//// ) -> (Vec<NodeOrSlicedLeaf<'a, N, L>>, L::Summary)
//// where
////     L: Summarize,
////     M: Metric<L>,
//// {
////     let mut nodes = Vec::new();
////     let mut summary = L::Summary::default();
////     nodes_to_end_rec(
////         root,
////         start,
////         &mut nodes,
////         &mut summary,
////         &mut M::zero(),
////         &mut false,
////     );
////     (nodes, summary)
//// }

//fn nodes_from_start_rec<'a, const N: usize, L, M>(
//    node: &'a Node<N, L>,
//    start: M,
//    vec: &mut Vec<NodeOrSlicedLeaf<'a, N, L>>,
//    summary: &mut L::Summary,
//    measured: &mut M,
//    found_start: &mut bool,
//) where
//    L: Summarize,
//    M: Metric<L>,
//{
//    match node {
//        Node::Leaf(leaf) => {
//            let start = start - *measured;
//            let end = M::measure(leaf.summary()); // TODO: remove this
//            let sliced = M::slice(leaf.value(), start..end);
//            *summary += &sliced.summarize();
//            vec.push(NodeOrSlicedLeaf::SlicedLeaf(sliced));
//            *found_start = true;
//            return;
//        },

//        Node::Internal(inode) => {
//            for child in inode.children() {
//                if *found_start {
//                    *summary += child.summary();
//                    vec.push(NodeOrSlicedLeaf::Whole(&**child));
//                    continue;
//                }
//                if start == *measured {
//                    *summary += child.summary();
//                    vec.push(NodeOrSlicedLeaf::Whole(&**child));
//                    *found_start = true;
//                    continue;
//                }
//                let size = M::measure(child.summary());
//                if *measured + size > start {
//                    nodes_from_start_rec(
//                        child,
//                        start,
//                        vec,
//                        summary,
//                        measured,
//                        found_start,
//                    );
//                } else {
//                    *measured += size;
//                }
//            }
//        },
//    }
//}

//fn nodes_to_end_rec<'a, const N: usize, L, M>(
//    node: &'a Node<N, L>,
//    end: M,
//    vec: &mut Vec<NodeOrSlicedLeaf<'a, N, L>>,
//    summary: &mut L::Summary,
//    measured: &mut M,
//    found_end: &mut bool,
//) where
//    L: Summarize,
//    M: Metric<L>,
//{
//    match node {
//        Node::Leaf(leaf) => {
//            let start = M::zero(); // TODO: remove this
//            let end = end - *measured;
//            let sliced = M::slice(leaf.value(), start..end);
//            *summary += &sliced.summarize();
//            vec.push(NodeOrSlicedLeaf::SlicedLeaf(sliced));
//            *found_end = true;
//            return;
//        },

//        Node::Internal(inode) => {
//            for child in inode.children() {
//                if *found_end {
//                    return;
//                }
//                let size = M::measure(child.summary());
//                if end == *measured + size {
//                    *summary += child.summary();
//                    vec.push(NodeOrSlicedLeaf::Whole(&**child));
//                    *found_end = true;
//                    return;
//                }
//                if *measured + size >= end {
//                    nodes_to_end_rec(
//                        child, end, vec, summary, measured, found_end,
//                    );
//                } else {
//                    *summary += child.summary();
//                    vec.push(NodeOrSlicedLeaf::Whole(&**child));
//                    *measured += size;
//                }
//            }
//        },
//    }
//}

//fn slice_vec_nodes<'a, const N: usize, L, M>(
//    orig: &[NodeOrSlicedLeaf<N, L>],
//    range: Range<M>,
//) -> (Vec<NodeOrSlicedLeaf<'a, N, L>>, L::Summary)
//where
//    L: Summarize,
//    M: Metric<L>,
//{
//    let mut sliced = Vec::new();
//    let mut summary = L::Summary::default();

//    // TODO: logic
//    //

//    let mut measured = M::zero();
//    let mut iter = orig.iter();

//    while let Some(node) = iter.next() {
//        let ciao = match node {
//            NodeOrSlicedLeaf::Whole(n) => *n,
//            NodeOrSlicedLeaf::SlicedLeaf(l) => {
//                &Node::Leaf(super::Leaf::from_value(l.clone()))
//            },
//        };

//        let size = M::measure(ciao.summary());
//        if measured + size > range.start {
//            nodes_from_start_rec(
//                ciao,
//                range.start - measured,
//                &mut nodes,
//                &mut summary,
//                &mut M::zero(),
//                &mut false,
//            );
//            measured += size;
//            break;
//        } else {
//            measured += size;
//        }
//    }

//    // Loop until we find the child containing the end of the range.
//    while let Some((i, child)) = iter.next() {
//        let size = M::measure(child.summary());
//        if measured + size >= range.end {
//            nodes_to_end_rec(
//                child,
//                range.end - measured,
//                &mut nodes,
//                &mut summary,
//                &mut M::zero(),
//                &mut false,
//            );
//            break;
//        } else {
//            summary += child.summary();
//            nodes.push(NodeOrSlicedLeaf::Whole(&**child));
//            measured += size;
//        }
//    }

//    (sliced, summary)
//}

////fn todo_start<'a, const N: usize, L, M>(
////    node: &'a Node<N, L>,
////    mut coordinate: NodeCoordinates<'a, N>,
////    start: M,
////) -> (NodeDescendant<'_, N, L>, L::Summary)
////where
////    L: Summarize,
////    M: Metric<L>,
////{
////    println!("{node:#?}");
////    println!("{coordinate:?}");
////    println!("start: {start:?}");

////    match node {
////        Node::Leaf(leaf) => {
////            // let start = range.start - measured;
////            // let sliced = M::slice(leaf.value(), start..);
////            // summary += &sliced.summarize();
////            // return;
////        },

////        Node::Internal(inode) => {
////            for (idx, child) in inode.children().iter().enumerate() {
////                let size = M::measure(child.summary());
////            }
////        },
////    }
////    (NodeDescendant::Whole(coordinate), L::Summary::default())
////}

////fn todo_end<'a, const N: usize, L, M>(
////    node: &'a Node<N, L>,
////    mut coordinate: NodeCoordinates<'a, N>,
////    end: M,
////) -> (NodeDescendant<'_, N, L>, L::Summary)
////where
////    L: Summarize,
////    M: Metric<L>,
////{
////    println!("{node:#?}");
////    println!("{coordinate:?}");
////    println!("end: {end:?}");

////    (NodeDescendant::Whole(coordinate), L::Summary::default())
////}
