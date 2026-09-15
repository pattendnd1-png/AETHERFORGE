#![forbid(unsafe_code)]

use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
};

use aether_workspace::{GitContextSnapshot, GitContextState};
use uuid::Uuid;

use crate::{ContextSource, RetrievalSignal};

#[derive(Debug, Clone)]
pub(crate) struct RankedCandidate {
    pub chunk_id: Uuid,
    pub source: ContextSource,
    pub content: String,
    pub estimated_tokens: usize,
    pub score_milli: i32,
    pub rationale: BTreeSet<RetrievalSignal>,
}

impl RankedCandidate {
    pub fn new(
        _file_id: Uuid,
        chunk_id: Uuid,
        source: ContextSource,
        content: String,
        estimated_tokens: usize,
    ) -> Self {
        Self {
            chunk_id,
            source,
            content,
            estimated_tokens,
            score_milli: 0,
            rationale: BTreeSet::new(),
        }
    }

    pub fn add_signal(&mut self, signal: RetrievalSignal, points: i32) {
        if self.rationale.insert(signal) {
            self.score_milli = self.score_milli.saturating_add(points);
        }
    }

    pub fn key(&self) -> CandidateKey {
        let range = self.source.range();
        let canonical_path = match &self.source {
            ContextSource::Memory { memory_id, .. } => {
                PathBuf::from(format!("memory://{memory_id}"))
            }
            _ => self.source.canonical_path().to_path_buf(),
        };
        CandidateKey {
            canonical_path,
            start_line: range.start_line,
            end_line: range.end_line,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct CandidateKey {
    canonical_path: PathBuf,
    start_line: u32,
    end_line: u32,
}

pub(crate) fn merge_candidate(
    candidates: &mut BTreeMap<CandidateKey, RankedCandidate>,
    incoming: RankedCandidate,
) {
    let key = incoming.key();
    if let Some(existing) = candidates.get_mut(&key) {
        for signal in incoming.rationale {
            if existing.rationale.insert(signal) {
                existing.score_milli = existing.score_milli.saturating_add(signal.default_points());
            }
        }
        existing.score_milli = existing.score_milli.max(incoming.score_milli);
        if existing.content.is_empty() && !incoming.content.is_empty() {
            existing.content = incoming.content;
            existing.estimated_tokens = incoming.estimated_tokens;
        }
    } else {
        candidates.insert(key, incoming);
    }
}

pub(crate) fn apply_git_boosts(
    candidates: &mut BTreeMap<CandidateKey, RankedCandidate>,
    snapshots: &[GitContextSnapshot],
) {
    for candidate in candidates.values_mut() {
        let path = candidate.source.canonical_path();
        if path.as_os_str().is_empty() {
            continue;
        }
        let boost = git_boost_for_path(path, snapshots);
        if boost > 0 {
            candidate.add_signal(RetrievalSignal::Git, boost);
        }
    }
}

fn git_boost_for_path(path: &std::path::Path, snapshots: &[GitContextSnapshot]) -> i32 {
    let mut recent = false;

    for snapshot in snapshots {
        if snapshot.state != GitContextState::Available {
            continue;
        }

        if snapshot
            .modified_paths
            .iter()
            .chain(snapshot.untracked_paths.iter())
            .any(|relative| snapshot.root.join(relative) == path)
        {
            return 40;
        }

        if snapshot.recent_commits.iter().any(|commit| {
            commit
                .touched_paths
                .iter()
                .any(|relative| snapshot.root.join(relative) == path)
        }) {
            recent = true;
        }
    }

    if recent { 20 } else { 0 }
}

pub(crate) fn sort_candidates(candidates: &mut [RankedCandidate]) {
    candidates.sort_by(|left, right| {
        right
            .score_milli
            .cmp(&left.score_milli)
            .then_with(|| left.source.display_path().cmp(right.source.display_path()))
            .then_with(|| {
                left.source
                    .range()
                    .start_line
                    .cmp(&right.source.range().start_line)
            })
            .then_with(|| left.chunk_id.cmp(&right.chunk_id))
    });
}

#[cfg(test)]
mod task12_tests {
    use super::*;
    use aether_workspace::{GitCommitSummary, GitContextSnapshot, GitContextState};

    #[test]
    fn git_boost_is_bounded_and_prefers_current_changes() {
        let root = PathBuf::from("/project");
        let snapshot = GitContextSnapshot {
            state: GitContextState::Available,
            root: root.clone(),
            branch: Some("main".into()),
            head: Some("abc".into()),
            modified_paths: vec![PathBuf::from("src/current.rs")],
            untracked_paths: Vec::new(),
            recent_commits: vec![GitCommitSummary {
                hash: "abc".into(),
                subject: "recent".into(),
                touched_paths: vec![PathBuf::from("src/recent.rs")],
            }],
        };

        assert_eq!(
            git_boost_for_path(
                &root.join("src/current.rs"),
                std::slice::from_ref(&snapshot)
            ),
            40
        );
        assert_eq!(
            git_boost_for_path(&root.join("src/recent.rs"), std::slice::from_ref(&snapshot)),
            20
        );
        assert_eq!(
            git_boost_for_path(&root.join("src/other.rs"), &[snapshot]),
            0
        );
        assert!(40 < RetrievalSignal::PrefixSymbol.default_points());
    }
}
