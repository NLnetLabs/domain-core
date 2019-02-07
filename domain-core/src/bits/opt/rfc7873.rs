//! EDNS Options form RFC 7873

use bytes::BufMut;
use crate::bits::compose::Compose;
use crate::bits::message_builder::OptBuilder;
use crate::bits::octets::Octets;
use crate::bits::parse::{ParseAll, ParseAllError, Parser, ShortBuf};
use crate::iana::OptionCode;
use super::CodeOptData;


//------------ Cookie --------------------------------------------------------

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Cookie([u8; 8]);

impl Cookie {
    pub fn new(cookie: [u8; 8]) -> Self {
        Cookie(cookie)
    }

    pub fn push(builder: &mut OptBuilder, cookie: [u8; 8])
                -> Result<(), ShortBuf> {
        builder.push(&Self::new(cookie))
    }

    pub fn cookie(&self) -> &[u8; 8] {
        &self.0
    }
}


//--- ParseAll and Compose

impl<O: Octets> ParseAll<O> for Cookie {
    type Err = ParseAllError;

    fn parse_all(
        parser: &mut Parser<O>,
        len: usize
    ) -> Result<Self, Self::Err> {
        ParseAllError::check(8, len)?;
        let mut res = [0u8; 8];
        parser.parse_buf(&mut res[..])?;
        Ok(Self::new(res))
    }
}


impl Compose for Cookie {
    fn compose_len(&self) -> usize {
        8
    }

    fn compose<B: BufMut>(&self, buf: &mut B) {
        buf.put_slice(&self.0[..])
    }
}


//--- OptData

impl CodeOptData for Cookie {
    const CODE: OptionCode = OptionCode::Cookie;
}

