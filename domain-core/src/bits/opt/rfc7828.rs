//! EDNS Options from RFC 7828

use bytes::BufMut;
use crate::bits::compose::Compose;
use crate::bits::message_builder::OptBuilder;
use crate::bits::octets::Octets;
use crate::bits::parse::{ParseAll, Parser, ParseAllError, ShortBuf};
use crate::iana::OptionCode;
use super::CodeOptData;


//------------ TcpKeepalive --------------------------------------------------

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct TcpKeepalive(u16);

impl TcpKeepalive {
    pub fn new(timeout: u16) -> Self {
        TcpKeepalive(timeout)
    }

    pub fn push(builder: &mut OptBuilder, timeout: u16)
                -> Result<(), ShortBuf> {
        builder.push(&Self::new(timeout))
    }

    pub fn timeout(self) -> u16 {
        self.0
    }
}


//--- ParseAll and Compose

impl<O: Octets> ParseAll<O> for TcpKeepalive {
    type Err = ParseAllError;

    fn parse_all(
        parser: &mut Parser<O>,
        len: usize
    ) -> Result<Self, Self::Err> {
        u16::parse_all(parser, len).map(Self::new)
    }
}

impl Compose for TcpKeepalive {
    fn compose_len(&self) -> usize {
        2
    }

    fn compose<B: BufMut>(&self, buf: &mut B) {
        self.0.compose(buf)
    }
}


//--- CodeOptData

impl CodeOptData for TcpKeepalive {
    const CODE: OptionCode = OptionCode::TcpKeepalive;
}

