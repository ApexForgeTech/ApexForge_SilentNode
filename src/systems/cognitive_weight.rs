use crate::domain::{ContractState, NodeData, SilentContract};

/// Breakdown of what contributes to cognitive weight.
#[derive(Debug, Clone, Default)]
pub struct WeightReport {
    /// Normalised total weight [0, 100].
    pub total: f32,
    pub ghost_nodes: usize,
    pub fossil_nodes: usize,
    pub void_nodes: usize,
    pub pending_contracts: usize,
    pub active_contracts: usize,
    /// Nodes with no connections (isolated — a silent burden).
    pub isolated_nodes: usize,
}

impl WeightReport {
    /// "Heavy" if total > 50.
    pub fn is_heavy(&self) -> bool {
        self.total > 50.0
    }

    /// Suggested visual density multiplier for the renderer (1.0 = normal, 1.5 = dense dark).
    pub fn visual_density(&self) -> f32 {
        1.0 + (self.total / 100.0) * 0.6
    }

    /// Particle drag multiplier — heavy universe = slower particles.
    pub fn particle_drag(&self) -> f32 {
        1.0 + (self.total / 100.0) * 1.5
    }

    pub fn summary(&self) -> String {
        format!(
            "Weight: {:.1}/100 | ghosts={} fossils={} void={} contracts={}/{} isolated={}",
            self.total,
            self.ghost_nodes,
            self.fossil_nodes,
            self.void_nodes,
            self.active_contracts,
            self.pending_contracts,
            self.isolated_nodes,
        )
    }
}

pub struct CognitiveWeightSystem;

impl CognitiveWeightSystem {
    pub fn new() -> Self {
        Self
    }

    /// Calculate cognitive weight from the current graph + contracts.
    pub fn calculate(
        &self,
        nodes: &[&NodeData],
        contracts: &[SilentContract],
        isolated_node_ids: &[uuid::Uuid],
    ) -> WeightReport {
        let ghost_nodes = nodes.iter().filter(|n| n.is_ghost).count();
        let fossil_nodes = nodes.iter().filter(|n| n.is_fossil).count();
        let void_nodes = nodes.iter().filter(|n| n.is_void).count();

        let pending_contracts = contracts
            .iter()
            .filter(|c| c.state == ContractState::Dormant || c.state == ContractState::Awakening)
            .count();
        let active_contracts = contracts
            .iter()
            .filter(|c| c.state == ContractState::Active)
            .count();
        let isolated_nodes = isolated_node_ids.len();

        let raw = ghost_nodes as f32 * 2.0
            + fossil_nodes as f32 * 1.0
            + void_nodes as f32 * 1.5
            + pending_contracts as f32 * 2.5
            + active_contracts as f32 * 3.5
            + isolated_nodes as f32 * 0.8;

        let total = raw.min(100.0);

        WeightReport {
            total,
            ghost_nodes,
            fossil_nodes,
            void_nodes,
            pending_contracts,
            active_contracts,
            isolated_nodes,
        }
    }
}

impl Default for CognitiveWeightSystem {
    fn default() -> Self {
        Self::new()
    }
}
