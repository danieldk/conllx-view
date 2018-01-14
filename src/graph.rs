use conllx::{Sentence, Token};
use petgraph::{Directed, Graph};

#[derive(Clone, Debug)]
pub struct DependencyNode {
    pub token: Token,
    pub offset: usize,
}

#[derive(Clone)]
pub struct DependencyGraph(pub Graph<DependencyNode, String, Directed>);

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
