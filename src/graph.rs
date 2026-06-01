use crate::domain::{EdgeData, EdgeType, GraphStats, NodeData};
use crate::error::GraphError;
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use petgraph::stable_graph::{EdgeIndex, NodeIndex, StableDiGraph};
use petgraph::visit::IntoEdgeReferences;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct CognitiveGraph {
    graph: StableDiGraph<NodeData, EdgeData>,
    node_index: DashMap<Uuid, NodeIndex>,
}

impl Default for CognitiveGraph {
    fn default() -> Self {
        Self::new()
    }
}

impl CognitiveGraph {
    pub fn new() -> Self {
        Self {
            graph: StableDiGraph::new(),
            node_index: DashMap::new(),
        }
    }

    pub fn add_node(&mut self, node: NodeData) -> Result<Uuid, GraphError> {
        if self.node_index.contains_key(&node.id) {
            return Err(GraphError::DuplicateNode(node.id));
        }

        let node_id = node.id;
        let index = self.graph.add_node(node);
        self.node_index.insert(node_id, index);
        Ok(node_id)
    }

    pub fn remove_node(&mut self, node_id: Uuid) -> Result<NodeData, GraphError> {
        let (_, index) = self
            .node_index
            .remove(&node_id)
            .ok_or(GraphError::NodeNotFound(node_id))?;

        self.graph
            .remove_node(index)
            .ok_or(GraphError::NodeNotFound(node_id))
    }

    pub fn get_node(&self, node_id: Uuid) -> Option<&NodeData> {
        let index = *self.node_index.get(&node_id)?;
        self.graph.node_weight(index)
    }

    pub fn get_node_mut(&mut self, node_id: Uuid) -> Option<&mut NodeData> {
        let index = *self.node_index.get(&node_id)?;
        self.graph.node_weight_mut(index)
    }

    pub fn nodes(&self) -> impl Iterator<Item = &NodeData> {
        self.graph.node_weights()
    }

    pub fn edges(&self) -> impl Iterator<Item = &EdgeData> {
        self.graph.edge_weights()
    }

    pub fn node_ids(&self) -> Vec<Uuid> {
        self.node_index.iter().map(|e| *e.key()).collect()
    }

    pub fn connect(
        &mut self,
        source_id: Uuid,
        target_id: Uuid,
        edge_type: EdgeType,
        weight: f32,
    ) -> Result<(), GraphError> {
        let source_index = *self
            .node_index
            .get(&source_id)
            .ok_or(GraphError::NodeNotFound(source_id))?;
        let target_index = *self
            .node_index
            .get(&target_id)
            .ok_or(GraphError::NodeNotFound(target_id))?;

        if self.graph.find_edge(source_index, target_index).is_some() {
            return Err(GraphError::DuplicateEdge {
                from_id: source_id,
                to_id: target_id,
            });
        }

        let edge = EdgeData::new(source_id, target_id, edge_type, weight);
        self.graph.add_edge(source_index, target_index, edge);
        Ok(())
    }

    pub fn disconnect(&mut self, source_id: Uuid, target_id: Uuid) -> Result<(), GraphError> {
        let source_index = *self
            .node_index
            .get(&source_id)
            .ok_or(GraphError::NodeNotFound(source_id))?;
        let target_index = *self
            .node_index
            .get(&target_id)
            .ok_or(GraphError::NodeNotFound(target_id))?;

        let edge_index: EdgeIndex =
            self.graph
                .find_edge(source_index, target_index)
                .ok_or(GraphError::DuplicateEdge {
                    from_id: source_id,
                    to_id: target_id,
                })?;
        self.graph.remove_edge(edge_index);
        Ok(())
    }

    pub fn neighbors(&self, node_id: Uuid) -> Result<Vec<&NodeData>, GraphError> {
        let index = *self
            .node_index
            .get(&node_id)
            .ok_or(GraphError::NodeNotFound(node_id))?;

        let neighbors = self
            .graph
            .neighbors(index)
            .filter_map(|neighbor_index| self.graph.node_weight(neighbor_index))
            .collect();

        Ok(neighbors)
    }

    pub fn outgoing_edges(&self, node_id: Uuid) -> Result<Vec<&EdgeData>, GraphError> {
        let index = *self
            .node_index
            .get(&node_id)
            .ok_or(GraphError::NodeNotFound(node_id))?;

        Ok(self.graph.edges(index).map(|edge| edge.weight()).collect())
    }

    pub fn incoming_edges(&self, node_id: Uuid) -> Result<Vec<&EdgeData>, GraphError> {
        let index = *self
            .node_index
            .get(&node_id)
            .ok_or(GraphError::NodeNotFound(node_id))?;

        Ok(self
            .graph
            .edges_directed(index, petgraph::Direction::Incoming)
            .map(|edge| edge.weight())
            .collect())
    }

    pub fn get_edge(&self, source_id: Uuid, target_id: Uuid) -> Option<&EdgeData> {
        let source_index = *self.node_index.get(&source_id)?;
        let target_index = *self.node_index.get(&target_id)?;
        let edge_index = self.graph.find_edge(source_index, target_index)?;
        self.graph.edge_weight(edge_index)
    }

    pub fn touch_node(&mut self, node_id: Uuid, at: DateTime<Utc>) -> Result<(), GraphError> {
        let node = self
            .get_node_mut(node_id)
            .ok_or(GraphError::NodeNotFound(node_id))?;
        node.touch(at);
        Ok(())
    }

    pub fn edge_count(&self) -> usize {
        self.graph.edge_count()
    }

    pub fn node_count(&self) -> usize {
        self.graph.node_count()
    }

    pub fn degree(&self, node_id: Uuid) -> usize {
        self.node_index
            .get(&node_id)
            .map(|index| self.graph.edges(*index).count())
            .unwrap_or(0)
    }

    pub fn search_nodes(&self, query: &str) -> Vec<&NodeData> {
        let needle = query.to_lowercase();
        self.nodes()
            .filter(|node| {
                node.content.to_lowercase().contains(&needle)
                    || format!("{:?}", node.node_type)
                        .to_lowercase()
                        .contains(&needle)
            })
            .collect()
    }

    pub fn stats(&self) -> GraphStats {
        let mut ghost_count = 0;
        let mut fossil_count = 0;
        let mut void_count = 0;

        for node in self.nodes() {
            if node.is_ghost {
                ghost_count += 1;
            }
            if node.is_fossil {
                fossil_count += 1;
            }
            if node.is_void {
                void_count += 1;
            }
        }

        GraphStats {
            node_count: self.node_count(),
            edge_count: self.edge_count(),
            ghost_count,
            fossil_count,
            void_count,
        }
    }

    pub fn export(&self) -> StoredGraphView {
        let nodes = self.nodes().cloned().collect();
        let edges = self
            .graph
            .edge_references()
            .map(|edge| edge.weight().clone())
            .collect();

        StoredGraphView { nodes, edges }
    }

    pub fn from_parts(
        nodes: impl IntoIterator<Item = NodeData>,
        edges: impl IntoIterator<Item = EdgeData>,
    ) -> Result<Self, GraphError> {
        let mut graph = Self::new();

        for node in nodes {
            graph.add_node(node)?;
        }

        for edge in edges {
            graph.connect(edge.source_id, edge.target_id, edge.edge_type, edge.weight)?;
        }

        Ok(graph)
    }
}

#[derive(Debug, Clone)]
pub struct StoredGraphView {
    pub nodes: Vec<NodeData>,
    pub edges: Vec<EdgeData>,
}
