use crate::workspace::SilentNodeWorkspace;

/// Export workspace graph to GraphViz DOT format.
pub fn export_dot(workspace: &SilentNodeWorkspace) -> String {
    let mut out = String::from("digraph silentnode {\n");
    out.push_str("    graph [rankdir=LR, bgcolor=\"#080c18\"];\n");
    out.push_str("    node  [shape=ellipse, style=filled, fontname=\"monospace\", fontsize=10];\n");
    out.push_str("    edge  [fontsize=9];\n\n");

    for node in workspace.graph.nodes() {
        let color = if node.is_ghost {
            "#334466"
        } else if node.is_fossil {
            "#665533"
        } else if node.is_void {
            "#220022"
        } else {
            match node.node_type {
                crate::domain::NodeType::Idea => "#1e3a5f",
                crate::domain::NodeType::Memory => "#3a2a5f",
                crate::domain::NodeType::Project => "#1a4a2a",
                crate::domain::NodeType::Person => "#4a2a1a",
                crate::domain::NodeType::Artifact => "#3a3a1a",
                crate::domain::NodeType::Media => "#1a3a4a",
                crate::domain::NodeType::Process => "#2a1a4a",
                crate::domain::NodeType::World => "#1a4a4a",
                crate::domain::NodeType::Ghost => "#334466",
                crate::domain::NodeType::Fossil => "#665533",
                crate::domain::NodeType::Other => "#334455",
            }
        };

        let label = dot_escape(&truncate(&node.content, 30));
        let id = node.id.simple().to_string();
        out.push_str(&format!(
            "    n{id} [label=\"{label}\", fillcolor=\"{color}\", fontcolor=\"#c8d8ff\", \
             tooltip=\"entropy={:.2} gravity={:.2} access={}\"];\n",
            node.entropy, node.gravity, node.access_count,
        ));
    }

    out.push('\n');

    for edge in workspace.graph.edges() {
        let src = edge.source_id.simple().to_string();
        let dst = edge.target_id.simple().to_string();
        let (color, style) = match edge.edge_type {
            crate::domain::EdgeType::Connection => ("#4477cc", "solid"),
            crate::domain::EdgeType::Resonance => ("#cc7744", "dashed"),
            crate::domain::EdgeType::Temporal => ("#44cc77", "dotted"),
            crate::domain::EdgeType::Causal => ("#cc4477", "bold"),
        };
        out.push_str(&format!(
            "    n{src} -> n{dst} [weight={:.2}, color=\"{color}\", style={style}, label=\"{:.2}\"];\n",
            edge.weight, edge.weight,
        ));
    }

    out.push_str("}\n");
    out
}

/// Export nodes as CSV: id,type,content,entropy,gravity,access_count,x,y,z,ghost,fossil,void
pub fn export_csv(workspace: &SilentNodeWorkspace) -> String {
    let mut out = String::from(
        "id,type,content,entropy,gravity,access_count,x,y,z,ghost,fossil,void,created_at\n",
    );
    for node in workspace.graph.nodes() {
        out.push_str(&format!(
            "{},{:?},{},{:.4},{:.4},{},{:.3},{:.3},{:.3},{},{},{},{}\n",
            node.id,
            node.node_type,
            csv_escape(&node.content),
            node.entropy,
            node.gravity,
            node.access_count,
            node.position.x,
            node.position.y,
            node.position.z,
            node.is_ghost as u8,
            node.is_fossil as u8,
            node.is_void as u8,
            node.created_at.to_rfc3339(),
        ));
    }
    out
}

/// Export edges as CSV: source_id,target_id,edge_type,weight,created_at
pub fn export_edges_csv(workspace: &SilentNodeWorkspace) -> String {
    let mut out = String::from("source_id,target_id,edge_type,weight,created_at\n");
    for edge in workspace.graph.edges() {
        out.push_str(&format!(
            "{},{},{:?},{:.4},{}\n",
            edge.source_id,
            edge.target_id,
            edge.edge_type,
            edge.weight,
            edge.created_at.to_rfc3339(),
        ));
    }
    out
}

/// Export workspace as a Markdown document: hierarchy by gravity, journal entries, stats.
pub fn export_markdown(workspace: &SilentNodeWorkspace) -> String {
    let stats = workspace.graph.stats();
    let mut out = String::new();

    out.push_str("# SilentNode Workspace Export\n\n");
    out.push_str(&format!(
        "**Nodes:** {}  **Edges:** {}  **Ghosts:** {}  **Fossils:** {}  **Void:** {}\n\n",
        stats.node_count, stats.edge_count, stats.ghost_count, stats.fossil_count, stats.void_count
    ));

    // Nodes sorted by gravity descending
    out.push_str("## Nodes\n\n");
    let mut nodes: Vec<&crate::domain::NodeData> = workspace.graph.nodes().collect();
    nodes.sort_by(|a, b| {
        b.gravity
            .partial_cmp(&a.gravity)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    for node in &nodes {
        let flags = {
            let mut f = Vec::new();
            if node.is_ghost {
                f.push("ghost");
            }
            if node.is_fossil {
                f.push("fossil");
            }
            if node.is_void {
                f.push("void");
            }
            if f.is_empty() {
                String::new()
            } else {
                format!(" *({})*", f.join(", "))
            }
        };
        out.push_str(&format!("### {}{}\n\n", md_escape(&node.content), flags));
        out.push_str(&format!("- **ID:** `{}`\n", node.id));
        out.push_str(&format!("- **Type:** {:?}\n", node.node_type));
        out.push_str(&format!(
            "- **Entropy:** {:.3}  **Gravity:** {:.3}  **Velocity:** {:.3}\n",
            node.entropy, node.gravity, node.velocity
        ));
        out.push_str(&format!("- **Accesses:** {}\n", node.access_count));
        out.push_str(&format!(
            "- **Position:** ({:.2}, {:.2}, {:.2})\n",
            node.position.x, node.position.y, node.position.z
        ));
        out.push_str(&format!(
            "- **Created:** {}\n",
            node.created_at.to_rfc3339()
        ));
        out.push_str(&format!(
            "- **Last access:** {}\n",
            node.accessed_at.to_rfc3339()
        ));

        // Outgoing connections
        if let Ok(outgoing) = workspace.graph.outgoing_edges(node.id) {
            if !outgoing.is_empty() {
                out.push_str("- **Connections:**\n");
                for edge in outgoing {
                    let dst_label = workspace
                        .graph
                        .get_node(edge.target_id)
                        .map(|n| n.content.as_str())
                        .unwrap_or("?");
                    out.push_str(&format!(
                        "  - → {} `{:?}` w={:.2}\n",
                        md_escape(dst_label),
                        edge.edge_type,
                        edge.weight
                    ));
                }
            }
        }

        out.push('\n');
    }

    // Journal entries
    let entries = workspace.journal.entries();
    if !entries.is_empty() {
        out.push_str("## Journal\n\n");
        for entry in entries {
            out.push_str(&format!(
                "### {} — {}\n\n",
                entry.timestamp.format("%Y-%m-%d %H:%M"),
                entry.season.as_deref().unwrap_or("no season")
            ));
            out.push_str(&format!("{}\n\n", entry.content));
            if !entry.linked_nodes.is_empty() {
                let names: Vec<String> = entry
                    .linked_nodes
                    .iter()
                    .filter_map(|id| workspace.graph.get_node(*id))
                    .map(|n| format!("`{}`", n.content))
                    .collect();
                out.push_str(&format!("*Linked nodes: {}*\n\n", names.join(", ")));
            }
        }
    }

    out
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}…", &s[..max])
    }
}

fn dot_escape(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
}

fn csv_escape(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

fn md_escape(s: &str) -> String {
    s.replace('*', "\\*")
        .replace('_', "\\_")
        .replace('[', "\\[")
        .replace(']', "\\]")
        .replace('`', "\\`")
        .replace('#', "\\#")
}
