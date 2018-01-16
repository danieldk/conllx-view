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
    tree_cbs: Vec<Box<FnMut(Handle)>>,
    sent_cbs: Vec<Box<FnMut(String)>>,
}

impl StatefulTreebankModel {
    pub fn from_iter<I>(iter: I) -> Self
    where
        I: IntoIterator<Item = DependencyGraph>,
    {
        StatefulTreebankModel {
            inner: TreebankModel::from_iter(iter),
            idx: 0,
            tree_cbs: Vec::new(),
            sent_cbs: Vec::new(),
        }
    }

    pub fn add_sentence_callback<F>(&mut self, cb: F)
    where
        F: 'static + FnMut(String),
    {
        self.sent_cbs.push(Box::new(cb));
    }

    pub fn add_tree_callback<F>(&mut self, cb: F)
    where
        F: 'static + FnMut(Handle),
    {
        self.tree_cbs.push(Box::new(cb));
    }

    fn sentence_callbacks(&mut self) {
        let sentence = self.sentence();

        for callback in &mut self.sent_cbs {
            (*callback)(sentence.clone())
        }
    }

    fn tree_callbacks(&mut self) {
        if let Ok(handle) = self.handle() {
            for callback in &mut self.tree_cbs {
                (*callback)(handle.clone())
            }
        }
    }

    pub fn handle(&self) -> Result<Handle> {
        self.inner.handle(self.idx)
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }

    pub fn next(&mut self) {
        if self.idx == self.inner.len() - 1 {
            return;
        }

        self.idx += 1;

        self.tree_callbacks();
        self.sentence_callbacks();
    }

    pub fn previous(&mut self) {
        if self.idx == 0 {
            return;
        }

        self.idx -= 1;

        self.tree_callbacks();
        self.sentence_callbacks();
    }

    pub fn sentence(&self) -> String {
        format!("Sentence {}", self.idx)
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
    s.as_ref()
        .chars()
        .flat_map(|c| c.escape_default())
        .collect()
}

fn graph_to_dot(graph: &DependencyGraph) -> Result<String> {
    let mut dot = String::new();

    dot.push_str("digraph deptree {\n");
    dot.push_str(r#"graph [charset = "UTF-8"]"#);
    dot.push_str(r#"node [shape=plaintext, height=0, width=0, fontsize=12, fontname="Helvetica"]"#);

    for node_idx in graph.0.node_indices() {
        writeln!(
            &mut dot,
            r#"n{}[label="{}"];"#,
            node_idx.index(),
            escape_str(graph.0[node_idx].token.form())
        )?;
    }

    dot.push_str(r##"edge [color="#4b0082", fontsize="8", fontname="Courier New"]"##);

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
