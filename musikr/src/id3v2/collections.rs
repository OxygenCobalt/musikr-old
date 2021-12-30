//! Frame collection and management.

use crate::id3v2::frames::{self, Frame, UnknownFrame, TextFrame, UserTextFrame, CreditsFrame};
use crate::id3v2::tag::{TagHeader, Version};
use std::collections::btree_map::{BTreeMap, IntoIter, Iter, IterMut, Entry, IntoKeys, IntoValues};
use std::iter::Extend;
use std::ops::{Deref, DerefMut, Index, IndexMut};
use std::cmp::Ordering;
use log::{info, warn};

/// A collection of known frames associated to their respective keys.
///
/// Each frame in a `FrameMap` is tied to a key consisting of the Frame ID followed by
/// any information that indicates the frame's uniqueness. For more information, see
/// [`Frame.key`](crate::id3v2::frames::Frame::key). `FrameMap` will attempt to dynmaically.
/// 
/// `FrameMap` is internally based on a [`BTreeMap`](std::collections::btree_map::BTreeMap).
/// The order of the frames when written will differ. See [`Tag::write`](crate::id3v2::Tag::save).
///
/// # Example
///
/// ```
/// use musikr::id3v2::collections::FrameMap;
/// use musikr::id3v2::frames::{TextFrame, UrlFrame};
/// use musikr::{text_frame, url_frame};
///
/// let mut map = FrameMap::new();
/// map.add(text_frame! { b"TLAN"; "eng" });
/// map.add(url_frame! { b"WOAR"; "example.com" });
///
/// assert!(map.contains_key("TLAN"));
/// assert!(map.contains_key("WOAR"));
///
/// // Text frames added to the FrameMap using add will be merged with any pre-existing
/// // text frames.
/// map.add(text_frame! { b"TLAN"; "deu" });
/// map.add(url_frame! { b"WOAR"; "test.com" });
///
/// assert_eq!(map["TLAN"].downcast::<TextFrame>().unwrap().text, &["eng", "deu"]);
/// assert_eq!(map["WOAR"].downcast::<UrlFrame>().unwrap().url, "example.com");
///
/// // Insert will overwrite any frames if present
/// map.insert(text_frame! { b"TLAN"; "ara"});
/// assert_eq!(map["TLAN"].downcast::<TextFrame>().unwrap().text, &["ara"]);
/// ```
#[derive(Debug, Clone, Default)]
pub struct FrameMap {
    map: BTreeMap<String, Box<dyn Frame>>,
}

impl FrameMap {
    /// Creates a new `FrameMap` instance. This is equivalent to calling
    /// `FrameMap::default`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a non-boxed frame to the `FrameMap`.
    ///
    /// This is equivalent to calling [add_boxed](crate::id3v2::collections::FrameMap::add_boxed)
    /// with the frame in a (`Box`)(std::boxed::Box).
    #[inline]
    pub fn add(&mut self, frame: impl Frame) {
        self.add_boxed(Box::new(frame));
    }

    /// Inserts a non-boxed frame to the `FrameMap`.
    ///
    /// This is equivalent to calling [insert_boxed](crate::id3v2::collections::FrameMap::add_boxed) 
    /// with the frame in a (`Box`)(std::boxed::Box).
    #[inline]
    pub fn insert(&mut self, frame: impl Frame) {
        self.insert_boxed(Box::new(frame))
    }

    /// Adds a boxed frame into the `FrameMap`.
    ///
    /// If a frame with the same key is not present within the `FrameMap`, then the frame will
    /// be added. If a frame is present, and both frames are [text frames](crate::id3v2::frames::text),
    /// then all of the text from the new frame will be added to the original frame. Otherwise,
    /// the frame is not added.
    pub fn add_boxed(&mut self, frame: Box<dyn Frame>) {
        let entry = self.map.entry(frame.key());

        match entry {
            Entry::Occupied(mut entry) => {
                // This entry is occupied. If these frames are text frames, than great,
                // we can merge them. If not, we just don't add it.
                let orig = entry.get_mut().deref_mut();
                let new = frame.deref();

                if is_both::<TextFrame>(orig, new) {
                    info!("merging added {} frame with pre-existing frame", new.id());
                    orig.downcast_mut::<TextFrame>().unwrap()
                        .text.extend(new.downcast::<TextFrame>().unwrap().text.clone())
                } else if is_both::<UserTextFrame>(orig, new) {
                    info!("merging added {} frame with pre-existing frame", new.id());
                    orig.downcast_mut::<UserTextFrame>().unwrap()
                        .text.extend(new.downcast::<UserTextFrame>().unwrap().text.clone())        
                } else if is_both::<CreditsFrame>(orig, new) {
                    info!("merging added {} frame with pre-existing frame", new.id());
                    orig.downcast_mut::<CreditsFrame>().unwrap()
                        .people.extend(new.downcast::<CreditsFrame>().unwrap().people.clone())
                }
            },

            Entry::Vacant(entry) => {
                entry.insert(frame);
            }
        }
    }

    /// Inserts a boxed frame into the `FrameMap`.
    ///
    /// If a frame with the same key is not present in the `FrameMap`, then the frame
    /// will be added. If a frame is present, then it will be overwritten.
    pub fn insert_boxed(&mut self, frame: Box<dyn Frame>) {
        self.map.insert(frame.key(), frame);
    }

    /// Returns a reference to the frame corresponding to the key.
    pub fn get(&self, key: &str) -> Option<&dyn Frame> {
        Some(self.map.get(key)?.deref())
    }

    /// Returns a mutable reference to the frame corresponding to the key.
    pub fn get_mut(&mut self, key: &str) -> Option<&mut dyn Frame> {
        Some(self.map.get_mut(key)?.deref_mut())
    }

    /// Returns a list of references to all frames that have the specified Frame ID.
    ///
    /// This method does not ensure the validity of the ID given.
    /// [`FrameId::inner`](crate::id3v2::frames::FrameId) can be used to transform
    /// a valid Frame ID into a value that can be used.
    pub fn get_all(&self, id: &[u8; 4]) -> Vec<&dyn Frame> {
        self.values().filter(|frame| frame.id() == id).collect()
    }

    /// Returns a list of mutable references to all frames that have the specified Frame ID.
    ///
    /// This method does not ensure the validity of the ID given.
    /// [`FrameId::inner`](crate::id3v2::frames::FrameId) can be used to transform
    /// a valid Frame ID into a value that can be used.
    pub fn get_all_mut(&mut self, id: &[u8; 4]) -> Vec<&mut dyn Frame> {
        self.values_mut().filter(|frame| frame.id() == id).collect()
    }

    /// Removes all frames that match the specified Frame ID.
    ///
    /// This method does not ensure the validity of the ID given.
    /// [`FrameId::inner`](crate::id3v2::frames::FrameId) can be used to transform
    /// a valid Frame ID into a value that can be used.
    pub fn remove_all(&mut self, id: &[u8; 4]) -> Vec<Box<dyn Frame>> {
        // We can't use retain here since it doesn't return the removed items, so we have
        // to iterate manually and find the values we need to remove. However, this also
        // means we have do the removal in two passes, one to collect what to remove
        // immutably and another to actually remove the items.
        let keys: Vec<String> = self
            .values()
            .filter_map(|frame| if frame.id() == id  { Some(frame.key()) } else { None })
            .collect();

        keys
            .iter()
            .map(|key| self.remove(key).unwrap())
            .collect()
    }

    /// Returns true if the map contains a value for the specified key.
    pub fn contains_key(&self, key: &str) -> bool {
        self.map.contains_key(key)
    }

    /// Returns true if the map contains any frames with the specified Frame ID.
    pub fn contains_any(&self, id: &[u8; 4]) -> bool {
        self.values().filter(|frame| frame.id() == id).count() != 0
    }

    /// Returns an iterator over the entries of the map, sorted by key.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &dyn Frame)> + '_ {
        self.map.iter().map(|(k, v)| (k.as_str(), v.deref()))
    }

    /// Returns an iterator over the values of the map, in order by key.
    pub fn values(&self) -> impl Iterator<Item = &dyn Frame> + '_ {
        self.map.values().map(|v| v.deref())
    }

    /// Returns a mutable iterator over the values of the map, in order by key.
    pub fn values_mut(&mut self) -> impl Iterator<Item = &mut dyn Frame> + '_ {
        self.map.values_mut().map(|v| v.deref_mut())
    }

    /// Retains only the elements specified by the predicate.
    ///
    /// In other words, remove all pairs `(k, v)` such that `f(&k, &mut v)` returns `false`. 
    /// The elements are visited in ascending key order.
    pub fn retain<F>(&mut self, mut keep: F)
    where
        F: FnMut(&String, &mut dyn Frame) -> bool,
    {
        self.map.retain(|k, v| keep(k, v.deref_mut()))
    }

    /// Returns a reference to the inner [`BTreeMap`](crate::id3v2::Tag::save) for this instance.
    pub fn inner(&self) -> &BTreeMap<String, Box<dyn Frame>> {
        &self.map
    }

    pub(crate) fn render(&self, header: &TagHeader) -> impl Iterator<Item=u8> + '_ {
        const PRIORITY: &[&[u8; 4]] = &[b"TIT2", b"TPE1", b"TALB", b"TRCK", b"TPOS", b"TDRC", b"TCON"];

        let mut frame_pairs: Vec<(&dyn Frame, Vec<u8>)> = Vec::new();

        for frame in self.values() {
            if !frame.is_empty() {
                match frames::render(header, frame.deref()) {
                    Ok(data) => frame_pairs.push((frame, data)),
                    Err(_) => warn!("could not render frame {}", frame.key()),
                }
            } else {
                info!("dropping empty frame {}", frame.key())
            }
        }

        // TIT2, TPE1, TALB, TRCK, TPOS, TDRC, and TCON are placed at the top of the tag
        // in that order, as they are considered "priority" metadata. All other frames
        // are placed in order of size and then key.
        frame_pairs.sort_by(|(a_frame, a_data), (b_frame, b_data)| {
            let a_priority = PRIORITY.iter().position(|&id| a_frame.id().as_ref() == id);
            let b_priority = PRIORITY.iter().position(|&id| b_frame.id().as_ref() == id);

            match (a_priority, b_priority) {
                (Some(a_pos), Some(b_pos)) => a_pos.cmp(&b_pos),
                (Some(_), None) => Ordering::Greater,
                (None, Some(_)) => Ordering::Less,
                (None, None) => match a_data.len().cmp(&b_data.len()) {
                    Ordering::Equal => a_frame.key().cmp(&b_frame.key()),
                    ord => ord
                }
            }
        });

        frame_pairs.into_iter()
            .flat_map(|(_, data)| data.into_iter())
    }

    delegate::delegate! {
        to self.map {
            /// Clears the map, removing all elements.
            pub fn clear(&mut self);
            /// Gets an iterator over the keys of the map, in sorted order.
            pub fn keys(&self);
            /// Returns the number of elements in the map.
            pub fn len(&self) -> usize;
            /// Returns true if the map contains no elements.
            pub fn is_empty(&self) -> bool;
            /// Creates a consuming iterator visiting all the keys, in sorted order. 
            /// The map cannot be used after calling this. The iterator element type
            /// is the frame key.
            pub fn into_keys(self) -> IntoKeys<String, Box<dyn Frame>>;
            /// Creates a consuming iterator visiting all the values, in order by key. 
            /// The map cannot be used after calling this. The iterator element type is
            /// the frame instances.
            pub fn into_values(self) -> IntoValues<String, Box<dyn Frame>>;
            /// Removes a key from the map, returning the value at the key if the key
            /// was previously in the map.
            pub fn remove(&mut self, key: &str) -> Option<Box<dyn Frame>>;
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
        self.map.get_mut(key).unwrap().deref_mut()
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

impl From<BTreeMap<String, Box<dyn Frame>>> for FrameMap {
    fn from(other: BTreeMap<String, Box<dyn Frame>>) -> Self {
        Self { map: other }
    }
}

#[inline(always)]
fn is_both<T: Frame>(orig: &mut dyn Frame, new: &dyn Frame) -> bool {
    orig.is::<T>() && new.is::<T>()
}

/// A collection of unknown frames.
///
/// This collection is immutable and tied to the [`Version`](crate::id3v2::tag::Version)
/// of the tag. If the tag is upgraded or downgraded at any point, then the frames in this
/// will not be written.
#[derive(Debug, Clone)]
pub struct UnknownFrames {
    version: Version,
    frames: Vec<UnknownFrame>,
}

impl UnknownFrames {
    pub(crate) fn new(version: Version, frames: Vec<UnknownFrame>) -> Self {
        Self { version, frames }
    }

    /// Returns the [`Version`](crate::id3v2::tag::Version) of the tag that these frames
    /// were initially written to.
    pub fn version(&self) -> Version {
        self.version
    }

    /// Returns a reference to the [`UnknownFrame`](crate::id3v2::frames::UnknownFrame)
    /// instances in this collection.
    pub fn frames(&self) -> &[UnknownFrame] {
        &self.frames
    }
}
