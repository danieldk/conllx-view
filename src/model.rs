use std::io::{Read, Write};
use std::iter::FromIterator;
use std::process::{Command, Stdio};

use rsvg::Handle;

use error::Result;
use graph::{DependencyGraph, Dot};

pub struct StatefulTreebankModel {
    inner: TreebankModel,
    idx: usize,
    callbacks: Vec<Box<Fn(&StatefulTreebankModel)>>,
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

    pub fn graph(&self) -> &DependencyGraph {
        self.inner
            .graph(self.idx)
            .expect("Stateful model has invalid index")
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

    pub fn graph(&self, idx: usize) -> Option<&DependencyGraph> {
        self.treebank.get(idx)
    }

    pub fn handle(&self, idx: usize) -> Result<Handle> {
        let svg = self.svg(idx)?;
        Ok(Handle::new_from_data(svg.as_bytes())?)
    }

    pub fn len(&self) -> usize {
        self.treebank.len()
    }

    fn svg(&self, idx: usize) -> Result<String> {
        let dot = self.treebank[idx].dot()?;
        dot_to_svg(&dot)
    }
}

impl From<Vec<DependencyGraph>> for TreebankModel {
    fn from(vec: Vec<DependencyGraph>) -> Self {
        TreebankModel { treebank: vec }
    }
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
