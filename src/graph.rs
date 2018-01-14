use std::borrow::Cow;

use conllx::{Sentence, Token};
use dot::{Edges, GraphWalk, Id, LabelText, Labeller, Nodes};
use petgraph::{Directed, Graph};
use petgraph::graph::{EdgeIndex, NodeIndex};

#[derive(Clone, Debug)]
pub struct DependencyNode {
    pub token: Token,
    pub offset: usize,
}

#[derive(Clone)]
pub struct DependencyGraph(pub Graph<DependencyNode, String, Directed>);

impl<'a> Labeller<'a, NodeIndex, EdgeIndex> for DependencyGraph {
    fn edge_label(&'a self, e: &EdgeIndex) -> LabelText<'a> {
        LabelText::LabelStr(Cow::Borrowed(&self.0[*e]))
    }

    fn graph_id(&'a self) -> Id<'a> {
        Id::new("deptree").expect("Incorrect identifier")
    }

    fn node_id(&'a self, n: &NodeIndex) -> Id<'a> {
        Id::new(format!("n{}", n.index())).expect("Incorrect identifier")
    }

    fn node_label(&'a self, n: &NodeIndex) -> LabelText<'a> {
        LabelText::LabelStr(Cow::Borrowed(self.0[*n].token.form()))
    }

    fn node_shape(&'a self, _node: &NodeIndex) -> Option<LabelText<'a>> {
        Some(LabelText::LabelStr("plaintext".into()))
    }
}

impl<'a> GraphWalk<'a, NodeIndex, EdgeIndex> for DependencyGraph {
    fn nodes(&self) -> Nodes<'a, NodeIndex> {
        let mut indices = Vec::new();

        for node_idx in self.0.node_indices() {
            indices.push(node_idx);
        }

        Cow::Owned(indices)
    }

    fn edges(&'a self) -> Edges<'a, EdgeIndex> {
        let mut indices = Vec::new();

        for edge_idx in self.0.edge_indices() {
            indices.push(edge_idx);
        }

        Cow::Owned(indices)
    }

    fn source(&'a self, edge: &EdgeIndex) -> NodeIndex {
        self.0.edge_endpoints(*edge).expect("Unknown edge").0
    }

    fn target(&'a self, edge: &EdgeIndex) -> NodeIndex {
        self.0.edge_endpoints(*edge).expect("Unknown edge").1
    }
}

pub fn sentence_to_graph(sentence: Sentence, projective: bool) -> DependencyGraph {
    let mut g = Graph::new();

    let nodes: Vec<_> = sentence
        .into_iter()
        .enumerate()
        .map(|(offset, token)| {
            g.add_node(DependencyNode {
                token: token.clone(),
                offset: offset,
            })
        })
        .collect();

    for (idx, node_idx) in nodes.iter().enumerate() {
        let head = if projective {
            g[*node_idx].token.p_head()
        } else {
            g[*node_idx].token.head()
        };

        let rel = if projective {
            g[*node_idx]
                .token
                .p_head_rel()
                .expect("Dependency relation missing")
                .to_owned()
        } else {
            g[*node_idx]
                .token
                .head_rel()
                .expect("Dependency relation missing")
                .to_owned()
        };

        let head = head.expect("Token does not have a head");

        if head != 0 {
            g.add_edge(nodes[head - 1], nodes[idx], rel);
        }
    }

    DependencyGraph(g)
}
