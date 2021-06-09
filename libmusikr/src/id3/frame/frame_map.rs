use std::collections::HashMap;
use std::ops::Index;
use std::collections::hash_map::{IntoIter, Iter, IterMut, Values, ValuesMut, Keys};
use std::ops::{Deref, DerefMut};
use crate::id3::frame::Id3Frame;

pub struct FrameMap {
    map: HashMap<String, Box<dyn Id3Frame>>
}

impl FrameMap {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self, frame: Box<dyn Id3Frame>) {
        self.map.entry(frame.key()).or_insert(frame);
    }

    pub fn insert(&mut self, frame: Box<dyn Id3Frame>) {
        self.map.insert(frame.key(), frame);
    }

    pub fn get(&self, key: &str) -> Option<&dyn Id3Frame> {
        Some(self.map.get(key)?.deref())
    }

    pub fn get_mut(&mut self, key: &str) -> Option<&mut dyn Id3Frame> {
        Some(self.map.get_mut(key)?.deref_mut())
    }

    pub fn get_all(&self, id: &str) -> Vec<&dyn Id3Frame> {
        self.frames()
            .filter(|frame| frame.id() == id)
            .map(|frame| frame.deref())
            .collect()
    }

    pub fn get_all_mut(&mut self, id: &str) -> Vec<&mut dyn Id3Frame> {
        // Tried using iterator methods here and the borrow checker had
        // a tantrum about static lifecycles, so normal for loop it is
        let mut vec: Vec<&mut dyn Id3Frame> = Vec::new();

        for frame in self.frames_mut() {
            if frame.id() == id {
                vec.push(frame.deref_mut())
            }
        }

        vec
    }

    pub fn remove_all(&mut self, id: &str) {
        self.map.retain(|_, frame| {
            frame.id() != id
        })
    }

    pub fn keys(&self) -> Keys<String, Box<dyn Id3Frame>> {
        self.map.keys()
    }

    pub fn frames(&self) -> Values<String, Box<dyn Id3Frame>> {
        self.map.values()
    }

    pub fn frames_mut(&mut self) -> ValuesMut<String, Box<dyn Id3Frame>> {
        self.map.values_mut()
    }

    pub fn hash_map(&self) -> &HashMap<String, Box<dyn Id3Frame>> {
        &self.map
    }
}

impl Default for FrameMap {
    fn default() -> Self {
        FrameMap {
            map: HashMap::new()
        }
    }
}

impl Index<&String> for FrameMap {
    type Output = dyn Id3Frame;

    fn index(&self, key: &String) -> &Self::Output {
        self.map[key].deref()
    }
}

impl IntoIterator for FrameMap {
    type Item = (String, Box<dyn Id3Frame>);
    type IntoIter = IntoIter<String, Box<dyn Id3Frame>>;

    fn into_iter(self) -> Self::IntoIter {
        self.map.into_iter()
    }
}

impl <'a> IntoIterator for &'a FrameMap {
    type Item = (&'a String, &'a Box<dyn Id3Frame>);
    type IntoIter = Iter<'a, String, Box<dyn Id3Frame>>;

    fn into_iter(self) -> Self::IntoIter {
        self.map.iter()
    }    
}

impl <'a> IntoIterator for &'a mut FrameMap {
    type Item = (&'a String, &'a mut Box<dyn Id3Frame>);
    type IntoIter = IterMut<'a, String, Box<dyn Id3Frame>>;

    fn into_iter(self) -> Self::IntoIter {
        self.map.iter_mut()
    }    
}