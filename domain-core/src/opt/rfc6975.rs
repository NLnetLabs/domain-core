//! EDNS Options from RFC 6975.

use core::slice;
use crate::compose::{Compose, ComposeTarget};
use crate::iana::{OptionCode, SecAlg};
// XXX use crate::message_builder::OptBuilder;
use crate::parse::{ParseAll, Parser, ParseSource, ShortBuf};
use super::CodeOptData;


//------------ Dau, Dhu, N3u -------------------------------------------------

macro_rules! option_type {
    ( $name:ident ) => {
        #[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        pub struct $name<Octets> {
            octets: Octets,
        }

        impl<Octets> $name<Octets> {
            pub fn from_octets(octets: Octets) -> Self {
                $name { octets }
            }

            pub fn iter(&self) -> SecAlgsIter
            where Octets: AsRef<[u8]> {
                SecAlgsIter::new(self.octets.as_ref())
            }

            /* XXX
            pub fn push(builder: &mut OptBuilder, algs: &[SecAlg])
                        -> Result<(), ShortBuf> {
                assert!(algs.len() <= ::std::u16::MAX as usize);
                builder.build(OptionCode::$name, algs.len() as u16, |buf| {
                    for alg in algs {
                        buf.compose(&alg.to_int())?
                    }
                    Ok(())
                })
            }
            */
        }

        //--- ParseAll, Compose

        impl<Octets: ParseSource> ParseAll<Octets> for $name<Octets> {
            type Err = ShortBuf;

            fn parse_all(
                parser: &mut Parser<Octets>,
                len: usize
            ) -> Result<Self, Self::Err> {
                parser.parse_octets(len).map(Self::from_octets)
            }
        }

        impl<Octets: AsRef<[u8]>> Compose for $name<Octets> {
            fn compose<T: ComposeTarget + ?Sized>(&self, target: &mut T) {
                target.append_slice(self.octets.as_ref())
            }
        }


        //--- CodeOptData
        
        impl<Octets> CodeOptData for $name<Octets> {
            const CODE: OptionCode = OptionCode::$name;
        }

        
        //--- IntoIter

        impl<'a, Octets: AsRef<[u8]>> IntoIterator for &'a $name<Octets> {
            type Item = SecAlg;
            type IntoIter = SecAlgsIter<'a>;

            fn into_iter(self) -> Self::IntoIter {
                self.iter()
            }
        }
    }
}

option_type!(Dau);
option_type!(Dhu);
option_type!(N3u);


//------------ SecAlgsIter ---------------------------------------------------

pub struct SecAlgsIter<'a>(slice::Iter<'a, u8>);

impl<'a> SecAlgsIter<'a> {
    fn new(slice: &'a [u8]) -> Self {
        SecAlgsIter(slice.iter())
    }
}

impl<'a> Iterator for SecAlgsIter<'a> {
    type Item = SecAlg;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().map(|x| SecAlg::from_int(*x))
    }
}