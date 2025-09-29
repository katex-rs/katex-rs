//! Abstraction for managing CSS class lists across DOM nodes.

use alloc::borrow::Cow;
use core::slice;

/// Iterator over class names stored in a [`ClassList`].
pub enum ClassListIter<'a> {
    /// Iterator over owned class storage.
    Owned(slice::Iter<'a, Cow<'static, str>>),
    /// Iterator yielding a single static class.
    Static(Option<&'a str>),
    /// Iterator over a compile-time set of static classes.
    Const(slice::Iter<'a, &'static str>),
    /// Empty iterator.
    Empty,
}

/// Represents a collection of CSS classes applied to a DOM node.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum ClassList {
    /// Owned storage for the list of classes.
    Owned(Vec<Cow<'static, str>>),
    /// Single static class stored without allocation.
    Static(&'static str),
    /// Multiple static classes stored without allocation.
    Const(&'static [&'static str]),
    /// Explicit empty list of classes.
    #[default]
    Empty,
}

const CLASS_PREALLOCATE: usize = 4;

impl ClassList {
    /// Returns `true` if the list contains no classes.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        match self {
            Self::Owned(vec) => vec.is_empty(),
            Self::Static(_) => false,
            Self::Const(slice) => slice.is_empty(),
            Self::Empty => true,
        }
    }

    /// Returns the number of classes in the list.
    #[must_use]
    pub const fn len(&self) -> usize {
        match self {
            Self::Owned(vec) => vec.len(),
            Self::Static(_) => 1,
            Self::Const(slice) => slice.len(),
            Self::Empty => 0,
        }
    }

    /// Returns the first class if present.
    #[inline]
    #[must_use]
    pub fn first(&self) -> Option<&str> {
        match self {
            Self::Owned(vec) => vec.first().map(AsRef::as_ref),
            Self::Static(class) => Some(*class),
            Self::Const(slice) => Some(*slice.first()?),
            Self::Empty => None,
        }
    }

    /// Gets a class at a specific index if it exists.
    #[must_use]
    pub fn get(&self, index: usize) -> Option<&str> {
        match self {
            Self::Owned(vec) => vec.get(index).map(AsRef::as_ref),
            Self::Static(class) => Some(*class),
            Self::Const(slice) => slice.get(index).map(AsRef::as_ref),
            Self::Empty => None,
        }
    }

    /// Gets a mutable reference to a class at a specific index.
    pub fn get_mut(&mut self, index: usize) -> Option<&mut Cow<'static, str>> {
        let vec = self.ensure_owned();
        vec.get_mut(index)
    }

    /// Reserves capacity for additional classes when owned.
    pub fn reserve(&mut self, additional: usize) {
        if additional == 0 {
            return;
        }
        let vec = self.ensure_owned();
        vec.reserve(additional);
    }

    /// Returns `true` if the class list contains the specified class name.
    #[must_use]
    pub fn contains(&self, class: &str) -> bool {
        match self {
            Self::Owned(vec) => vec.iter().any(|entry| entry == class),
            Self::Static(entry) => *entry == class,
            Self::Const(entries) => entries.contains(&class),
            Self::Empty => false,
        }
    }

    #[inline]
    fn ensure_owned(&mut self) -> &mut Vec<Cow<'static, str>> {
        match self {
            Self::Owned(v) => v,
            _ => self.inner_owned(),
        }
    }

    #[cold]
    fn inner_owned(&mut self) -> &mut Vec<Cow<'static, str>> {
        let v = match self {
            Self::Empty => Vec::with_capacity(CLASS_PREALLOCATE),
            Self::Static(class) => {
                let mut v = Vec::with_capacity(CLASS_PREALLOCATE);
                if !class.is_empty() {
                    v.push(Cow::Borrowed(*class));
                }
                v
            }
            Self::Const(slice) => {
                let mut v = Vec::with_capacity(slice.len().saturating_add(CLASS_PREALLOCATE));
                v.extend(slice.iter().map(|&s| Cow::Borrowed(s)));
                v
            }
            Self::Owned(_) => unreachable!(),
        };

        *self = Self::Owned(v);
        if let Self::Owned(v) = self {
            v
        } else {
            unreachable!()
        }
    }

    /// Pushes a class onto the list, ignoring empty entries.
    pub fn push<S>(&mut self, class: S)
    where
        S: Into<Cow<'static, str>>,
    {
        let class = class.into();
        if class.is_empty() {
            return;
        }

        let vec = self.ensure_owned();
        vec.push(class);
    }

    /// Inserts a class at the specified position, ignoring empty entries.
    pub fn insert<S>(&mut self, index: usize, class: S)
    where
        S: Into<Cow<'static, str>>,
    {
        let class = class.into();
        if class.is_empty() {
            return;
        }

        let vec = self.ensure_owned();
        vec.insert(index, class);
    }

    /// Iterate over all classes in the list.
    #[must_use]
    pub fn iter(&self) -> ClassListIter<'_> {
        match self {
            Self::Owned(vec) => ClassListIter::Owned(vec.iter()),
            Self::Static(class) => {
                if class.is_empty() {
                    ClassListIter::Empty
                } else {
                    ClassListIter::Static(Some(*class))
                }
            }
            Self::Const(slice) => ClassListIter::Const(slice.iter()),
            Self::Empty => ClassListIter::Empty,
        }
    }
}

impl From<Vec<Cow<'static, str>>> for ClassList {
    fn from(value: Vec<Cow<'static, str>>) -> Self {
        Self::Owned(value)
    }
}

impl<'a> IntoIterator for &'a ClassList {
    type Item = &'a str;
    type IntoIter = ClassListIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl Extend<Cow<'static, str>> for ClassList {
    fn extend<T: IntoIterator<Item = Cow<'static, str>>>(&mut self, iter: T) {
        let vec = self.ensure_owned();
        vec.extend(iter.into_iter().filter(|s| !s.is_empty()));
    }
}

impl From<&'static str> for ClassList {
    fn from(value: &'static str) -> Self {
        Self::Static(value)
    }
}

impl From<&'static [&'static str]> for ClassList {
    fn from(value: &'static [&'static str]) -> Self {
        Self::Const(value)
    }
}

impl<'a> Iterator for ClassListIter<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            ClassListIter::Owned(iter) => iter.next().map(AsRef::as_ref),
            ClassListIter::Static(class) => class.take(),
            ClassListIter::Const(iter) => {
                for class in iter {
                    if !class.is_empty() {
                        return Some(*class);
                    }
                }
                None
            }
            ClassListIter::Empty => None,
        }
    }
}
