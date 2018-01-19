use conllx::{Sentence, Token};
use petgraph::{Directed, Graph};

#[derive(Clone, Debug)]
pub struct DependencyNode {
    pub token: Token,
    pub offset: usize,
}

#[derive(Clone)]
pub struct DependencyGraph(pub Graph<DependencyNode, String, Directed>);

impl From<Sentence> for DependencyGraph {
    fn from(sentence: Sentence) -> Self {
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
            let head = g[*node_idx].token.head();

            let rel = g[*node_idx]
                .token
                .head_rel()
                .expect("Dependency relation missing")
                .to_owned();

            let head = head.expect("Token does not have a head");

            if head != 0 {
                g.add_edge(nodes[head - 1], nodes[idx], rel);
            }
        }

        DependencyGraph(g)
    }
}
