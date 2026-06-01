// Phase 13: Dream Engine
// Proposes new edges, ghost revivals, potential node merges, and entropy alerts.
// All proposals are ephemeral — generated fresh from current workspace state.

use crate::intelligence::SuggestionEngine;
use crate::workspace::SilentNodeWorkspace;
use std::collections::HashSet;
use uuid::Uuid;

// ── Proposal types ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum ProposalKind {
    /// Connect two semantically similar nodes that are not yet linked.
    SuggestEdge {
        from: Uuid,
        to: Uuid,
        similarity: f32,
    },
    /// Revive a neglected ghost node that resonates with active nodes.
    ReviveGhost { node_id: Uuid },
    /// Merge two nearly-identical nodes (similarity ≥ 0.80).
    MergeNodes { a: Uuid, b: Uuid, similarity: f32 },
    /// Alert: node has dangerously high entropy.
    EntropyAlert { node_id: Uuid, entropy: f32 },
}

#[derive(Debug, Clone)]
pub struct DreamProposal {
    pub id: Uuid,
    pub kind: ProposalKind,
    pub confidence: f32,
    pub description: String,
}

// ── Engine ────────────────────────────────────────────────────────────────────

pub struct DreamEngine;

impl DreamEngine {
    pub fn new() -> Self {
        Self
    }

    /// Generate all dream proposals for the current workspace state.
    pub fn generate(&self, workspace: &SilentNodeWorkspace) -> Vec<DreamProposal> {
        let mut proposals: Vec<DreamProposal> = Vec::new();

        self.propose_edges(workspace, &mut proposals);
        self.propose_revivals(workspace, &mut proposals);
        self.propose_merges(workspace, &mut proposals);
        self.propose_entropy_alerts(workspace, &mut proposals);

        proposals.sort_by(|a, b| {
            b.confidence
                .partial_cmp(&a.confidence)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        proposals.truncate(20);
        proposals
    }

    fn propose_edges(&self, workspace: &SilentNodeWorkspace, out: &mut Vec<DreamProposal>) {
        let engine = SuggestionEngine::new();
        let node_ids: Vec<Uuid> = workspace.graph.node_ids();
        let mut seen: HashSet<(Uuid, Uuid)> = HashSet::new();

        for &id in node_ids.iter().take(30) {
            if let Some(n) = workspace.graph.get_node(id) {
                if n.is_ghost || n.is_void {
                    continue;
                }
            }

            let related = engine.suggest_related(workspace, id, 5);
            for r in &related {
                if r.similarity < 0.35 {
                    continue;
                }
                // skip if already connected in either direction
                if workspace.graph.get_edge(id, r.node_id).is_some()
                    || workspace.graph.get_edge(r.node_id, id).is_some()
                {
                    continue;
                }
                let key = (id.min(r.node_id), id.max(r.node_id));
                if !seen.insert(key) {
                    continue;
                }

                let from_label = workspace
                    .graph
                    .get_node(id)
                    .map(|n| n.content.as_str())
                    .unwrap_or("?");
                let to_label = workspace
                    .graph
                    .get_node(r.node_id)
                    .map(|n| n.content.as_str())
                    .unwrap_or("?");
                let fl = clip(from_label, 20);
                let tl = clip(to_label, 20);

                out.push(DreamProposal {
                    id: Uuid::new_v4(),
                    confidence: r.similarity,
                    description: format!("Connect «{fl}» ↔ «{tl}»  sim={:.2}", r.similarity),
                    kind: ProposalKind::SuggestEdge {
                        from: id,
                        to: r.node_id,
                        similarity: r.similarity,
                    },
                });
            }
        }
    }

    fn propose_revivals(&self, workspace: &SilentNodeWorkspace, out: &mut Vec<DreamProposal>) {
        let ghosts: Vec<Uuid> = workspace
            .graph
            .nodes()
            .filter(|n| n.is_ghost)
            .map(|n| n.id)
            .collect();

        for ghost_id in ghosts {
            let ghost = match workspace.graph.get_node(ghost_id) {
                Some(g) => g,
                None => continue,
            };

            let active_neighbors = workspace
                .graph
                .neighbors(ghost_id)
                .unwrap_or_default()
                .iter()
                .filter(|n| !n.is_ghost && !n.is_fossil)
                .count();

            let confidence =
                (ghost.gravity * 0.25 + active_neighbors as f32 * 0.15).clamp(0.0, 1.0);
            if confidence < 0.05 {
                continue;
            }

            let label = clip(&ghost.content, 25);
            out.push(DreamProposal {
                id: Uuid::new_v4(),
                confidence,
                description: format!("Revive «{label}»  {active_neighbors} active neighbors"),
                kind: ProposalKind::ReviveGhost { node_id: ghost_id },
            });
        }
    }

    fn propose_merges(&self, workspace: &SilentNodeWorkspace, out: &mut Vec<DreamProposal>) {
        let engine = SuggestionEngine::new();
        let node_ids: Vec<Uuid> = workspace.graph.node_ids();
        let mut seen: HashSet<(Uuid, Uuid)> = HashSet::new();

        for &id in node_ids.iter().take(25) {
            let related = engine.suggest_related(workspace, id, 5);
            for r in &related {
                if r.similarity < 0.80 {
                    continue;
                }
                let key = (id.min(r.node_id), id.max(r.node_id));
                if !seen.insert(key) {
                    continue;
                }

                let a_label = workspace
                    .graph
                    .get_node(id)
                    .map(|n| n.content.as_str())
                    .unwrap_or("?");
                let b_label = workspace
                    .graph
                    .get_node(r.node_id)
                    .map(|n| n.content.as_str())
                    .unwrap_or("?");
                let al = clip(a_label, 18);
                let bl = clip(b_label, 18);

                out.push(DreamProposal {
                    id: Uuid::new_v4(),
                    confidence: r.similarity,
                    description: format!("Merge «{al}» + «{bl}»  sim={:.2}", r.similarity),
                    kind: ProposalKind::MergeNodes {
                        a: id,
                        b: r.node_id,
                        similarity: r.similarity,
                    },
                });
            }
        }
    }

    fn propose_entropy_alerts(
        &self,
        workspace: &SilentNodeWorkspace,
        out: &mut Vec<DreamProposal>,
    ) {
        for node in workspace.graph.nodes() {
            if node.entropy < 0.75 {
                continue;
            }
            let label = clip(&node.content, 25);
            out.push(DreamProposal {
                id: Uuid::new_v4(),
                confidence: node.entropy,
                description: format!("High entropy «{label}»  η={:.2}", node.entropy),
                kind: ProposalKind::EntropyAlert {
                    node_id: node.id,
                    entropy: node.entropy,
                },
            });
        }
    }
}

impl Default for DreamEngine {
    fn default() -> Self {
        Self::new()
    }
}

fn clip(s: &str, max: usize) -> &str {
    if s.len() <= max {
        s
    } else {
        &s[..max]
    }
}
