//! Frame collection and management.

use crate::id3v2::frames::Frame;
use indexmap::map::{IntoIter, Iter, IterMut, Keys, Values, ValuesMut};
use indexmap::IndexMap;
use std::ops::{Deref, DerefMut, Index, IndexMut};

#[derive(Debug, Clone, Default)]
pub struct FrameMap {
    map: IndexMap<String, Box<dyn Frame>>,
}

impl FrameMap {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self, frame: Box<dyn Frame>) {
        self.map.entry(frame.key()).or_insert(frame);
    }

    pub fn insert(&mut self, frame: Box<dyn Frame>) {
        self.map.insert(frame.key(), frame);
    }

    pub fn get(&self, key: &str) -> Option<&dyn Frame> {
        Some(self.map.get(key)?.deref())
    }

    pub fn get_mut(&mut self, key: &str) -> Option<&mut dyn Frame> {
        Some(self.map.get_mut(key)?.deref_mut())
    }

    pub fn get_all(&self, id: &str) -> Vec<&dyn Frame> {
        self.values()
            .filter(|frame| frame.id() == id)
            .map(|frame| frame.deref())
            .collect()
    }

    pub fn get_all_mut(&mut self, id: &str) -> Vec<&mut dyn Frame> {
        self.values_mut()
            .filter(|frame| frame.id() == id)
            .map(|frame| frame.deref_mut())
            .collect()
    }

    pub fn remove_all(&mut self, id: &str) {
        self.map.retain(|_, frame| frame.id() != id)
    }

    pub fn keys(&self) -> Keys<String, Box<dyn Frame>> {
        self.map.keys()
    }

    pub fn values(&self) -> Values<String, Box<dyn Frame>> {
        self.map.values()
    }

    pub fn values_mut(&mut self) -> ValuesMut<String, Box<dyn Frame>> {
        self.map.values_mut()
    }

    pub fn len(&self) -> usize {
        self.map.len()
    }

    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    pub fn contains(&self, frame: &dyn Frame) -> bool {
        self.map.contains_key(&frame.key())
    }

    pub fn map(&self) -> &IndexMap<String, Box<dyn Frame>> {
        &self.map
    }
}

impl Index<&str> for FrameMap {
    type Output = dyn Frame;

    fn index(&self, key: &str) -> &Self::Output {
        self.map[key].deref()
    }
}

impl IndexMut<&str> for FrameMap {
    fn index_mut(&mut self, key: &str) -> &mut Self::Output {
        self.map[key].deref_mut()
    }
}

impl IntoIterator for FrameMap {
    type Item = (String, Box<dyn Frame>);
    type IntoIter = IntoIter<String, Box<dyn Frame>>;

    fn into_iter(self) -> Self::IntoIter {
        self.map.into_iter()
    }
}

impl<'a> IntoIterator for &'a FrameMap {
    type Item = (&'a String, &'a Box<dyn Frame>);
    type IntoIter = Iter<'a, String, Box<dyn Frame>>;

    fn into_iter(self) -> Self::IntoIter {
        self.map.iter()
    }
}

impl<'a> IntoIterator for &'a mut FrameMap {
    type Item = (&'a String, &'a mut Box<dyn Frame>);
    type IntoIter = IterMut<'a, String, Box<dyn Frame>>;

    fn into_iter(self) -> Self::IntoIter {
        self.map.iter_mut()
    }
}
