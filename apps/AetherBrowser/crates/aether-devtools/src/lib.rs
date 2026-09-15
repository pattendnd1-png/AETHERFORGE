#![forbid(unsafe_code)]
//! Engine-neutral developer-tool capability reporting.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DevToolsCapability {
    DomInspection,
    Console,
    NetworkInspection,
    SourceDebugging,
    PerformanceTimeline,
}
