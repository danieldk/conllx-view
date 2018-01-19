use std::iter::FromIterator;

use graph::DependencyGraph;

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

    pub fn len(&self) -> usize {
        self.treebank.len()
    }
}

impl From<Vec<DependencyGraph>> for TreebankModel {
    fn from(vec: Vec<DependencyGraph>) -> Self {
        TreebankModel { treebank: vec }
    }
}
