use crate::domain::{EdgeType, NodeData, NodeType};
use crate::graph::CognitiveGraph;
use crate::GraphError;
use std::collections::HashSet;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct SimilarNodeSuggestion {
    pub node_id: Uuid,
    pub score: f32,
    pub content_preview: String,
}

#[derive(Debug, Clone)]
pub struct MaterializationResult {
    pub node_id: Uuid,
    pub node_type: NodeType,
    pub suggestions: Vec<SimilarNodeSuggestion>,
}

#[derive(Debug, Clone)]
pub struct MaterializationEngine {
    similarity_threshold: f32,
    auto_connect: bool,
}

impl Default for MaterializationEngine {
    fn default() -> Self {
        Self {
            similarity_threshold: 0.35,
            auto_connect: true,
        }
    }
}

impl MaterializationEngine {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn materialize(
        &self,
        raw_text: &str,
        graph: &mut CognitiveGraph,
    ) -> Result<MaterializationResult, GraphError> {
        let node_type = infer_node_type(raw_text);
        let existing_nodes = graph.nodes().cloned().collect::<Vec<_>>();
        let tokens = tokenize(raw_text);

        let mut suggestions = existing_nodes
            .iter()
            .filter_map(|candidate| {
                let score = similarity_score(&tokens, &tokenize(&candidate.content));
                (score >= self.similarity_threshold).then(|| SimilarNodeSuggestion {
                    node_id: candidate.id,
                    score,
                    content_preview: candidate.content.clone(),
                })
            })
            .collect::<Vec<_>>();
        suggestions.sort_by(|left, right| right.score.total_cmp(&left.score));

        let node = NodeData::new(node_type, raw_text);
        let node_id = node.id;
        graph.add_node(node)?;

        if self.auto_connect {
            for suggestion in suggestions.iter().take(3) {
                graph.connect(
                    node_id,
                    suggestion.node_id,
                    EdgeType::Resonance,
                    suggestion.score,
                )?;
            }
        }

        Ok(MaterializationResult {
            node_id,
            node_type,
            suggestions,
        })
    }
}

fn infer_node_type(raw_text: &str) -> NodeType {
    let lower = raw_text.to_lowercase();
    if lower.contains("project") || lower.contains("build") || lower.contains("roadmap") {
        NodeType::Project
    } else if lower.contains("remember") || lower.contains("memory") || lower.contains("journal") {
        NodeType::Memory
    } else if lower.contains("person") || lower.contains("team") || lower.contains("friend") {
        NodeType::Person
    } else if lower.contains("file") || lower.contains("artifact") || lower.contains("spec") {
        NodeType::Artifact
    } else {
        NodeType::Idea
    }
}

fn tokenize(input: &str) -> HashSet<String> {
    input
        .split(|ch: char| !ch.is_alphanumeric())
        .filter(|token| token.len() > 2)
        .map(|token| token.to_lowercase())
        .collect()
}

fn similarity_score(left: &HashSet<String>, right: &HashSet<String>) -> f32 {
    if left.is_empty() || right.is_empty() {
        return 0.0;
    }

    let intersection = left.intersection(right).count() as f32;
    let union = left.union(right).count() as f32;
    intersection / union
}
