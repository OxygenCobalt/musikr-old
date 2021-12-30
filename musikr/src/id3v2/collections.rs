//! Frame collection and management.

use crate::id3v2::frames::{Frame, UnknownFrame, TextFrame, UserTextFrame, CreditsFrame};
use crate::id3v2::tag::Version;
use indexmap::map::{Drain, IntoIter, Iter, IterMut, Keys, Entry};
use indexmap::IndexMap;
use std::cmp::Ordering;
use std::iter::Extend;
use std::ops::{Deref, DerefMut, Index, IndexMut, RangeBounds};

// TODO: Migrate to BTreeMap and make a ordered function

#[derive(Debug, Clone, Default)]
pub struct FrameMap {
    map: IndexMap<String, Box<dyn Frame>>,
}

impl FrameMap {
    pub fn new() -> Self {
        Self::default()
    }

    #[inline]
    pub fn add(&mut self, frame: impl Frame) {
        self.add_boxed(Box::new(frame));
    }

    #[inline]
    pub fn insert(&mut self, frame: impl Frame) {
        self.insert_boxed(Box::new(frame))
    }

    pub fn add_boxed(&mut self, frame: Box<dyn Frame>) {
        let entry = self.map.entry(frame.key());

        match entry {
            Entry::Occupied(mut entry) => {
                // This entry is occupied, let's see if we can merge these frames.
                // The only frames we can merge sanely here are text frames, otherwise,
                // we just leave it as is.
                let orig = entry.get_mut().deref_mut();
                let new = frame.deref();

                if is_both::<TextFrame>(orig, new) {
                    orig.downcast_mut::<TextFrame>().unwrap()
                        .text.extend(new.downcast::<TextFrame>().unwrap().text.clone())
                } else if is_both::<UserTextFrame>(orig, new) {
                    orig.downcast_mut::<UserTextFrame>().unwrap()
                        .text.extend(new.downcast::<UserTextFrame>().unwrap().text.clone())        
                } else if is_both::<CreditsFrame>(orig, new) {
                    orig.downcast_mut::<CreditsFrame>().unwrap()
                        .people.extend(new.downcast::<CreditsFrame>().unwrap().people.clone())
                }
            },

            Entry::Vacant(entry) => {
                // Entry is unoccupied, add the frame.
                entry.insert(frame);
            }
        }
    }

    pub fn insert_boxed(&mut self, frame: Box<dyn Frame>) {
        self.map.insert(frame.key(), frame);
    }

    pub fn get(&self, key: &str) -> Option<&dyn Frame> {
        Some(self.map.get(key)?.deref())
    }

    pub fn get_mut(&mut self, key: &str) -> Option<&mut dyn Frame> {
        Some(self.map.get_mut(key)?.deref_mut())
    }

    pub fn get_all(&self, id: &[u8; 4]) -> Vec<&dyn Frame> {
        self.values().filter(|frame| frame.id() == id).collect()
    }

    pub fn get_all_mut(&mut self, id: &[u8; 4]) -> Vec<&mut dyn Frame> {
        self.values_mut().filter(|frame| frame.id() == id).collect()
    }

    pub fn remove_all(&mut self, id: &[u8; 4]) -> Vec<Box<dyn Frame>> {
        // We can't use retain here since it doesn't return the removed items, so we have
        // to iterate manually and find the indices for the values we need to remove.
        let indicies: Vec<usize> = self
            .values()
            .enumerate()
            .filter_map(|(i, frame)| if frame.id() == id { Some(i) } else { None })
            .collect();

        // Swap remove here so that this doesn't become O(scary)
        indicies
            .iter()
            .map(|&i| self.map.swap_remove_index(i).unwrap().1)
            .collect()
    }

    pub fn contains(&self, frame: &dyn Frame) -> bool {
        self.contains_key(&frame.key())
    }

    pub fn contains_key(&self, key: &str) -> bool {
        self.map.contains_key(key)
    }

    pub fn contains_any(&self, id: &[u8; 4]) -> bool {
        self.values().filter(|frame| frame.id() == id).count() != 0
    }

    pub fn split_off(&mut self, at: usize) -> Self {
        Self {
            map: self.map.split_off(at),
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = (&str, &dyn Frame)> + '_ {
        self.map.iter().map(|(k, v)| (k.as_str(), v.deref()))
    }

    pub fn values(&self) -> impl Iterator<Item = &dyn Frame> + '_ {
        self.map.values().map(|v| v.deref())
    }

    pub fn values_mut(&mut self) -> impl Iterator<Item = &mut dyn Frame> + '_ {
        self.map.values_mut().map(|v| v.deref_mut())
    }

    pub fn sort_by<F>(&mut self, mut cmp: F)
    where
        F: FnMut(&String, &dyn Frame, &String, &dyn Frame) -> Ordering,
    {
        self.map
            .sort_by(|ka, va, kb, vb| cmp(ka, va.deref(), kb, vb.deref()))
    }

    pub fn sorted_by<F>(self, mut cmp: F) -> IntoIter<String, Box<dyn Frame>>
    where
        F: FnMut(&String, &dyn Frame, &String, &dyn Frame) -> Ordering,
    {
        self.map
            .sorted_by(|ka, va, kb, vb| cmp(ka, va.deref(), kb, vb.deref()))
    }

    pub fn retain<F>(&mut self, mut keep: F)
    where
        F: FnMut(&String, &mut dyn Frame) -> bool,
    {
        self.map.retain(|k, v| keep(k, v.deref_mut()))
    }

    pub fn first(&self) -> Option<(&String, &dyn Frame)> {
        let (k, v) = self.map.first()?;
        Some((k, v.deref()))
    }

    pub fn first_mut(&mut self) -> Option<(&String, &mut dyn Frame)> {
        let (k, v) = self.map.first_mut()?;
        Some((k, v.deref_mut()))
    }

    pub fn last(&self) -> Option<(&String, &dyn Frame)> {
        let (k, v) = self.map.last()?;
        Some((k, v.deref()))
    }

    pub fn last_mut(&mut self) -> Option<(&String, &mut dyn Frame)> {
        let (k, v) = self.map.last_mut()?;
        Some((k, v.deref_mut()))
    }

    pub fn map(&self) -> &IndexMap<String, Box<dyn Frame>> {
        &self.map
    }

    delegate::delegate! {
        to self.map {
            pub fn clear(&mut self);
            pub fn keys(&self) -> Keys<String, Box<dyn Frame>>;
            pub fn len(&self) -> usize;
            pub fn is_empty(&self) -> bool;
            pub fn capacity(&self) -> usize;
            pub fn truncate(&mut self, len: usize);
            pub fn drain<R>(&mut self, range: R) -> Drain<'_, String, Box<dyn Frame>>
                where R: RangeBounds<usize>;
            pub fn reserve(&mut self, additional: usize);
            pub fn remove(&mut self, id: &str) -> Option<Box<dyn Frame>>;
            pub fn shrink_to_fit(&mut self);
            pub fn pop(&mut self) -> Option<(String, Box<dyn Frame>)>;
            pub fn sort_keys(&mut self);
            pub fn reverse(&mut self);
        }
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

impl Extend<(String, Box<dyn Frame>)> for FrameMap {
    fn extend<I: IntoIterator<Item = (String, Box<dyn Frame>)>>(&mut self, iterable: I) {
        self.map.extend(iterable)
    }
}

impl From<IndexMap<String, Box<dyn Frame>>> for FrameMap {
    fn from(other: IndexMap<String, Box<dyn Frame>>) -> Self {
        Self { map: other }
    }
}

#[inline(always)]
fn is_both<T: Frame>(orig: &mut dyn Frame, new: &dyn Frame) -> bool {
    orig.is::<T>() && new.is::<T>()
}

#[derive(Debug, Clone)]
pub struct UnknownFrames {
    version: Version,
    frames: Vec<UnknownFrame>,
}

impl UnknownFrames {
    pub(crate) fn new(version: Version, frames: Vec<UnknownFrame>) -> Self {
        Self { version, frames }
    }

    pub fn version(&self) -> Version {
        self.version
    }

    pub fn frames(&self) -> &[UnknownFrame] {
        &self.frames
    }
}
