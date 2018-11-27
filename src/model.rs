use std::iter::FromIterator;

use conllx::graph::Sentence;
use enum_map::EnumMap;

#[derive(EnumMap)]
pub enum ModelUpdate {
    Any,
    TreeSelection,
    TreebankLen,
}

pub struct StatefulTreebankModel {
    inner: TreebankModel,
    idx: usize,
    callbacks: EnumMap<ModelUpdate, Vec<Box<Fn(&StatefulTreebankModel) + Send + 'static>>>,
}

impl StatefulTreebankModel {
    pub fn new() -> Self {
        StatefulTreebankModel {
            inner: TreebankModel::new(),
            idx: 0,
            callbacks: EnumMap::new(),
        }
    }

    #[allow(dead_code)]
    pub fn from_iter<I>(iter: I) -> Self
    where
        I: IntoIterator<Item = Sentence>,
    {
        StatefulTreebankModel {
            inner: TreebankModel::from_iter(iter),
            idx: 0,
            callbacks: EnumMap::new(),
        }
    }

    fn callbacks(&mut self, update: ModelUpdate) {
        for callback in &self.callbacks[update] {
            (*callback)(&self)
        }

        for callback in &self.callbacks[ModelUpdate::Any] {
            (*callback)(&self)
        }
    }

    pub fn connect_update<F>(&mut self, update: ModelUpdate, callback: F)
    where
        F: 'static + Fn(&StatefulTreebankModel) + Send,
    {
        self.callbacks[update].push(Box::new(callback));
    }

    pub fn first(&mut self) {
        self.set_idx(0);
    }

    /// Return the current dependency graph. Returns `None` when the
    /// treebank is currently empty.
    pub fn graph(&self) -> Option<&Sentence> {
        self.inner.graph(self.idx)
    }

    pub fn idx(&self) -> usize {
        self.idx
    }

    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
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

    pub fn push(&mut self, graph: Sentence) {
        let first = self.is_empty();

        self.inner.push(graph);

        self.callbacks(ModelUpdate::TreebankLen);

        if first {
            self.callbacks(ModelUpdate::TreeSelection);
        }
    }

    fn set_idx(&mut self, idx: usize) {
        if idx < self.len() {
            self.idx = idx;
        }

        self.callbacks(ModelUpdate::TreeSelection);
    }
}

pub struct TreebankModel {
    treebank: Vec<Sentence>,
}

impl TreebankModel {
    pub fn new() -> Self {
        TreebankModel {
            treebank: Vec::new(),
        }
    }

    pub fn from_iter<I>(iter: I) -> Self
    where
        I: IntoIterator<Item = Sentence>,
    {
        TreebankModel {
            treebank: Vec::from_iter(iter),
        }
    }

    pub fn graph(&self, idx: usize) -> Option<&Sentence> {
        self.treebank.get(idx)
    }

    pub fn is_empty(&self) -> bool {
        self.treebank.is_empty()
    }

    pub fn len(&self) -> usize {
        self.treebank.len()
    }

    pub fn push(&mut self, graph: Sentence) {
        self.treebank.push(graph);
    }
}

impl From<Vec<Sentence>> for TreebankModel {
    fn from(vec: Vec<Sentence>) -> Self {
        TreebankModel { treebank: vec }
    }
}
