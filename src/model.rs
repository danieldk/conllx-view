use std::fmt::Write as FmtWrite;
use std::io::{Read, Write};
use std::iter::FromIterator;
use std::process::{Command, Stdio};

use rsvg::Handle;

use error::Result;
use graph::DependencyGraph;

pub struct StatefulTreebankModel {
    inner: TreebankModel,
    idx: usize,
    callbacks: Vec<Box<Fn(&StatefulTreebankModel)>>,
}

pub trait DependencyTreeDot {
    fn dependency_tree_dot(&self) -> Result<String>;
}

impl StatefulTreebankModel {
    pub fn from_iter<I>(iter: I) -> Self
    where
        I: IntoIterator<Item = DependencyGraph>,
    {
        StatefulTreebankModel {
            inner: TreebankModel::from_iter(iter),
            idx: 0,
            callbacks: Vec::new(),
        }
    }

    fn callbacks(&mut self) {
        for callback in &self.callbacks {
            (*callback)(&self)
        }
    }

    pub fn connect_update<F>(&mut self, callback: F)
    where
        F: 'static + Fn(&StatefulTreebankModel),
    {
        self.callbacks.push(Box::new(callback));
    }

    pub fn first(&mut self) {
        self.set_idx(0);
    }

    pub fn handle(&self) -> Result<Handle> {
        self.inner.handle(self.idx)
    }

    pub fn idx(&self) -> usize {
        self.idx
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }

    pub fn next(&mut self) {
        let idx = self.idx;
        self.set_idx(idx + 1);
    }

    pub fn previous(&mut self) {
        let idx = self.idx;
        self.set_idx(idx - 1);
    }

    fn set_idx(&mut self, idx: usize) {
        if idx < self.len() {
            self.idx = idx;
        }

        self.callbacks();
    }

    pub fn tokens(&self) -> Vec<&str> {
        self.inner.tokens(self.idx)
    }
}

impl DependencyTreeDot for StatefulTreebankModel {
    fn dependency_tree_dot(&self) -> Result<String> {
        graph_to_dot(&self.inner.treebank[self.idx])
    }
}

pub struct TreebankModel {
    treebank: Vec<DependencyGraph>,
}

impl TreebankModel {
    pub fn from_iter<I>(iter: I) -> Self
    where
        I: IntoIterator<Item = DependencyGraph>,
    {
        TreebankModel {
            treebank: Vec::from_iter(iter),
        }
    }

    pub fn handle(&self, idx: usize) -> Result<Handle> {
        let svg = self.svg(idx)?;
        Ok(Handle::new_from_data(svg.as_bytes())?)
    }

    pub fn len(&self) -> usize {
        self.treebank.len()
    }

    fn svg(&self, idx: usize) -> Result<String> {
        let dot = graph_to_dot(&self.treebank[idx])?;
        dot_to_svg(&dot)
    }

    pub fn tokens(&self, idx: usize) -> Vec<&str> {
        let graph = &self.treebank[idx].0;

        let mut tokens = Vec::new();
        for node_idx in graph.node_indices() {
            tokens.push(graph[node_idx].token.form());
        }

        tokens
    }
}

impl From<Vec<DependencyGraph>> for TreebankModel {
    fn from(vec: Vec<DependencyGraph>) -> Self {
        TreebankModel { treebank: vec }
    }
}

fn escape_str<S>(s: S) -> String
where
    S: AsRef<str>,
{
    s.as_ref().replace('"', r#"\""#)
}

fn graph_to_dot(graph: &DependencyGraph) -> Result<String> {
    let mut dot = String::new();

    dot.push_str("digraph deptree {\n");
    dot.push_str("graph [charset = \"UTF-8\"]\n");
    dot.push_str(
        "node [shape=plaintext, height=0, width=0, fontsize=12, fontname=\"Helvetica\"]\n",
    );

    for node_idx in graph.0.node_indices() {
        writeln!(
            &mut dot,
            r#"n{}[label="{}"];"#,
            node_idx.index(),
            escape_str(graph.0[node_idx].token.form())
        )?;
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

fn dot_to_svg(dot: &str) -> Result<String> {
    // FIXME: bind against C library?

    // Spawn Graphviz dot for rendering SVG (Fixme: bind against C library?).
    let process = Command::new("dot")
        .arg("-Tsvg")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()?;

    process.stdin.unwrap().write_all(dot.as_bytes())?;

    let mut svg = String::new();
    process.stdout.unwrap().read_to_string(&mut svg)?;

    Ok(svg)
}
