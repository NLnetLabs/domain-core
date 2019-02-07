/// Uncompressed, relative domain names.
///
/// This is a private module. Its public types are re-exported by the parent.

use std::{cmp, fmt, hash, ops};
use bytes::{BufMut, Bytes};
use crate::bits::compose::Compose;
use crate::bits::octets::Octets;
use super::builder::DnameBuilder;
use super::chain::{Chain, LongChainError};
use super::dname::Dname;
use super::label::{Label, LabelTypeError, SplitLabelError};
use super::traits::{ToLabelIter, ToRelativeDname};


//------------ RelativeDname -------------------------------------------------

/// An uncompressed, relative domain name.
///
/// A relative domain name is one that doesn’t end with the root label. As the
/// name suggests, it is relative to some other domain name. This type wraps
/// a [`Bytes`] value containing such a relative name similarly to the way
/// [`Dname`] wraps an absolute one. In fact, it behaves very similarly to
/// [`Dname`] taking into account differences when slicing and dicing names.
///
/// `RelativeDname` guarantees that the name is at most 254 bytes long. As the
/// length limit for a domain name is actually 255 bytes, this means that you
/// can always safely turn a `RelativeDname` into a `Dname` by adding the root
/// label (which is exactly one byte long).
///
/// [`Bytes`]: ../../../bytes/struct.Bytes.html
/// [`Dname`]: struct.Dname.html
#[derive(Clone)]
pub struct RelativeDname<O=Bytes> {
    octets: O,
}

/// # Creation and Conversion
///
impl<O: Octets> RelativeDname<O> {
    /// Creates a relative domain name from octets without checking.
    ///
    /// Since the content of the octets can be anything, really, this is
    /// an unsafe function.
    pub(super) unsafe fn from_octets_unchecked(octets: O) -> Self {
        RelativeDname { octets }
    }

    /// Creates an empty relative domain name.
    ///
    /// Determining what this could possibly be useful for is left as an
    /// excercise to the reader.
    pub fn empty() -> Self {
        unsafe {
            RelativeDname::from_octets_unchecked(O::from_static(b""))
        }
    }

    /// Creates a relative domain name representing the wildcard label.
    ///
    /// The wildcard label is intended to match any label. There are special
    /// rules for names with wildcard labels. Note that the comparison traits
    /// implemented for domain names do *not* consider wildcards and treat
    /// them as regular labels.
    pub fn wildcard() -> Self {
        unsafe {
            RelativeDname::from_octets_unchecked(O::from_static(b"\x01*"))
        }
    }

    /// Creates a relative domain name from octets.
    ///
    /// This checks that `octets` contains a properly encoded relative domain
    /// name and fails if it doesn’t.
    pub fn from_octets(octets: O) -> Result<Self, RelativeDnameError> {
        if octets.len() > 254 {
            return Err(RelativeDnameError::LongName)
        }
        {
            let mut tmp = octets.as_ref();
            while !tmp.is_empty() {
                let (label, tail) = Label::split_from(tmp)?;
                if label.is_root() {
                    return Err(RelativeDnameError::AbsoluteName);
                }
                tmp = tail;
            }
        }
        Ok(unsafe { RelativeDname::from_octets_unchecked(octets) })
    }

    /// Returns a reference to the octets.
    pub fn as_octets(&self) -> &O {
        &self.octets
    }

    /// Returns a reference to the underlying byte slice.
    pub fn as_slice(&self) -> &[u8] {
        self.octets.as_ref()
    }

    /// Converts the name into its underlying octets.
    pub fn unwrap(self) -> O {
        self.octets
    }

    /// Creates a domain name by concatenating `self` with `other`.
    ///
    /// Depending on whether `other` is an absolute or relative domain name,
    /// the resulting name will behave like an absolute or relative name.
    /// 
    /// The method will fail if the combined length of the two names is
    /// greater than the size limit of 255. Note that in this case you will
    /// loose both `self` and `other`, so it might be worthwhile to check
    /// first.
    pub fn chain<N: Compose>(
        self, other: N
    ) -> Result<Chain<Self, N>, LongChainError> {
        Chain::new(self, other)
    }

    /// Creates an absolute name by chaining the root label to it.
    pub fn chain_root(self) -> Chain<Self, Dname<&'static [u8]>> {
        self.chain(Dname::root()).unwrap()
    }
}

impl RelativeDname<Bytes> {
    pub fn from_bytes(bytes: Bytes) -> Result<Self, RelativeDnameError> {
        Self::from_octets(bytes)
    }

    /// Returns a reference to the underlying bytes value.
    pub fn as_bytes(&self) -> &Bytes {
        &self.octets
    }

    /// Converts the name into the underlying bytes value.
    pub fn into_bytes(self) -> Bytes {
        self.octets
    }

    /// Converts the name into a domain name builder for appending data.
    ///
    /// If the underlying bytes value can be converted into a [`BytesMut`][]
    /// (via its [`try_mut`] method), the builder will use that directly.
    /// Otherwise, it will create an all new [`BytesMut`] from the name’s
    /// content.
    ///
    /// [`BytesMut`]: ../../../bytes/struct.BytesMut.html
    /// [`try_mut`]: ../../../bytes/struct.BytesMut.html#method.try_mut
    pub fn into_builder(self) -> DnameBuilder {
        let bytes = match self.octets.try_mut() {
            Ok(bytes) => bytes,
            Err(bytes) => bytes.as_ref().into()
        };
        unsafe { DnameBuilder::from_bytes(bytes) }
    }

    /// Converts the name into an absolute name by appending the root label.
    ///
    /// This manipulates the name itself and thus may have to copy it. If
    /// you just need an absolute name, you can perhaps use [`chain_root`]
    /// instead.
    ///
    /// [`chain_root`]: #method.chain_root
    pub fn into_absolute(self) -> Dname<Bytes> {
        self.into_builder().into_dname().unwrap()
    }
}

impl<'a> RelativeDname<&'a [u8]> {
    /// Creates a domain name for an octet slice.
    pub fn from_slice(slice: &'a [u8]) -> Result<Self, RelativeDnameError> {
        Self::from_octets(slice)
    }
}

impl RelativeDname<&'static [u8]> {
    /// Creates an empty domain name as a static octet slice.
    pub fn static_empty() -> Self {
        Self::empty()
    }

    /// Creates a wildcard domain name as a static octet slice.
    pub fn static_wildcard() -> Self {
        Self::wildcard()
    }
}


/// # Working with Labels
///
impl<O: Octets> RelativeDname<O> {
    /// Returns an iterator over the labels of the domain name.
    pub fn iter(&self) -> DnameIter {
        DnameIter::new(self.octets.as_ref())
    }

    /// Returns the number of labels in the name.
    pub fn label_count(&self) -> usize {
        self.iter().count()
    }

    /// Returns a reference to the first label if the name isn’t empty.
    pub fn first(&self) -> Option<&Label> {
        self.iter().next()
    }

    /// Returns a reference to the last label if the name isn’t empty.
    pub fn last(&self) -> Option<&Label> {
        self.iter().next_back()
    }

    /// Returns the number of dots in the string representation of the name.
    ///
    /// Specifically, returns a value equal to the number of labels minus one,
    /// except for an empty name where it returns a zero, also.
    pub fn ndots(&self) -> usize {
        if self.is_empty() { 0 }
        else {
            self.label_count() - 1
        }
    }

    /// Determines whether `base` is a prefix of `self`.
    pub fn starts_with<'a, N: ToLabelIter<'a>>(&'a self, base: &'a N) -> bool {
        <Self as ToLabelIter>::starts_with(self, base)
    }

    /// Determines whether `base` is a suffix of `self`.
    pub fn ends_with<'a, N: ToLabelIter<'a>>(&'a self, base: &'a N) -> bool {
        <Self as ToLabelIter>::ends_with(self, base)
    }

    /// Returns whether an index points to the first byte of a label.
    pub fn is_label_start(&self, mut index: usize) -> bool {
        if index == 0 {
            return true
        }
        let mut tmp = self.as_slice();
        while !tmp.is_empty() {
            let (label, tail) = Label::split_from(tmp).unwrap();
            let len = label.len() + 1;
            if index < len {
                return false
            }
            else if index == len {
                return true
            }
            index -= len;
            tmp = tail;
        }
        false
    }

    /// Like `is_label_start` but panics if it isn’t.
    fn check_index(&self, index: usize) {
        if !self.is_label_start(index) {
            panic!("index not at start of a label");
        }
    }

    /// Returns a part of the name indicated by start and end octet indexes.
    ///
    /// The returned name will start at position `start` and end right before
    /// position `end`. Both positions need to be the beginning of a label.
    ///
    /// # Panics
    ///
    /// The method panics if either position is not the beginning of a label
    /// or is out of bounds.
    pub fn range(&self, start: usize, end: usize) -> Self {
        self.check_index(start);
        self.check_index(end);
        unsafe { Self::from_octets_unchecked(self.octets.range(start, end)) }
    }

    /// Returns the part of the name indicated by start and end octet indexes.
    ///
    /// This is the old name of [`range`] and is therefore obsolete.
    ///
    /// [`range`]: #method.range
    #[deprecated(since="0.5.0", note="renamed to range")]
    pub fn slice(&self, start: usize, end: usize) -> Self {
        self.range(start, end)
    }

    /// Returns the part of the name starting at the given octet index.
    ///
    /// # Panics
    ///
    /// The method panics if the index is not the beginning of a label
    /// or is beyond the end of the name.
    pub fn range_from(&self, start: usize) -> Self {
        self.check_index(start);
        unsafe { Self::from_octets_unchecked(self.octets.range_from(start)) }
    }

    /// Returns the part of the name starting at the given octet index.
    ///
    /// This is the old name of [`range_from`] and is therefore obsolete.
    ///
    /// [`range_from`]: #method.range_from
    pub fn slice_from(&self, begin: usize) -> Self {
        self.range_from(begin)
    }

    /// Returns the part of the name ending before the given octet index.
    ///
    /// # Panics
    ///
    /// The method panics if the position is not the beginning of a label
    /// or is beyond the end of the name.
    pub fn range_to(&self, end: usize) -> Self {
        self.check_index(end);
        unsafe { Self::from_octets_unchecked(self.octets.range_to(end)) }
    }

    /// Returns the part of the name ending at the given octet index.
    ///
    /// This is the old name of [`range_to`] and is therefore obsolete.
    ///
    /// [`range_to`]: #method.range_from
    pub fn slice_to(&self, end: usize) -> Self {
        self.range_to(end)
    }

    /// Splits the name in two at the given octet index.
    ///
    /// Consumes the name and returns a tuple of the two parts left over after
    /// splitting.
    ///
    /// # Panics
    ///
    /// The method panics if the position is not the beginning of a label
    /// or is beyond the end of the name.
    pub fn split_at(self, mid: usize) -> (Self, Self) {
        self.check_index(mid);
        unsafe {(
            Self::from_octets_unchecked(self.octets.range_to(mid)),
            Self::from_octets_unchecked(self.octets.range_from(mid))
        )}
    }

    /// Splits off the name starting at the given octet index.
    ///
    /// Afterwards, `self` will contain the name ending before the position
    /// while the name starting at the position will be returned.
    ///
    /// # Panics
    ///
    /// The method panics if the position is not the beginning of a label
    /// or is beyond the end of the name.
    pub fn split_off(&mut self, mid: usize) -> Self {
        self.check_index(mid);
        let right = self.octets.range_from(mid);
        self.octets = self.octets.range_to(mid);
        unsafe { Self::from_octets_unchecked(right) }
    }

    /// Splits off the name ending before the given octet index.
    ///
    /// Afterwards, `self` will contain the name starting at the index
    /// while the name ending right before it will be returned. 
    ///
    /// # Panics
    ///
    /// The method panics if the position is not the beginning of a label
    /// or is beyond the end of the name.
    pub fn split_to(&mut self, mid: usize) -> Self {
        self.check_index(mid);
        let left = self.octets.range_to(mid);
        self.octets = self.octets.range_from(mid);
        unsafe { Self::from_octets_unchecked(left) }
    }

    /// Truncates the name to the given length.
    ///
    /// # Panics
    ///
    /// The method panics if the position is not the beginning of a label
    /// or is beyond the end of the name.
    pub fn truncate(&mut self, len: usize) {
        self.check_index(len);
        self.octets = self.octets.range_to(len);
    }

    /// Splits off the first label.
    ///
    /// If there is at least one label in the name, returns the first label
    /// as a relative domain name with exactly one label and makes `self`
    /// contain the domain name starting after that first label. If the name
    /// is empty, returns `None`.
    pub fn split_first(&mut self) -> Option<Self> {
        if self.is_empty() {
            return None
        }
        let first_end = match self.iter().next() {
            Some(label) => label.len() + 1,
            None => return None
        };
        Some(self.split_to(first_end))
    }

    /// Reduces the name to its parent.
    ///
    /// Returns whether that actually happened, since if the name is already
    /// empty it can’t.
    pub fn parent(&mut self) -> bool {
        self.split_first().is_some()
    }

    /// Strips the suffix `base` from the domain name.
    ///
    /// This will fail if `base` isn’t actually a suffix, i.e., if
    /// [`ends_with`] doesn’t return `true`.
    ///
    /// [`ends_with`]: #method.ends_with
    pub fn strip_suffix<N: ToRelativeDname>(&mut self, base: &N)
                                            -> Result<(), StripSuffixError> {
        if self.ends_with(base) {
            let idx = self.octets.len() - base.compose_len();
            self.split_off(idx);
            Ok(())
        }
        else {
            Err(StripSuffixError)
        }
    }
}


//--- Compose

impl<O: Octets> Compose for RelativeDname<O> {
    fn compose_len(&self) -> usize {
        self.octets.len()
    }

    fn compose<B: BufMut>(&self, buf: &mut B) {
        buf.put_slice(self.as_ref())
    }
}


//--- ToLabelIter and ToRelativeDname

impl<'a, O: Octets> ToLabelIter<'a> for RelativeDname<O> {
    type LabelIter = DnameIter<'a>;

    fn iter_labels(&'a self) -> Self::LabelIter {
        self.iter()
    }
}

impl<O: Octets> ToRelativeDname for RelativeDname<O> {
    fn to_name(&self) -> RelativeDname<Bytes> {
        unsafe {
            RelativeDname::from_octets_unchecked(
                self.octets.clone().into_bytes()
            )
        }
    }
    
    fn as_flat_slice(&self) -> Option<&[u8]> {
        Some(self.as_slice())
    }
}


//--- Deref and AsRef

impl<O: Octets> ops::Deref for RelativeDname<O> {
    type Target = O;

    fn deref(&self) -> &O {
        self.as_octets()
    }
}

impl<O: Octets> AsRef<[u8]> for RelativeDname<O> {
    fn as_ref(&self) -> &[u8] {
        self.as_slice()
    }
}


//--- IntoIterator

impl<'a, O: Octets> IntoIterator for &'a RelativeDname<O> {
    type Item = &'a Label;
    type IntoIter = DnameIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}


//--- PartialEq and Eq

impl<N: ToRelativeDname, O: Octets> PartialEq<N> for RelativeDname<O> {
    fn eq(&self, other: &N) -> bool {
        self.name_eq(other)
    }
}

impl<O: Octets> Eq for RelativeDname<O> { }


//--- PartialOrd and Ord

impl<N: ToRelativeDname, O: Octets> PartialOrd<N> for RelativeDname<O> {
    fn partial_cmp(&self, other: &N) -> Option<cmp::Ordering> {
        Some(self.name_cmp(other))
    }
}

impl<O: Octets> Ord for RelativeDname<O> {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        self.name_cmp(other)
    }
}


//--- Hash

impl<O: Octets> hash::Hash for RelativeDname<O> {
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        for item in self.iter() {
            item.hash(state)
        }
    }
}


//--- Display and Debug

impl<O: Octets> fmt::Display for RelativeDname<O> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut iter = self.iter();
        match iter.next() {
            Some(label) => label.fmt(f)?,
            None => return Ok(())
        }
        for label in iter {
            f.write_str(".")?;
            label.fmt(f)?;
        }
        Ok(())
    }
}

impl<O: Octets> fmt::Debug for RelativeDname<O> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "RelativeDname({})", self)
    }
}


//------------ DnameIter -----------------------------------------------------

/// An iterator over the labels in an uncompressed name.
#[derive(Clone, Debug)]
pub struct DnameIter<'a> {
    slice: &'a [u8],
}

impl<'a> DnameIter<'a> {
    pub(super) fn new(slice: &'a [u8]) -> Self {
        DnameIter { slice }
    }
}

impl<'a> Iterator for DnameIter<'a> {
    type Item = &'a Label;

    fn next(&mut self) -> Option<Self::Item> {
        let (label, tail) = match Label::split_from(self.slice) {
            Ok(res) => res,
            Err(_) => return None,
        };
        self.slice = tail;
        Some(label)
    }
}

impl<'a> DoubleEndedIterator for DnameIter<'a> {
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.slice.is_empty() {
            return None
        }
        let mut tmp = self.slice;
        loop {
            let (label, tail) = Label::split_from(tmp).unwrap();
            if tail.is_empty() {
                let end = self.slice.len() - (label.len() + 1);
                self.slice = &self.slice[..end];
                return Some(label)
            }
            else {
                tmp = tail
            }
        }
    }
}


//------------ RelativeDnameError --------------------------------------------

/// An error happened while creating a domain name from octets.
#[derive(Clone, Copy, Debug, Eq, Fail, PartialEq)]
pub enum RelativeDnameError {
    /// A bad label was encountered.
    #[fail(display="{}", _0)]
    BadLabel(LabelTypeError),

    /// A compressed name was encountered.
    #[fail(display="compressed domain name")]
    CompressedName,

    /// The data ended before the end of a label.
    #[fail(display="unexpected end of input")]
    ShortData,

    /// The domain name was longer than 255 octets.
    #[fail(display="long domain name")]
    LongName,

    /// The root label was encountered.
    #[fail(display="absolute domain name")]
    AbsoluteName,
}

impl From<LabelTypeError> for RelativeDnameError {
    fn from(err: LabelTypeError) -> Self {
        RelativeDnameError::BadLabel(err)
    }
}

impl From<SplitLabelError> for RelativeDnameError {
    fn from(err: SplitLabelError) -> Self {
        match err {
            SplitLabelError::Pointer(_) => RelativeDnameError::CompressedName,
            SplitLabelError::BadType(t) => RelativeDnameError::BadLabel(t),
            SplitLabelError::ShortBuf => RelativeDnameError::ShortData,
        }
    }
}


//------------ StripSuffixError ----------------------------------------------

/// An attempt was made to strip a suffix that wasn’t actually a suffix.
#[derive(Clone, Copy, Debug, Eq, Fail, PartialEq)]
#[fail(display="suffix not found")]
pub struct StripSuffixError;


//============ Testing =======================================================

#[cfg(test)]
mod test {
    use super::*;
    use ::bits::parse::ShortBuf;

    macro_rules! assert_panic {
        ( $cond:expr ) => {
            {
                let result = ::std::panic::catch_unwind(|| $cond);
                assert!(result.is_err());
            }
        }
    }

    #[test]
    fn empty() {
        assert_eq!(RelativeDname::static_empty().as_slice(), b"");
    }

    #[test]
    fn wildcard() {
        assert_eq!(RelativeDname::static_wildcard().as_slice(), b"\x01*");
    }

    #[test]
    fn from_slice() {
        // good names
        assert_eq!(RelativeDname::from_slice(b"").unwrap().as_slice(), b"");
        assert_eq!(RelativeDname::from_slice(b"\x03www").unwrap().as_slice(),
                   b"\x03www");
        assert_eq!(RelativeDname::from_slice(b"\x03www\x07example")
                                 .unwrap().as_slice(),
                   b"\x03www\x07example");

        // absolute names
        assert_eq!(RelativeDname::from_slice(b"\x03www\x07example\x03com\0"),
                   Err(RelativeDnameError::AbsoluteName));
        assert_eq!(RelativeDname::from_slice(b"\0"),
                   Err(RelativeDnameError::AbsoluteName));

        // bytes shorter than what label length says.
        assert_eq!(Dname::from_slice(b"\x03www\x07exa"),
                   Err(ShortBuf.into()));

        // label 63 long ok, 64 bad.
        let mut slice = [0u8; 64];
        slice[0] = 63;
        assert!(RelativeDname::from_slice(&slice[..]).is_ok());
        let mut slice = [0u8; 65];
        slice[0] = 64;
        assert!(RelativeDname::from_slice(&slice[..]).is_err());

        // name 254 long ok, 255 bad.
        let mut buf = Vec::new();
        for _ in 0..25 {
            buf.extend_from_slice(b"\x09123456789");
        }
        assert_eq!(buf.len(), 250);
        let mut tmp = buf.clone();
        tmp.extend_from_slice(b"\x03123");
        assert_eq!(RelativeDname::from_slice(&tmp).map(|_| ()), Ok(()));
        buf.extend_from_slice(b"\x041234");
        assert!(RelativeDname::from_slice(&buf).is_err());
        
        // bad label heads: compressed, other types.
        assert_eq!(RelativeDname::from_slice(b"\xa2asdasds"),
                   Err(LabelTypeError::Undefined.into()));
        assert_eq!(RelativeDname::from_slice(b"\x62asdasds"),
                   Err(LabelTypeError::Extended(0x62).into()));
        assert_eq!(RelativeDname::from_slice(b"\xccasdasds"),
                   Err(RelativeDnameError::CompressedName.into()));
    }

    #[test]
    fn into_absolute() {
        assert_eq!(
            RelativeDname::from_bytes(
                Bytes::from_static(b"\x03www\x07example\x03com")
            ).unwrap().into_absolute().as_slice(),
            b"\x03www\x07example\x03com\0"
        );
    }

    // chain is tested with the Chain type.

    #[test]
    fn chain_root() {
        assert_eq!(Dname::from_slice(b"\x03www\x07example\x03com\0").unwrap(),
                   RelativeDname::from_slice(b"\x03www\x07example\x03com")
                                 .unwrap().chain_root());
    }

    #[test]
    fn iter() {
        use ::bits::name::dname::test::cmp_iter;

        cmp_iter(RelativeDname::static_empty().iter(), &[]);
        cmp_iter(RelativeDname::static_wildcard().iter(), &[b"*"]);
        cmp_iter(RelativeDname::from_slice(b"\x03www\x07example\x03com")
                               .unwrap().iter(),
                 &[b"www", b"example", b"com"]);
    }

    #[test]
    fn iter_back() {
        use ::bits::name::dname::test::cmp_iter_back;

        cmp_iter_back(RelativeDname::static_empty().iter(), &[]);
        cmp_iter_back(RelativeDname::static_wildcard().iter(), &[b"*"]);
        cmp_iter_back(RelativeDname::from_slice(b"\x03www\x07example\x03com")
                                    .unwrap().iter(),
                      &[b"com", b"example", b"www"]);
    }

    #[test]
    fn label_count() {
        assert_eq!(RelativeDname::static_empty().label_count(), 0);
        assert_eq!(RelativeDname::static_wildcard().label_count(), 1);
        assert_eq!(RelativeDname::from_slice(b"\x03www\x07example\x03com")
                                 .unwrap().label_count(),
                   3);
    }

    #[test]
    fn first() {
        assert_eq!(RelativeDname::static_empty().first(), None);
        assert_eq!(RelativeDname::from_slice(b"\x03www").unwrap()
                                 .first().unwrap().as_slice(),
                   b"www");
        assert_eq!(RelativeDname::from_slice(b"\x03www\x07example").unwrap()
                                 .first().unwrap().as_slice(),
                   b"www");
    }

    #[test]
    fn last() {
        assert_eq!(RelativeDname::static_empty().last(), None);
        assert_eq!(RelativeDname::from_slice(b"\x03www").unwrap()
                                 .last().unwrap().as_slice(),
                   b"www");
        assert_eq!(RelativeDname::from_slice(b"\x03www\x07example").unwrap()
                                 .last().unwrap().as_slice(),
                   b"example");
    }

    #[test]
    fn ndots() {
        assert_eq!(RelativeDname::static_empty().ndots(), 0);
        assert_eq!(RelativeDname::from_slice(b"\x03www").unwrap().ndots(),
                   0);
        assert_eq!(RelativeDname::from_slice(b"\x03www\x07example").unwrap()
                                 .ndots(),
                   1);
    }

    #[test]
    fn starts_with() {
        let matrix = [
            ( RelativeDname::static_empty(),
              [ true, false, false, false, false, false ]),
            ( RelativeDname::from_slice(b"\x03www").unwrap(),
              [ true, true, false, false, false, false ]),
            ( RelativeDname::from_slice(b"\x03www\x07example").unwrap(),
              [ true, true, true, false, false, false ]),
            ( RelativeDname::from_slice(b"\x03www\x07example\x03com").unwrap(),
              [ true, true, true, true, false, false ]),
            ( RelativeDname::from_slice(b"\x07example\x03com").unwrap(),
              [ true, false, false, false, true, false ]),
            ( RelativeDname::from_slice(b"\x03com").unwrap(),
              [ true, false, false, false, false, true ])
        ];
        for i in 0..6 {
            for j in 0..6 {
                assert_eq!(matrix[i].0.starts_with(&matrix[j].0),
                           matrix[i].1[j],
                           "i={}, j={}", i, j)
            }
        }
    }

    #[test]
    fn ends_with() {
        let matrix = [
            ( RelativeDname::static_empty(),
              [ true, false, false, false, false, false ]),
            ( RelativeDname::from_slice(b"\x03www").unwrap(),
              [ true, true, false, false, false, false ]),
            ( RelativeDname::from_slice(b"\x03www\x07example").unwrap(),
              [ true, false, true, false, false, false ]),
            ( RelativeDname::from_slice(b"\x03www\x07example\x03com").unwrap(),
              [ true, false, false, true, true, true]),
            ( RelativeDname::from_slice(b"\x07example\x03com").unwrap(),
              [ true, false, false, false, true, true]),
            ( RelativeDname::from_slice(b"\x03com").unwrap(),
              [ true, false, false, false, false, true ]),
        ];
        for i in 0..matrix.len() {
            for j in 0..matrix.len() {
                assert_eq!(matrix[i].0.ends_with(&matrix[j].0),
                           matrix[i].1[j],
                           "i={}, j={}", i, j)
            }
        }
    }

    #[test]
    fn is_label_start() {
        let wec = RelativeDname::from_slice(b"\x03www\x07example\x03com")
                                .unwrap();

        assert!( wec.is_label_start(0)); // \x03
        assert!(!wec.is_label_start(1)); // w
        assert!(!wec.is_label_start(2)); // w
        assert!(!wec.is_label_start(3)); // w
        assert!( wec.is_label_start(4)); // \x07
        assert!(!wec.is_label_start(5)); // e
        assert!(!wec.is_label_start(6)); // x
        assert!(!wec.is_label_start(7)); // a
        assert!(!wec.is_label_start(8)); // m
        assert!(!wec.is_label_start(9)); // p
        assert!(!wec.is_label_start(10)); // l
        assert!(!wec.is_label_start(11)); // e
        assert!( wec.is_label_start(12)); // \x03
        assert!(!wec.is_label_start(13)); // c
        assert!(!wec.is_label_start(14)); // o
        assert!(!wec.is_label_start(15)); // m
        assert!( wec.is_label_start(16)); // empty label
        assert!(!wec.is_label_start(17)); // 
    }

    #[test]
    fn range() {
        let wec = RelativeDname::from_slice(b"\x03www\x07example\x03com")
                                .unwrap();
        assert_eq!(wec.range(0, 4).as_slice(), b"\x03www");
        assert_eq!(wec.range(0, 12).as_slice(), b"\x03www\x07example");
        assert_eq!(wec.range(4, 12).as_slice(), b"\x07example");
        assert_eq!(wec.range(4, 16).as_slice(), b"\x07example\x03com");

        assert_panic!(wec.range(0,3));
        assert_panic!(wec.range(1,4));
        assert_panic!(wec.range(0,11));
        assert_panic!(wec.range(1,12));
        assert_panic!(wec.range(0,17));
        assert_panic!(wec.range(4,17));
        assert_panic!(wec.range(0,18));
    }

    #[test]
    fn range_from() {
        let wec = RelativeDname::from_slice(b"\x03www\x07example\x03com")
                                .unwrap();

        assert_eq!(wec.range_from(0).as_slice(),
                   b"\x03www\x07example\x03com");
        assert_eq!(wec.range_from(4).as_slice(), b"\x07example\x03com");
        assert_eq!(wec.range_from(12).as_slice(), b"\x03com");
        assert_eq!(wec.range_from(16).as_slice(), b"");

        assert_panic!(wec.range_from(17));
        assert_panic!(wec.range_from(18));
    }

    #[test]
    fn range_to() {
        let wec = RelativeDname::from_slice(b"\x03www\x07example\x03com")
                                .unwrap();

        assert_eq!(wec.range_to(0).as_slice(), b"");
        assert_eq!(wec.range_to(4).as_slice(), b"\x03www");
        assert_eq!(wec.range_to(12).as_slice(), b"\x03www\x07example");
        assert_eq!(wec.range_to(16).as_slice(), b"\x03www\x07example\x03com");

        assert_panic!(wec.range_to(17));
        assert_panic!(wec.range_to(18));
    }

    #[test]
    fn split_off() {
        let wec = RelativeDname::from_slice(b"\x03www\x07example\x03com")
                                .unwrap();

        let mut tmp = wec.clone();
        assert_eq!(tmp.split_off(0).as_slice(), b"\x03www\x07example\x03com");
        assert_eq!(tmp.as_slice(), b"");

        let mut tmp = wec.clone();
        assert_eq!(tmp.split_off(4).as_slice(), b"\x07example\x03com");
        assert_eq!(tmp.as_slice(), b"\x03www");

        let mut tmp = wec.clone();
        assert_eq!(tmp.split_off(12).as_slice(), b"\x03com");
        assert_eq!(tmp.as_slice(), b"\x03www\x07example");

        let mut tmp = wec.clone();
        assert_eq!(tmp.split_off(16).as_slice(), b"");
        assert_eq!(tmp.as_slice(), b"\x03www\x07example\x03com");

        assert_panic!(wec.clone().split_off(1));
        assert_panic!(wec.clone().split_off(14));
        assert_panic!(wec.clone().split_off(17));
        assert_panic!(wec.clone().split_off(18));
    }

    #[test]
    fn split_to() {
        let wec = RelativeDname::from_slice(b"\x03www\x07example\x03com")
                                .unwrap();

        let mut tmp = wec.clone();
        assert_eq!(tmp.split_to(0).as_slice(), b"");
        assert_eq!(tmp.as_slice(), b"\x03www\x07example\x03com");

        let mut tmp = wec.clone();
        assert_eq!(tmp.split_to(4).as_slice(), b"\x03www");
        assert_eq!(tmp.as_slice(), b"\x07example\x03com");

        let mut tmp = wec.clone();
        assert_eq!(tmp.split_to(12).as_slice(), b"\x03www\x07example");
        assert_eq!(tmp.as_slice(), b"\x03com");

        let mut tmp = wec.clone();
        assert_eq!(tmp.split_to(16).as_slice(), b"\x03www\x07example\x03com");
        assert_eq!(tmp.as_slice(), b"");

        assert_panic!(wec.clone().split_to(1));
        assert_panic!(wec.clone().split_to(14));
        assert_panic!(wec.clone().split_to(17));
        assert_panic!(wec.clone().split_to(18));
    }

    #[test]
    fn truncate() {
        let wec = RelativeDname::from_slice(b"\x03www\x07example\x03com")
                                .unwrap();

        let mut tmp = wec.clone();
        tmp.truncate(0);
        assert_eq!(tmp.as_slice(), b"");

        let mut tmp = wec.clone();
        tmp.truncate(4);
        assert_eq!(tmp.as_slice(), b"\x03www");

        let mut tmp = wec.clone();
        tmp.truncate(12);
        assert_eq!(tmp.as_slice(), b"\x03www\x07example");

        let mut tmp = wec.clone();
        tmp.truncate(16);
        assert_eq!(tmp.as_slice(), b"\x03www\x07example\x03com");
        
        assert_panic!(wec.clone().truncate(1));
        assert_panic!(wec.clone().truncate(14));
        assert_panic!(wec.clone().truncate(17));
        assert_panic!(wec.clone().truncate(18));
    }

    #[test]
    fn split_first() {
        let mut wec = RelativeDname::from_slice(b"\x03www\x07example\x03com")
                                    .unwrap();

        assert_eq!(wec.split_first().unwrap().as_slice(), b"\x03www");
        assert_eq!(wec.as_slice(), b"\x07example\x03com");
        assert_eq!(wec.split_first().unwrap().as_slice(), b"\x07example");
        assert_eq!(wec.as_slice(), b"\x03com");
        assert_eq!(wec.split_first().unwrap().as_slice(), b"\x03com");
        assert_eq!(wec.as_slice(), b"");
        assert!(wec.split_first().is_none());
        assert_eq!(wec.as_slice(), b"");
        assert!(wec.split_first().is_none());
        assert_eq!(wec.as_slice(), b"");
    }

    #[test]
    fn parent() {
        let mut wec = RelativeDname::from_slice(b"\x03www\x07example\x03com")
                                    .unwrap();

        assert!(wec.parent());
        assert_eq!(wec.as_slice(), b"\x07example\x03com");
        assert!(wec.parent());
        assert_eq!(wec.as_slice(), b"\x03com");
        assert!(wec.parent());
        assert_eq!(wec.as_slice(), b"");
        assert!(!wec.parent());
        assert_eq!(wec.as_slice(), b"");
        assert!(!wec.parent());
        assert_eq!(wec.as_slice(), b"");
    }

    #[test]
    fn strip_suffix() {
        let wec = RelativeDname::from_slice(b"\x03www\x07example\x03com")
                                .unwrap();
        let ec = RelativeDname::from_slice(b"\x07example\x03com").unwrap();
        let c = RelativeDname::from_slice(b"\x03com").unwrap();
        let wen = RelativeDname::from_slice(b"\x03www\x07example\x03net")
                                .unwrap();
        let en = RelativeDname::from_slice(b"\x07example\x03net").unwrap();
        let n = RelativeDname::from_slice(b"\x03net").unwrap();

        let mut tmp = wec.clone();
        assert_eq!(tmp.strip_suffix(&wec), Ok(()));
        assert_eq!(tmp.as_slice(), b"");

        let mut tmp = wec.clone();
        assert_eq!(tmp.strip_suffix(&ec), Ok(()));
        assert_eq!(tmp.as_slice(), b"\x03www");

        let mut tmp = wec.clone();
        assert_eq!(tmp.strip_suffix(&c), Ok(()));
        assert_eq!(tmp.as_slice(), b"\x03www\x07example");

        let mut tmp = wec.clone();
        assert_eq!(tmp.strip_suffix(&RelativeDname::static_empty()), Ok(()));
        assert_eq!(tmp.as_slice(), b"\x03www\x07example\x03com");

        assert_eq!(wec.clone().strip_suffix(&wen), Err(StripSuffixError));
        assert_eq!(wec.clone().strip_suffix(&en), Err(StripSuffixError));
        assert_eq!(wec.clone().strip_suffix(&n), Err(StripSuffixError));
    }

    // No test for Compose since the implementation is so simple.

    #[test]
    fn eq() {
        assert_eq!(
            RelativeDname::from_slice(b"\x03www\x07example\x03com").unwrap(),
            RelativeDname::from_slice(b"\x03www\x07example\x03com").unwrap()
        );
        assert_eq!(
            RelativeDname::from_slice(b"\x03www\x07example\x03com").unwrap(),
            RelativeDname::from_slice(b"\x03wWw\x07eXAMple\x03Com").unwrap()
        );
        assert_eq!(
            RelativeDname::from_slice(b"\x03www\x07example\x03com").unwrap(),
            RelativeDname::from_slice(b"\x03www").unwrap()
                .chain(RelativeDname::from_slice(b"\x07example\x03com")
                                     .unwrap())
                .unwrap()
        );
        assert_eq!(
            RelativeDname::from_slice(b"\x03www\x07example\x03com").unwrap(),
            RelativeDname::from_slice(b"\x03wWw").unwrap()
                .chain(RelativeDname::from_slice(b"\x07eXAMple\x03coM")
                                     .unwrap())
                .unwrap()
        );

        assert_ne!(
            RelativeDname::from_slice(b"\x03www\x07example\x03com").unwrap(),
            RelativeDname::from_slice(b"\x03ww4\x07example\x03com").unwrap()
        );
        assert_ne!(
            RelativeDname::from_slice(b"\x03www\x07example\x03com").unwrap(),
            RelativeDname::from_slice(b"\x03www").unwrap()
                .chain(RelativeDname::from_slice(b"\x073xample\x03com")
                                     .unwrap())
                .unwrap()
        );
    }

    #[test]
    fn cmp() {
        use std::cmp::Ordering;

        // The following is taken from section 6.1 of RFC 4034.
        let names = [
            RelativeDname::from_slice(b"\x07example").unwrap(),
            RelativeDname::from_slice(b"\x01a\x07example").unwrap(),
            RelativeDname::from_slice(b"\x08yljkjljk\x01a\x07example").unwrap(),
            RelativeDname::from_slice(b"\x01Z\x01a\x07example").unwrap(),
            RelativeDname::from_slice(b"\x04zABC\x01a\x07example").unwrap(),
            RelativeDname::from_slice(b"\x01z\x07example").unwrap(),
            RelativeDname::from_slice(b"\x01\x01\x01z\x07example").unwrap(),
            RelativeDname::from_slice(b"\x01*\x01z\x07example").unwrap(),
            RelativeDname::from_slice(b"\x01\xc8\x01z\x07example").unwrap(),
        ];
        for i in 0..names.len() {
            for j in 0..names.len() {
                let ord = if i < j { Ordering::Less }
                          else if i == j { Ordering::Equal }
                          else { Ordering::Greater };
                assert_eq!(names[i].partial_cmp(&names[j]), Some(ord));
                assert_eq!(names[i].cmp(&names[j]), ord);
            }
        }

        let n1 = RelativeDname::from_slice(b"\x03www\x07example\x03com")
                               .unwrap();
        let n2 = RelativeDname::from_slice(b"\x03wWw\x07eXAMple\x03Com")
                                .unwrap();
        assert_eq!(n1.partial_cmp(&n2), Some(Ordering::Equal));
        assert_eq!(n1.cmp(&n2), Ordering::Equal);
    }

    #[test]
    fn hash() {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut s1 = DefaultHasher::new();
        let mut s2 = DefaultHasher::new();
        RelativeDname::from_slice(b"\x03www\x07example\x03com").unwrap()
                       .hash(&mut s1);
        RelativeDname::from_slice(b"\x03wWw\x07eXAMple\x03Com").unwrap()
                       .hash(&mut s2);
        assert_eq!(s1.finish(), s2.finish());
    }

    // Display and Debug skipped for now.
}

