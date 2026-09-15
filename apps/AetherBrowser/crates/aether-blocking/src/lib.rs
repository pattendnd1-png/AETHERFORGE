#![forbid(unsafe_code)]
//! Native content-blocking decision contracts.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BlockDecision {
    Allow,
    Block,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RequestContext {
    pub url: String,
    pub top_level_url: Option<String>,
}

pub trait RuleMatcher {
    fn decide(&self, request: &RequestContext) -> BlockDecision;
}
