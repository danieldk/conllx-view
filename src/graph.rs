use std::fmt::Write as FmtWrite;
use std::io::{Read, Write};
use std::process::{Command, Stdio};

use conllx::{Features, Sentence, Token};
use failure::{Error, ResultExt};
use itertools::Itertools;
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

pub trait Dot {
    fn dot(&self) -> Result<String, Error>;
}

impl Dot for DependencyGraph {
    fn dot(&self) -> Result<String, Error> {
        graph_to_dot(self)
    }
}

pub trait Tikz {
    fn tikz(&self) -> Result<String, Error>;
}

impl Tikz for DependencyGraph {
    fn tikz(&self) -> Result<String, Error> {
        graph_to_tikz(self)
    }
}

pub trait Tokens {
    fn tokens(&self) -> Vec<&str>;
}

impl Tokens for DependencyGraph {
    fn tokens(&self) -> Vec<&str> {
        let mut tokens = Vec::new();
        for node_idx in self.0.node_indices() {
            tokens.push(self.0[node_idx].token.form());
        }

        tokens
    }
}

pub trait Svg {
    fn svg(&self) -> Result<String, Error>;
}

impl Svg for DependencyGraph {
    fn svg(&self) -> Result<String, Error> {
        let dot = self.dot()?;
        dot_to_svg(&dot)
    }
}

fn dot_to_svg(dot: &str) -> Result<String, Error> {
    // FIXME: bind against C library?

    // Spawn Graphviz dot for rendering SVG (Fixme: bind against C library?).
    let process = Command::new("dot")
        .arg("-Tsvg")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .context("Could not start Graphviz dot")?;

    process
        .stdin
        .unwrap()
        .write_all(dot.as_bytes())
        .context("Could not write graph to dot stdin")?;

    let mut svg = String::new();
    process
        .stdout
        .unwrap()
        .read_to_string(&mut svg)
        .context("Could not read graph SVG from dot stdout")?;

    Ok(svg)
}

fn escape_str<S>(s: S) -> String
where
    S: AsRef<str>,
{
    s.as_ref().replace('"', r#"\""#)
}

fn graph_to_dot(graph: &DependencyGraph) -> Result<String, Error> {
    let mut dot = String::new();

    dot.push_str("digraph deptree {\n");
    dot.push_str("graph [charset = \"UTF-8\"]\n");
    dot.push_str(
        "node [shape=plaintext, height=0, width=0, fontsize=12, fontname=\"Helvetica\"]\n",
    );

    for node_idx in graph.0.node_indices() {
        let marked = graph.0[node_idx]
            .token
            .features()
            .map(Features::as_map)
            .map(|m| m.contains_key("mark"))
            .unwrap_or(false);

        if marked {
            writeln!(
                &mut dot,
                r#"n{}[label="{}", fontcolor="firebrick3"];"#,
                node_idx.index(),
                escape_str(graph.0[node_idx].token.form())
            )?;
        } else {
            writeln!(
                &mut dot,
                r#"n{}[label="{}"];"#,
                node_idx.index(),
                escape_str(graph.0[node_idx].token.form())
            )?;
        }
    }

    dot.push_str("edge [color=\"#4b0082\", fontsize=\"8\", fontname=\"Courier New\"]\n");

    for edge_idx in graph.0.edge_indices() {
        let weight = &graph.0[edge_idx];
        let (source, target) = graph.0.edge_endpoints(edge_idx).unwrap();

        writeln!(
            &mut dot,
            r#"n{} -> n{}[label="{}"];"#,
            source.index(),
            target.index(),
            escape_str(weight)
        )?;
    }

    dot.push_str("}");

    Ok(dot)
}

fn graph_to_tikz(graph: &DependencyGraph) -> Result<String, Error> {
    let mut dot = String::new();

    dot.push_str("\\documentclass{standalone}\n\n");
    dot.push_str("\\usepackage{tikz-dependency}\n\n");
    dot.push_str("\\begin{document}\n\n");
    dot.push_str("\\begin{dependency}\n");
    dot.push_str("\\begin{deptext}");

    dot.push_str(&graph
        .0
        .node_indices()
        .map(|idx| {
            let marked = graph.0[idx]
                .token
                .features()
                .map(Features::as_map)
                .map(|m| m.contains_key("mark"))
                .unwrap_or(false);

            if marked {
                format!("\\underline{{{}}}", graph.0[idx].token.form())
            } else {
                graph.0[idx].token.form().to_owned()
            }
        })
        .join(" \\& "));

    dot.push_str("\\\\\n\\end{deptext}\n");

    for edge_idx in graph.0.edge_indices() {
        let weight = &graph.0[edge_idx];
        let (source, target) = graph.0.edge_endpoints(edge_idx).unwrap();

        writeln!(
            &mut dot,
            "\\depedge{{{}}}{{{}}}{{{}}}",
            source.index() + 1,
            target.index() + 1,
            escape_str(weight)
        )?;
    }

    dot.push_str("\\end{dependency}\n\n");
    dot.push_str("\\end{document}");

    Ok(dot)
}
