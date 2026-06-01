#[cfg(feature = "python")]
use pyo3::prelude::*;

use chrono::{DateTime, NaiveDate, Utc};
use silentnode_core::{
    export_csv,
    export_dot,
    export_edges_csv,
    export_html_dashboard,
    export_markdown,
    launch,
    migrate_sqlite_to_surreal_path,
    pull,
    push,
    run_tui,
    // Phase 6
    season_aura_colors,
    serve,
    start_api_server,
    AnalyticsEngine,
    ChangeType,
    DreamEngine,
    EdgeType,
    EntropyEngine,
    FocusDepth,
    GravityEngine,
    MaterializationEngine,
    NodeData,
    NodeType,
    Position3,
    RenderConfig,
    SilentNodeWorkspace,
    SqliteWorkspaceStore,
    SuggestionEngine,
    SurrealWorkspaceStore,
    SynthesisEngine,
    TectonicDetector,
    Vault,
    VisualizationEngine,
    WeatherSystem,
    WorkspaceSnapshot,
    WorkspaceStore,
};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use tokio::runtime::Runtime;
use uuid::Uuid;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = Paths::new();
    fs::create_dir_all(&config.data_dir)?;

    let args = env::args().skip(1).collect::<Vec<_>>();
    let command = Command::parse(&args)?;

    match command {
        Command::Help => print_help(),
        Command::Status => {
            let workspace = load_workspace(&config)?;
            print_status(&workspace, &config);
        }
        Command::ListNodes { query } => {
            let workspace = load_workspace(&config)?;
            print_nodes(&workspace, query.as_deref());
        }
        Command::ShowNode { node_id } => {
            let workspace = load_workspace(&config)?;
            print_node_details(&workspace, &node_id)?;
        }
        Command::Seed => {
            let mut workspace = load_workspace(&config)?;
            seed_workspace(&mut workspace)?;
            persist_and_render(&workspace, &config)?;
            println!("Seeded workspace and saved snapshot.");
        }
        Command::MigrateToSurreal => {
            migrate_default_sqlite_to_surreal(&config)?;
            println!(
                "Migrated SQLite snapshot {} into local Surreal store {}.",
                config.sqlite_path.display(),
                config.surreal_path.display()
            );
        }
        Command::SurrealHealth => {
            let counts = surreal_health(&config)?;
            println!("Surreal health");
            println!("Path: {}", config.surreal_path.display());
            println!("Nodes: {}", counts.nodes);
            println!("Edges: {}", counts.edges);
            println!("Focus events: {}", counts.focus_events);
            println!("Journal entries: {}", counts.journal_entries);
        }
        Command::Render => {
            let workspace = load_workspace(&config)?;
            workspace.render_html(&VisualizationEngine::new(), &config.html_path)?;
            println!("Rendered HTML universe to {}", config.html_path.display());
        }
        Command::Recompute => {
            let mut workspace = load_workspace(&config)?;
            recompute_workspace(&mut workspace);
            persist_and_render(&workspace, &config)?;
            println!("Recomputed entropy/gravity and persisted workspace.");
        }
        Command::Simulate { steps, delta_time } => {
            let mut workspace = load_workspace(&config)?;
            for _ in 0..steps {
                workspace.step_gravity(&GravityEngine::new(), delta_time);
            }
            workspace.tick_entropy(&EntropyEngine::new());
            persist_and_render(&workspace, &config)?;
            println!("Simulated {steps} gravity steps and rendered workspace.");
        }
        Command::AddThought { text } => {
            let mut workspace = load_workspace(&config)?;
            let result = workspace.materialize_thought(&MaterializationEngine::new(), &text)?;
            recompute_workspace(&mut workspace);
            persist_and_render(&workspace, &config)?;
            println!(
                "Added thought as {:?}: {}",
                result.node_type, result.node_id
            );
            if !result.suggestions.is_empty() {
                println!("Suggestions:");
                for suggestion in result.suggestions.iter().take(5) {
                    println!(
                        "  {}  score={:.2}  {}",
                        suggestion.node_id, suggestion.score, suggestion.content_preview
                    );
                }
            }
        }
        Command::Connect {
            source_id,
            target_id,
            weight,
            edge_type,
        } => {
            let mut workspace = load_workspace(&config)?;
            let source_id = Uuid::parse_str(&source_id)?;
            let target_id = Uuid::parse_str(&target_id)?;
            workspace.connect_nodes(source_id, target_id, parse_edge_type(&edge_type)?, weight)?;
            recompute_workspace(&mut workspace);
            persist_and_render(&workspace, &config)?;
            println!("Connected {source_id} -> {target_id} as {edge_type} ({weight:.2}).");
        }
        Command::Disconnect {
            source_id,
            target_id,
        } => {
            let mut workspace = load_workspace(&config)?;
            let source_id = Uuid::parse_str(&source_id)?;
            let target_id = Uuid::parse_str(&target_id)?;
            workspace.disconnect_nodes(source_id, target_id)?;
            recompute_workspace(&mut workspace);
            persist_and_render(&workspace, &config)?;
            println!("Disconnected {source_id} -> {target_id}.");
        }
        Command::DeleteNode { node_id } => {
            let mut workspace = load_workspace(&config)?;
            let node_id = Uuid::parse_str(&node_id)?;
            let removed = workspace.remove_node(node_id)?;
            persist_and_render(&workspace, &config)?;
            println!("Removed node {}: {}", removed.id, removed.content);
        }
        Command::ReviveNode { node_id } => {
            let mut workspace = load_workspace(&config)?;
            let node_id = Uuid::parse_str(&node_id)?;
            workspace.revive_node(node_id)?;
            recompute_workspace(&mut workspace);
            persist_and_render(&workspace, &config)?;
            println!("Revived node {}", node_id);
        }
        Command::Journal { text, season } => {
            let mut workspace = load_workspace(&config)?;
            let entry = workspace.add_journal_entry(text, season);
            persist_and_render(&workspace, &config)?;
            println!(
                "Journal entry saved: {}  linked_nodes={}",
                entry.id,
                entry.linked_nodes.len()
            );
        }
        Command::JournalSearch { query } => {
            let workspace = load_workspace(&config)?;
            let entries = workspace.search_journal(&query);
            if entries.is_empty() {
                println!("No journal entries matched.");
            } else {
                for entry in entries {
                    println!("{}  {}", entry.timestamp.to_rfc3339(), entry.id);
                    print_linked_nodes(&workspace, &entry.linked_nodes);
                    println!("  {}", entry.content);
                }
            }
        }
        Command::JournalList { days } => {
            let workspace = load_workspace(&config)?;
            let entries = workspace.recent_journal(days);
            if entries.is_empty() {
                println!("No journal entries in the last {days} day(s).");
            } else {
                for entry in entries {
                    println!(
                        "{}  {}  season={}",
                        entry.timestamp.to_rfc3339(),
                        entry.id,
                        entry.season.as_deref().unwrap_or("none")
                    );
                    print_linked_nodes(&workspace, &entry.linked_nodes);
                    println!("  {}", entry.content);
                }
            }
        }
        Command::JournalRange { from, to } => {
            let workspace = load_workspace(&config)?;
            let entries = workspace.journal_between(from, to);
            if entries.is_empty() {
                println!(
                    "No journal entries between {} and {}.",
                    from.to_rfc3339(),
                    to.to_rfc3339()
                );
            } else {
                for entry in entries {
                    println!(
                        "{}  {}  season={}",
                        entry.timestamp.to_rfc3339(),
                        entry.id,
                        entry.season.as_deref().unwrap_or("none")
                    );
                    print_linked_nodes(&workspace, &entry.linked_nodes);
                    println!("  {}", entry.content);
                }
            }
        }
        Command::Focus {
            node_id,
            seconds,
            depth,
        } => {
            let mut workspace = load_workspace(&config)?;
            let node_id = Uuid::parse_str(&node_id)?;
            let depth = parse_focus_depth(&depth)?;
            workspace.record_focus(node_id, seconds, depth)?;
            recompute_workspace(&mut workspace);
            persist_and_render(&workspace, &config)?;
            println!("Focus event recorded for {}", node_id);
        }
        Command::ExportEncrypted { password, output } => {
            let workspace = load_workspace(&config)?;
            export_encrypted_snapshot(&workspace.snapshot(), &password, &output)?;
            println!("Encrypted snapshot written to {}", output.display());
        }
        Command::ImportEncrypted { password, input } => {
            let snapshot = import_encrypted_snapshot(&password, &input)?;
            let workspace = SilentNodeWorkspace::from_snapshot(snapshot)?;
            persist_and_render(&workspace, &config)?;
            println!("Imported encrypted snapshot from {}", input.display());
        }
        Command::RotateVaultPassword {
            current_password,
            new_password,
        } => {
            rotate_vault_password(&config, &current_password, &new_password)?;
            println!("Vault password rotated for {}", config.vault_dir.display());
        }
        Command::Trail { hours } => {
            let workspace = load_workspace(&config)?;
            let trail = workspace.recent_trail(hours);
            if trail.is_empty() {
                println!("No focus events in the last {hours} hour(s).");
            } else {
                for event in trail {
                    let label = workspace
                        .get_node(event.node_id)
                        .map(|node| node.content.as_str())
                        .unwrap_or("<missing node>");
                    println!(
                        "{}  {}  {}s  {:?}",
                        event.timestamp.to_rfc3339(),
                        label,
                        event.duration_seconds,
                        event.depth
                    );
                }
            }
        }
        Command::NodeTrail { node_id, hours } => {
            let workspace = load_workspace(&config)?;
            let node_id = Uuid::parse_str(&node_id)?;
            let events = workspace.focus_history_for_node(node_id, hours);
            if events.is_empty() {
                println!("No focus history for {node_id} in the last {hours} hour(s).");
            } else {
                for event in events {
                    println!(
                        "{}  {}s  {:?}",
                        event.timestamp.to_rfc3339(),
                        event.duration_seconds,
                        event.depth
                    );
                }
            }
        }
        Command::NodeJournal { node_id, days } => {
            let workspace = load_workspace(&config)?;
            let node_id = Uuid::parse_str(&node_id)?;
            let entries = workspace.journal_for_node(node_id, days);
            if entries.is_empty() {
                println!("No journal entries linked to {node_id} in the last {days} day(s).");
            } else {
                for entry in entries {
                    println!(
                        "{}  {}  season={}",
                        entry.timestamp.to_rfc3339(),
                        entry.id,
                        entry.season.as_deref().unwrap_or("none")
                    );
                    println!("  {}", entry.content);
                }
            }
        }
        Command::Heatmap { days } => {
            let workspace = load_workspace(&config)?;
            let report = workspace.heatmap_report(days);
            if report.is_empty() {
                println!("No heatmap data in the last {days} day(s).");
            } else {
                for (node_id, score) in report {
                    let label = workspace
                        .get_node(node_id)
                        .map(|node| node.content.as_str())
                        .unwrap_or("<missing node>");
                    println!("{score:.3}  {node_id}  {label}");
                }
            }
        }
        Command::Launch { width, height } => {
            let workspace = load_workspace(&config)?;
            let render_config = RenderConfig {
                width,
                height,
                autosave_path: Some(config.sqlite_path.clone()),
                ..RenderConfig::default()
            };
            println!(
                "Launching GPU renderer ({}×{}) — close window to exit.",
                width, height
            );
            launch(workspace, render_config);
        }
        Command::Weather => {
            let workspace = load_workspace(&config)?;
            let mut weather = WeatherSystem::new();
            workspace.derive_weather(&mut weather);
            println!("Weather state: {}", weather.current.name());
            println!("  intensity:   {:.3}", weather.current.intensity());
            println!("  turbulence:  {:.3}", weather.current.turbulence());
            println!("  pulse rate:  {:.3}", weather.current.pulse_rate());
            println!("  particle speed: {:.3}", weather.current.particle_speed());
            let p = weather.current.primary_color();
            let s = weather.current.secondary_color();
            println!(
                "  primary:     rgba({:.2}, {:.2}, {:.2}, {:.2})",
                p[0], p[1], p[2], p[3]
            );
            println!(
                "  secondary:   rgba({:.2}, {:.2}, {:.2}, {:.2})",
                s[0], s[1], s[2], s[3]
            );
        }
        Command::Weight => {
            let workspace = load_workspace(&config)?;
            let report = workspace.cognitive_weight();
            println!("{}", report.summary());
            println!("  visual density:  {:.3}", report.visual_density());
            println!("  particle drag:   {:.3}", report.particle_drag());
            println!("  heavy:           {}", report.is_heavy());
        }
        Command::Souls => {
            let workspace = load_workspace(&config)?;
            let souls = workspace.derive_souls();
            if souls.is_empty() {
                println!("No Project/World nodes found.");
            } else {
                for soul in souls {
                    let label = workspace
                        .get_node(soul.project_id)
                        .map(|n| n.content.as_str())
                        .unwrap_or("<unknown>");
                    println!(
                        "{}  {}  activity={:.2} maturity={:.2} social={:.2}  {:?} {:?}",
                        soul.project_id,
                        label,
                        soul.activity_level,
                        soul.maturity,
                        soul.social_density,
                        soul.particle_style,
                        soul.glow_pattern,
                    );
                }
            }
        }
        Command::Tectonics => {
            let workspace = load_workspace(&config)?;
            let snapshot = workspace.tectonic_snapshot();
            println!("Graph snapshot taken at {}", snapshot.taken_at.to_rfc3339());
            println!(
                "  nodes: {}  edges: {}",
                snapshot.nodes.len(),
                snapshot.edges.len()
            );
            let detector = TectonicDetector::new();
            match workspace.check_tectonics(&snapshot, &detector) {
                Some(event) => println!("Tectonic event: {}", event.description),
                None => println!(
                    "No tectonic shift detected (baseline snapshot matches current state)."
                ),
            }
        }

        // ── Phase 6: Pattern Recognition ──────────────────────────────────────
        Command::Season => {
            let workspace = load_workspace(&config)?;
            let report = workspace.cognitive_season();
            println!("Cognitive Season: {:?}", report.season);
            println!("  creation rate:   {:.3}", report.creation_rate);
            println!("  focus density:   {:.3}", report.focus_density);
            println!("  exploration:     {:.3}", report.exploration_ratio);
            println!("  revisit ratio:   {:.3}", report.revisit_ratio);
            println!("  avg entropy:     {:.3}", report.avg_entropy);
            let (p, s) = season_aura_colors(report.season);
            println!(
                "  aura primary:    rgba({:.2},{:.2},{:.2},{:.2})",
                p[0], p[1], p[2], p[3]
            );
            println!(
                "  aura secondary:  rgba({:.2},{:.2},{:.2},{:.2})",
                s[0], s[1], s[2], s[3]
            );
        }
        Command::Oracle => {
            let workspace = load_workspace(&config)?;
            let signals = workspace.oracle_signals();
            if signals.is_empty() {
                println!("No oracle signals detected.");
            } else {
                println!("Oracle Signals ({}):", signals.len());
                for sig in &signals {
                    println!("  [strength={:.3}] {}", sig.strength, sig.description);
                }
            }
        }
        Command::Rituals => {
            let workspace = load_workspace(&config)?;
            let rituals = workspace.detect_rituals();
            if rituals.is_empty() {
                println!("No behavioral rituals detected yet (need more focus history).");
            } else {
                println!("Detected Rituals ({}):", rituals.len());
                for r in &rituals {
                    println!(
                        "  {} — {} occurrences  strength={:.2}",
                        r.name, r.occurrence_count, r.strength
                    );
                    let steps: Vec<String> = r
                        .sequence
                        .iter()
                        .map(|id| {
                            workspace
                                .get_node(*id)
                                .map(|n| n.content.as_str())
                                .unwrap_or("?")
                                .to_string()
                        })
                        .collect();
                    println!("    sequence: {}", steps.join(" → "));
                }
            }
        }
        Command::Mirror { days } => {
            let workspace = load_workspace(&config)?;
            let portrait = workspace.cognitive_mirror(days);
            println!("Cognitive Mirror (last {days} days):");
            if portrait.priority_gaps.is_empty() {
                println!("  Priority gaps: none detected");
            } else {
                println!("  Priority gaps:");
                for pg in portrait.priority_gaps.iter().take(5) {
                    let label = workspace
                        .get_node(pg.node_id)
                        .map(|n| n.content.as_str())
                        .unwrap_or("?");
                    let dir = if pg.gap > 0 {
                        "neglected"
                    } else {
                        "over-focused"
                    };
                    println!("    {} — {} (gap={:+})", label, dir, pg.gap);
                }
            }
            if let Some(n) = portrait.most_neglected {
                let label = workspace
                    .get_node(n)
                    .map(|nd| nd.content.as_str())
                    .unwrap_or("?");
                println!("  Most neglected: {} — {}", n, label);
            }
            if let Some(n) = portrait.most_obsessed {
                let label = workspace
                    .get_node(n)
                    .map(|nd| nd.content.as_str())
                    .unwrap_or("?");
                println!("  Most obsessed:  {} — {}", n, label);
            }
            println!("  Blind spots: {}", portrait.blind_spots.len());
            println!("  Obsessions:  {}", portrait.obsessions.len());
            // ── Evolution Portrait ────────────────────────────────────────
            if !portrait.evolution.is_empty() {
                println!("  Evolution ({} tracked nodes):", portrait.evolution.len());
                for e in portrait.evolution.iter().take(5) {
                    let arrow = match e.trajectory.as_str() {
                        "rising" => "↑",
                        "decaying" => "↓",
                        _ => "→",
                    };
                    let central = if e.was_central { " [was central]" } else { "" };
                    println!(
                        "    {} {} entropy {:.2}→{:.2}{}",
                        arrow, e.label, e.entropy_start, e.entropy_now, central
                    );
                    if let Some((from, to)) = &e.type_evolution {
                        println!("       type: {} → {}", from, to);
                    }
                }
            }
            // ── Creative Pattern ──────────────────────────────────────────
            if let Some(cp) = &portrait.creative_pattern {
                println!(
                    "  Creative pattern: {} (peak hour={:?}, day={:?}, avg session={:.0}s)",
                    cp.focus_period, cp.peak_hour, cp.peak_weekday, cp.avg_deep_session_secs
                );
            }
        }
        Command::Heatmap7 { days } => {
            let workspace = load_workspace(&config)?;
            let heatmap = workspace.thought_heatmap(days);
            println!("Thought Heatmap (last {days} days):");
            for entry in heatmap.entries.iter().take(10) {
                let label = workspace
                    .get_node(entry.node_id)
                    .map(|n| n.content.as_str())
                    .unwrap_or("?");
                println!("  {:.3}  {}  {}", entry.energy, entry.node_id, label);
            }
            let loops = workspace.obsessive_loops();
            if !loops.is_empty() {
                println!("Obsessive loops ({}):", loops.len());
                for l in &loops {
                    let label = workspace
                        .get_node(l.node_id)
                        .map(|n| n.content.as_str())
                        .unwrap_or("?");
                    println!("  {} — entropy={:.2}", label, l.entropy);
                }
            }
        }
        Command::Contracts => {
            let workspace = load_workspace(&config)?;
            let contracts = workspace.detect_contracts();
            if contracts.is_empty() {
                println!("No silent contracts detected.");
            } else {
                println!("Silent Contracts ({}):", contracts.len());
                for c in &contracts {
                    println!("  {}", c.description);
                }
            }
        }
        Command::Resonances => {
            let workspace = load_workspace(&config)?;
            let pairs = workspace.resonant_pairs();
            if pairs.is_empty() {
                println!("No resonant pairs found.");
            } else {
                println!("Resonant Pairs ({}):", pairs.len().min(10));
                for p in pairs.iter().take(10) {
                    let la = workspace
                        .get_node(p.node_a)
                        .map(|n| n.content.as_str())
                        .unwrap_or("?");
                    let lb = workspace
                        .get_node(p.node_b)
                        .map(|n| n.content.as_str())
                        .unwrap_or("?");
                    println!("  {:.3}  \"{}\"  ↔  \"{}\"", p.similarity, la, lb);
                }
            }
        }

        // ── Phase 7: Social Graph ──────────────────────────────────────────────
        Command::Civilizations => {
            let workspace = load_workspace(&config)?;
            let civs = workspace.detect_civilizations();
            if civs.is_empty() {
                println!("No civilizations detected yet (need more connected nodes).");
            } else {
                println!("Thought Civilizations ({}):", civs.len());
                for (i, civ) in civs.iter().enumerate() {
                    let dominant = civ
                        .dominant_node
                        .and_then(|id| workspace.get_node(id))
                        .map(|n| n.content.as_str())
                        .unwrap_or("?");
                    println!(
                        "  [{}] {} members  density={:.3}  age={:.1}d  dominant={}",
                        i + 1,
                        civ.member_nodes.len(),
                        civ.internal_density,
                        civ.age_days,
                        dominant
                    );
                }
            }
        }
        Command::Crystallize => {
            let workspace = load_workspace(&config)?;
            let civs = workspace.detect_civilizations();
            let mut crystalized = 0usize;
            for civ in &civs {
                let check = workspace.check_crystallization(civ);
                if check.qualifies {
                    let crystal = workspace.crystallize_civilization(civ);
                    println!(
                        "Crystal formed: {}  nodes={}  density={:.3}",
                        crystal.id,
                        crystal.member_nodes.len(),
                        crystal.internal_density
                    );
                    crystalized += 1;
                }
            }
            if crystalized == 0 {
                println!("No civilizations qualify for crystallization yet.");
                println!("Requirements: density>0.65, size≥4, age≥14 days.");
            }
        }
        Command::Shadows => {
            let workspace = load_workspace(&config)?;
            let shadows = workspace.digital_shadows();
            if shadows.is_empty() {
                println!("No digital shadows detected.");
            } else {
                println!("Digital Shadows ({}):", shadows.len());
                for s in &shadows {
                    println!("  {}", s.description);
                }
            }
        }
        Command::ShatterCrystal { node_id } => {
            let mut workspace = load_workspace(&config)?;
            let node_id = Uuid::parse_str(&node_id)?;
            // Find the crystal containing this node
            let civs = workspace.detect_civilizations();
            let civ = civs.iter().find(|c| c.member_nodes.contains(&node_id));
            match civ {
                None => println!(
                    "No crystallized civilization found containing node {}.",
                    node_id
                ),
                Some(c) => {
                    let crystal = workspace.crystallize_civilization(c);
                    let released = workspace.shatter_crystal(&crystal);
                    persist_and_render(&workspace, &config)?;
                    println!(
                        "Crystal shattered — {} nodes released back to fluid state.",
                        released
                    );
                }
            }
        }
        Command::IlluminateShadow { node_id } => {
            let mut workspace = load_workspace(&config)?;
            let node_id = Uuid::parse_str(&node_id)?;
            workspace.illuminate_shadow(node_id)?;
            persist_and_render(&workspace, &config)?;
            println!(
                "Shadow illuminated — node {} brought into the active universe.",
                node_id
            );
        }
        Command::NameShadow { node_id, name } => {
            let mut workspace = load_workspace(&config)?;
            let node_id = Uuid::parse_str(&node_id)?;
            workspace.name_shadow(node_id, Some(name.clone()))?;
            persist_and_render(&workspace, &config)?;
            println!(
                "Shadow named: '{}' — acknowledged without commitment.",
                name
            );
        }
        Command::ReleaseShadow { node_id } => {
            let mut workspace = load_workspace(&config)?;
            let node_id = Uuid::parse_str(&node_id)?;
            workspace.release_shadow(node_id)?;
            persist_and_render(&workspace, &config)?;
            println!(
                "Shadow released — node {} consciously dissolved to the void.",
                node_id
            );
        }
        Command::FulfillContract { node_id } => {
            let mut workspace = load_workspace(&config)?;
            let node_id = Uuid::parse_str(&node_id)?;
            workspace.fulfill_contract(node_id)?;
            persist_and_render(&workspace, &config)?;
            println!(
                "Contract fulfilled — deep work commitment recorded for node {}.",
                node_id
            );
        }
        Command::ReleaseContract { node_id } => {
            let mut workspace = load_workspace(&config)?;
            let node_id = Uuid::parse_str(&node_id)?;
            workspace.release_contract(node_id)?;
            persist_and_render(&workspace, &config)?;
            println!("Contract released — conscious closure recorded in journal.");
        }
        Command::OpenChambers { threshold } => {
            let workspace = load_workspace(&config)?;
            let chambers = workspace.open_resonance_chambers(threshold);
            if chambers.is_empty() {
                println!("No resonance chambers opened (threshold={threshold:.2}).");
            } else {
                println!(
                    "Resonance Chambers ({}) [threshold={threshold:.2}]:",
                    chambers.len()
                );
                for ch in &chambers {
                    let la = workspace
                        .get_node(ch.node_a)
                        .map(|n| n.content.chars().take(30).collect::<String>())
                        .unwrap_or_default();
                    let lb = workspace
                        .get_node(ch.node_b)
                        .map(|n| n.content.chars().take(30).collect::<String>())
                        .unwrap_or_default();
                    println!(
                        "  {} ↔ {}  similarity={:.3}  cross-civ={}",
                        la, lb, ch.similarity, !ch.same_civilization
                    );
                }
                println!("  Use 'connect <a> <b>' to accept a chamber.");
            }
        }
        Command::VoidNode { node_id } => {
            let mut workspace = load_workspace(&config)?;
            let node_id = Uuid::parse_str(&node_id)?;
            workspace.send_to_void(node_id)?;
            persist_and_render(&workspace, &config)?;
            println!("Sent node {} to the void.", node_id);
        }
        Command::UnvoidNode { node_id } => {
            let mut workspace = load_workspace(&config)?;
            let node_id = Uuid::parse_str(&node_id)?;
            workspace.extract_from_void(node_id)?;
            persist_and_render(&workspace, &config)?;
            println!("Extracted node {} from the void.", node_id);
        }
        Command::VoidCheck { node_id } => {
            let workspace = load_workspace(&config)?;
            let node_id = Uuid::parse_str(&node_id)?;
            match workspace.check_void_emergence(node_id) {
                None => println!("Node {} is not in the void.", node_id),
                Some(check) => {
                    println!("Void emergence check for {}:", node_id);
                    println!("  resonance score: {:.3}", check.resonance_score);
                    println!("  emergence likely: {}", check.emergence_likely);
                    println!(
                        "  similar active nodes: {}",
                        check.similar_active_nodes.len()
                    );
                    for id in check.similar_active_nodes.iter().take(5) {
                        let label = workspace
                            .get_node(*id)
                            .map(|n| n.content.as_str())
                            .unwrap_or("?");
                        println!("    {} — {}", id, label);
                    }
                }
            }
        }

        // ── Phase 5: Temporal ─────────────────────────────────────────────────
        Command::TemporalStatus => {
            let workspace = load_workspace(&config)?;
            println!("Temporal Engine");
            println!("  snapshots:     {}", workspace.temporal_snapshot_count());
            println!(
                "  tracked nodes: {}",
                workspace.temporal.tracked_node_count()
            );
        }
        Command::SnapshotAll => {
            let mut workspace = load_workspace(&config)?;
            workspace.snapshot_all_nodes();
            persist_and_render(&workspace, &config)?;
            println!(
                "Snapshotted all {} nodes into temporal engine.",
                workspace.graph.node_count()
            );
        }
        Command::RecordChange { node_id } => {
            let mut workspace = load_workspace(&config)?;
            let node_id = Uuid::parse_str(&node_id)?;
            workspace.record_temporal_change(node_id, ChangeType::Accessed);
            persist_and_render(&workspace, &config)?;
            println!("Recorded temporal access event for {}", node_id);
        }
        Command::Archaeology { node_id } => {
            let workspace = load_workspace(&config)?;
            let node_id = Uuid::parse_str(&node_id)?;
            match workspace.open_archaeology(node_id) {
                None => println!("No temporal history for node {}", node_id),
                Some(session) => {
                    println!("Archaeology session for {}", node_id);
                    println!("  depth: {} snapshots", session.depth());
                    println!("  timeline:");
                    for (idx, ts, ct) in session.timeline() {
                        println!("    [{idx:>3}] {}  {:?}", ts.to_rfc3339(), ct);
                    }
                    let current = session.resurrect();
                    println!(
                        "  current state (latest): {:?}  entropy={:.3}  access={}",
                        current.node_type, current.entropy, current.access_count
                    );
                }
            }
        }
        Command::ReconstructDay { date } => {
            let workspace = load_workspace(&config)?;
            let rec = workspace.reconstruct_day(date);
            println!("{}", rec.summary());
            println!("  node states:   {}", rec.node_states.len());
            println!("  focus events:  {}", rec.focus_events.len());
            println!("  journal:       {}", rec.journal_entries.len());
            println!(
                "  aura:          {} (intensity={:.3} turbulence={:.3})",
                rec.aura_signature.state_name,
                rec.aura_signature.intensity,
                rec.aura_signature.turbulence
            );
            if let Some(dom) = rec.dominant_node {
                let label = workspace
                    .get_node(dom)
                    .map(|n| n.content.as_str())
                    .unwrap_or("<unknown>");
                println!("  dominant node: {} — {}", dom, label);
            }
        }
        Command::CompareDays { day_a, day_b } => {
            let workspace = load_workspace(&config)?;
            let cmp = workspace.compare_days(day_a, day_b);
            println!("{}", cmp.summary());
            if !cmp.new_nodes.is_empty() {
                println!("  new nodes ({}):", cmp.new_nodes.len());
                for id in &cmp.new_nodes {
                    let label = workspace
                        .get_node(*id)
                        .map(|n| n.content.as_str())
                        .unwrap_or("?");
                    println!("    + {} — {}", id, label);
                }
            }
            if !cmp.removed_nodes.is_empty() {
                println!("  removed nodes ({}):", cmp.removed_nodes.len());
                for id in &cmp.removed_nodes {
                    println!("    - {}", id);
                }
            }
            if !cmp.changed_nodes.is_empty() {
                println!("  changed nodes ({}):", cmp.changed_nodes.len());
                for c in &cmp.changed_nodes {
                    let label = workspace
                        .get_node(c.node_id)
                        .map(|n| n.content.as_str())
                        .unwrap_or("?");
                    println!(
                        "    ~ {} — {}  Δentropy={:+.3}  Δaccess={:+}{}{}",
                        c.node_id,
                        label,
                        c.entropy_delta,
                        c.access_delta,
                        if c.became_ghost { " [→ghost]" } else { "" },
                        if c.became_fossil { " [→fossil]" } else { "" },
                    );
                }
            }
        }
        Command::FossilCheck { node_id } => {
            let workspace = load_workspace(&config)?;
            let node_id = Uuid::parse_str(&node_id)?;
            match workspace.check_fossilization(node_id) {
                None => println!("Node {} not found.", node_id),
                Some(check) => {
                    println!("Fossilization check for {}", node_id);
                    println!("  {}", check.summary());
                }
            }
        }
        Command::Fossilize { node_id } => {
            let mut workspace = load_workspace(&config)?;
            let node_id = Uuid::parse_str(&node_id)?;
            workspace.fossilize_node(node_id)?;
            persist_and_render(&workspace, &config)?;
            println!("Fossilized node {}.", node_id);
        }
        Command::Excavate { node_id } => {
            let mut workspace = load_workspace(&config)?;
            let node_id = Uuid::parse_str(&node_id)?;
            workspace.excavate_node(node_id)?;
            persist_and_render(&workspace, &config)?;
            println!("Excavated node {} (restored to ghost).", node_id);
        }
        Command::Lore => {
            let workspace = load_workspace(&config)?;
            let lore = workspace.detect_lore(&[]);
            if lore.is_empty() {
                println!("No lore arcs detected yet. Add more nodes and temporal snapshots.");
            } else {
                println!("Lore Arcs ({} detected):", lore.len());
                for entry in &lore {
                    println!(
                        "\n  [{:?}] significance={:.2}  {}",
                        entry.arc_type, entry.significance, entry.title
                    );
                    println!("  {}", entry.narrative);
                    if !entry.linked_nodes.is_empty() {
                        let names: Vec<&str> = entry
                            .linked_nodes
                            .iter()
                            .filter_map(|id| workspace.get_node(*id))
                            .map(|n| n.content.as_str())
                            .collect();
                        println!("  nodes: {}", names.join(", "));
                    }
                }
            }
        }

        // ── Sync commands ────────────────────────────────────────────────────
        // ── Phase 8: External World Integration ───────────────────────────────
        Command::MembraneStatus => {
            let workspace = load_workspace(&config)?;
            workspace.membrane.print_status();
        }
        Command::MembraneAllow { pattern } => {
            use silentnode_core::membrane::{CrossingDirection, MembraneRule};
            let mut workspace = load_workspace(&config)?;
            workspace.membrane.add_rule(
                MembraneRule::allow(pattern.clone(), CrossingDirection::Both)
                    .with_description(format!("user-added allow rule for {pattern}")),
            );
            persist_and_render(&workspace, &config)?;
            println!("Membrane: ALLOW rule added for '{}'.", pattern);
        }
        Command::MembraneBlock { pattern } => {
            use silentnode_core::membrane::{CrossingDirection, MembraneRule};
            let mut workspace = load_workspace(&config)?;
            workspace.membrane.add_rule(
                MembraneRule::block(pattern.clone(), CrossingDirection::Both)
                    .with_description(format!("user-added block rule for {pattern}")),
            );
            persist_and_render(&workspace, &config)?;
            println!("Membrane: BLOCK rule added for '{}'.", pattern);
        }
        Command::MembraneLog { last } => {
            let workspace = load_workspace(&config)?;
            let log = workspace.membrane.combined_log();
            let start = log.len().saturating_sub(last);
            println!("Membrane log (last {} events):", last.min(log.len()));
            for ev in &log[start..] {
                println!("  {}", ev.summary());
            }
        }
        Command::PortalStatus => {
            let workspace = load_workspace(&config)?;
            workspace.portals.print_status();
        }
        Command::PortalAddFilesystem { path } => {
            use silentnode_core::portals::FilesystemPortal;
            let mut workspace = load_workspace(&config)?;
            let portal = FilesystemPortal::new(&path);
            println!("Filesystem portal created for '{}'. Scanning...", path);
            let entries = portal.scan_entries();
            workspace.portals.add_filesystem_portal(portal);
            persist_and_render(&workspace, &config)?;
            println!(
                "  {} entries found (most recently modified first):",
                entries.len()
            );
            for (name, is_dir, age_secs) in entries.iter().take(10) {
                let age_h = age_secs / 3600.0;
                println!(
                    "  {} {:30} {:.1}h ago",
                    if *is_dir { "DIR " } else { "FILE" },
                    name,
                    age_h
                );
            }
        }
        Command::PortalAddExternal { name, kind, url } => {
            use silentnode_core::portals::{ExternalPortal, PortalType};
            let ptype = match kind.as_str() {
                "media" => PortalType::Media,
                "code" => PortalType::Code,
                "knowledge" => PortalType::Knowledge,
                "comms" | "communication" => PortalType::Communication,
                _ => PortalType::Knowledge,
            };
            let mut workspace = load_workspace(&config)?;
            let portal = ExternalPortal::new(name.clone(), ptype, url.clone());
            workspace.portals.add_external_portal(portal);
            persist_and_render(&workspace, &config)?;
            println!("External portal added: '{}' ({}) → {}", name, kind, url);
        }
        Command::ProcessStatus => {
            let workspace = load_workspace(&config)?;
            workspace.processes.print_status();
        }
        Command::ProcessLink { pid, node_id } => {
            let mut workspace = load_workspace(&config)?;
            let node_id = Uuid::parse_str(&node_id)?;
            workspace.processes.link_to_node(pid, node_id);
            persist_and_render(&workspace, &config)?;
            let label = workspace
                .get_node(node_id)
                .map(|n| n.content.as_str())
                .unwrap_or("?");
            println!(
                "Process PID {} linked to node '{}' ({}).",
                pid, label, node_id
            );
        }
        Command::CalendarStatus => {
            let workspace = load_workspace(&config)?;
            workspace.calendar.print_status(Utc::now());
        }
        Command::CalendarAdd {
            title,
            days_ahead,
            category,
        } => {
            use silentnode_core::calendar::{CalendarEvent, EventCategory};
            let cat = match category.as_str() {
                "meeting" => EventCategory::Meeting,
                "deadline" => EventCategory::Deadline,
                "review" => EventCategory::Review,
                "personal" => EventCategory::Personal,
                "recurring" => EventCategory::Recurring,
                "milestone" => EventCategory::Milestone,
                _ => EventCategory::Meeting,
            };
            let start = Utc::now() + chrono::Duration::days(days_ahead);
            let end = start + chrono::Duration::hours(1);
            let mut workspace = load_workspace(&config)?;
            let event = CalendarEvent::new(title.clone(), cat, start, end);
            let ev_id = event.id;
            workspace.calendar.add_event(event);
            persist_and_render(&workspace, &config)?;
            println!(
                "Calendar event added: '{}' in {}d (id={}).",
                title, days_ahead, ev_id
            );
        }
        Command::CalendarPrep { event_id } => {
            let workspace = load_workspace(&config)?;
            let ev_id = Uuid::parse_str(&event_id)?;
            match workspace
                .calendar
                .analyze_preparation(ev_id, workspace.focus.events(), 14)
            {
                None => println!("Event {} not found.", event_id),
                Some(analysis) => {
                    println!("Preparation analysis for event {}:", event_id);
                    println!("  Has prep activity:   {}", analysis.has_prep_activity);
                    println!(
                        "  Prep start (h before): {:.1}",
                        analysis.avg_prep_start_hours_before
                    );
                    println!(
                        "  Procrastination score: {:.2}",
                        analysis.procrastination_score
                    );
                    println!("  Prep nodes touched: {}", analysis.prep_nodes.len());
                }
            }
        }
        Command::SyncServe { port } => {
            let workspace = load_workspace(&config)?;
            let addr: std::net::SocketAddr = format!("0.0.0.0:{port}").parse()?;
            Runtime::new()?.block_on(serve(&workspace, addr))?;
        }
        Command::SyncPull { addr } => {
            let workspace = load_workspace(&config)?;
            let socket_addr: std::net::SocketAddr = addr.parse()?;
            let merged = Runtime::new()?.block_on(pull(&workspace, socket_addr))?;
            let nodes = merged.graph.nodes.len();
            println!("[sync] pull complete — {nodes} nodes in merged snapshot");
            let new_ws = SilentNodeWorkspace::from_snapshot(merged)?;
            persist_and_render(&new_ws, &config)?;
            println!("[sync] workspace saved");
        }
        Command::SyncPush { addr } => {
            let workspace = load_workspace(&config)?;
            let socket_addr: std::net::SocketAddr = addr.parse()?;
            let merged = Runtime::new()?.block_on(push(&workspace, socket_addr))?;
            let nodes = merged.graph.nodes.len();
            println!("[sync] push complete — {nodes} nodes in merged snapshot");
            let new_ws = SilentNodeWorkspace::from_snapshot(merged)?;
            persist_and_render(&new_ws, &config)?;
            println!("[sync] workspace saved");
        }

        // ── Phase 9: Intelligence + Export + API ──────────────────────────────
        Command::Suggestions { limit } => {
            let workspace = load_workspace(&config)?;
            let engine = SuggestionEngine::new();
            let suggestions = engine.suggest_next_focus(&workspace, limit);
            if suggestions.is_empty() {
                println!("No suggestions (workspace may be empty).");
            } else {
                println!("Next Focus Suggestions ({}):", suggestions.len());
                for s in &suggestions {
                    println!(
                        "  score={:.3}  {}  — {}",
                        s.score, s.content_preview, s.reason
                    );
                }
            }
        }
        Command::Related { node_id, limit } => {
            let workspace = load_workspace(&config)?;
            let node_id = Uuid::parse_str(&node_id)?;
            let engine = SuggestionEngine::new();
            let related = engine.suggest_related(&workspace, node_id, limit);
            if related.is_empty() {
                println!("No related nodes found.");
            } else {
                println!("Related nodes ({}):", related.len());
                for r in &related {
                    println!("  sim={:.3}  {}", r.similarity, r.content_preview);
                }
            }
        }
        Command::Clusters { k } => {
            let workspace = load_workspace(&config)?;
            let engine = SuggestionEngine::new();
            let clusters = engine.cluster_content(&workspace, k);
            if clusters.is_empty() {
                println!("No clusters found.");
            } else {
                println!("Content Clusters (k={k}):");
                for (i, c) in clusters.iter().enumerate() {
                    println!(
                        "  [{}] centroid=\"{}\"  members={}",
                        i + 1,
                        c.label,
                        c.member_ids.len()
                    );
                    for id in c.member_ids.iter().take(5) {
                        let label = workspace
                            .get_node(*id)
                            .map(|n| n.content.as_str())
                            .unwrap_or("?");
                        println!("       • {label}");
                    }
                    if c.member_ids.len() > 5 {
                        println!("       … {} more", c.member_ids.len() - 5);
                    }
                }
            }
        }
        Command::ExportDot { path } => {
            let workspace = load_workspace(&config)?;
            let dot = export_dot(&workspace);
            let out_path = path.unwrap_or_else(|| config.data_dir.join("graph.dot"));
            fs::write(&out_path, &dot)?;
            println!("DOT graph written to {}", out_path.display());
        }
        Command::ExportCsv { path } => {
            let workspace = load_workspace(&config)?;
            let csv = export_csv(&workspace);
            let out_path = path.unwrap_or_else(|| config.data_dir.join("nodes.csv"));
            fs::write(&out_path, &csv)?;
            println!("Nodes CSV written to {}", out_path.display());
        }
        Command::ExportEdgesCsv { path } => {
            let workspace = load_workspace(&config)?;
            let csv = export_edges_csv(&workspace);
            let out_path = path.unwrap_or_else(|| config.data_dir.join("edges.csv"));
            fs::write(&out_path, &csv)?;
            println!("Edges CSV written to {}", out_path.display());
        }
        Command::ExportMarkdown { path } => {
            let workspace = load_workspace(&config)?;
            let md = export_markdown(&workspace);
            let out_path = path.unwrap_or_else(|| config.data_dir.join("workspace.md"));
            fs::write(&out_path, &md)?;
            println!("Markdown export written to {}", out_path.display());
        }
        Command::ImportText { file } => {
            let content = fs::read_to_string(&file)?;
            let mut workspace = load_workspace(&config)?;
            let engine = MaterializationEngine::new();
            let mut added = 0usize;
            for line in content.lines() {
                let line = line.trim();
                if line.is_empty() || line.starts_with('#') {
                    continue;
                }
                workspace.materialize_thought(&engine, line)?;
                added += 1;
            }
            recompute_workspace(&mut workspace);
            persist_and_render(&workspace, &config)?;
            println!("Imported {added} thoughts from {}", file.display());
        }
        Command::Api { port } => {
            let workspace = load_workspace(&config)?;
            println!("[api] Starting REST API server on port {port}");
            Runtime::new()?.block_on(start_api_server(
                workspace,
                port,
                config.sqlite_path.clone(),
            ))?;
        }
        Command::MlTrain => {
            println!("[ML] Training all models from your data…");
            let status = std::process::Command::new(python_executable())
                .args(["-m", "silentnode_py.ml.cli", "train"])
                .status()
                .map_err(|e| format!("python executable not found: {e}"))?;
            if !status.success() {
                eprintln!("[ML] Training failed. Run: .venv/bin/pip install -r requirements.txt");
            }
        }
        Command::MlStatus => {
            let out = std::process::Command::new(python_executable())
                .args(["-m", "silentnode_py.ml.cli", "status"])
                .output()
                .map_err(|e| format!("python executable not found: {e}"))?;
            print!("{}", String::from_utf8_lossy(&out.stdout));
        }
        Command::MlGhostRisk => {
            let out = std::process::Command::new(python_executable())
                .args(["-m", "silentnode_py.ml.cli", "ghost-risk"])
                .output()
                .map_err(|e| format!("python executable not found: {e}"))?;
            print!("{}", String::from_utf8_lossy(&out.stdout));
        }
        Command::Tui => {
            let workspace = load_workspace(&config)?;
            run_tui(workspace, config.sqlite_path.clone())?;
        }
        Command::Dashboard { path } => {
            let workspace = load_workspace(&config)?;
            let html = export_html_dashboard(&workspace);
            let out_path = path.unwrap_or_else(|| config.data_dir.join("dashboard.html"));
            fs::write(&out_path, &html)?;
            println!("Dashboard written to {}", out_path.display());
            println!(
                "Open in browser: file://{}",
                out_path.canonicalize()?.display()
            );
        }

        // ── Phase 12: Analytics ──────────────────────────────────────────────
        Command::Analytics => {
            let workspace = load_workspace(&config)?;
            let engine = AnalyticsEngine::new();
            let health = engine.health_report(&workspace);
            println!("{}", health.summary());
            println!("  avg entropy:  {:.3}", health.avg_entropy);
            println!("  density:      {:.4}", health.density);
            println!("  activity:     {:.0}%", health.activity_rate * 100.0);
            println!("  decay ratio:  {:.0}%", health.decay_ratio * 100.0);
            println!("  components:   {}", health.component_count);
            println!("  bridges:      {}", health.bridge_count);
        }
        Command::Pagerank { limit } => {
            let workspace = load_workspace(&config)?;
            let entries = AnalyticsEngine::new().pagerank(&workspace, limit);
            if entries.is_empty() {
                println!("No nodes found.");
            } else {
                println!("PageRank (top {}):", entries.len());
                for (i, e) in entries.iter().enumerate() {
                    println!(
                        "  {:>2}. score={:.5}  {}",
                        i + 1,
                        e.score,
                        e.content_preview
                    );
                }
            }
        }
        Command::Bridges => {
            let workspace = load_workspace(&config)?;
            let bridges = AnalyticsEngine::new().find_bridges(&workspace);
            if bridges.is_empty() {
                println!("No bridge edges found — graph is well-connected.");
            } else {
                println!("Bridge Edges ({}):", bridges.len());
                for b in &bridges {
                    println!(
                        "  {} → {}  w={:.2}",
                        b.source_preview, b.target_preview, b.weight
                    );
                }
            }
        }
        Command::Health => {
            let workspace = load_workspace(&config)?;
            let h = AnalyticsEngine::new().health_report(&workspace);
            println!("{}", h.summary());
        }

        // ── Phase 13: Dream + Synthesis ──────────────────────────────────────
        Command::Dream => {
            let workspace = load_workspace(&config)?;
            let proposals = DreamEngine::new().generate(&workspace);
            if proposals.is_empty() {
                println!("No dream proposals (workspace may be sparse).");
            } else {
                println!("Dream Proposals ({}):", proposals.len());
                for p in &proposals {
                    println!("  [{:.2}]  {}", p.confidence, p.description);
                }
            }
        }
        Command::Synthesize { text } => {
            let workspace = load_workspace(&config)?;
            let narrative = SynthesisEngine::new().synthesize_topic(&workspace, &text);
            println!("{narrative}");
        }
        Command::KnowledgeGaps { topic } => {
            let workspace = load_workspace(&config)?;
            let gaps = SynthesisEngine::new().find_knowledge_gaps(&workspace, &topic);
            if gaps.is_empty() {
                println!("No knowledge gaps found for «{topic}» — all terms are in the graph.");
            } else {
                println!(
                    "Knowledge gaps for «{topic}» ({} missing terms):",
                    gaps.len()
                );
                for g in &gaps {
                    println!("  • {g}");
                }
            }
        }
        Command::ThoughtChain { from, to } => {
            let workspace = load_workspace(&config)?;
            let from_id = Uuid::parse_str(&from)?;
            let to_id = Uuid::parse_str(&to)?;
            let engine = SynthesisEngine::new();
            match engine.trace_causal_chain(&workspace, from_id, to_id) {
                None => println!("No path found from {from} to {to}."),
                Some(path) => {
                    println!("Causal chain ({} hops):", path.len() - 1);
                    println!("  {}", engine.format_chain(&workspace, &path));
                    println!();
                    for id in &path {
                        let label = workspace
                            .get_node(*id)
                            .map(|n| n.content.as_str())
                            .unwrap_or("?");
                        println!("  {} — {}", id, label);
                    }
                }
            }
        }

        // ── Phase 6: Python Pattern Intelligence ─────────────────────────────
        #[cfg(not(feature = "python"))]
        Command::DeepRituals
        | Command::DeepSeason
        | Command::DeepOracle
        | Command::DeepMirror
        | Command::DeepHeatmap { .. }
        | Command::DeepContracts
        | Command::DeepResonances { .. }
        | Command::LouvainCivs { .. }
        | Command::DeepShadows
        | Command::DeepAudioParams
        | Command::DeepAudioParametric
        | Command::DeepIngest { .. }
        | Command::DeepSignature => {
            eprintln!("Python analysis commands require --features python.");
            eprintln!("Build with: cargo run --features python -- <command>");
        }

        #[cfg(feature = "python")]
        Command::DeepRituals => {
            use pyo3::Python;
            let workspace = load_workspace(&config)?;
            let ws_json = py_workspace_json(&workspace);
            Python::with_gil(|py| -> pyo3::PyResult<()> {
                silentnode_core::python::ensure_path(py)?;
                let module = py.import_bound("silentnode_py.patterns.ritual")?;
                let result: String = module
                    .call_method1("detect_rituals", (ws_json.as_str(),))?
                    .extract()?;
                print_py_analysis("Ritual Detection (DBSCAN)", &result);
                Ok(())
            })
            .unwrap_or_else(|e| eprintln!("Python error: {e}"));
        }

        #[cfg(feature = "python")]
        Command::DeepSeason => {
            use pyo3::Python;
            let workspace = load_workspace(&config)?;
            let ws_json = py_workspace_json(&workspace);
            Python::with_gil(|py| -> pyo3::PyResult<()> {
                silentnode_core::python::ensure_path(py)?;
                let module = py.import_bound("silentnode_py.patterns.seasons")?;
                let result: String = module
                    .call_method1("detect_season", (ws_json.as_str(),))?
                    .extract()?;
                print_py_analysis("Cognitive Season (Statistical)", &result);
                Ok(())
            })
            .unwrap_or_else(|e| eprintln!("Python error: {e}"));
        }

        #[cfg(feature = "python")]
        Command::DeepOracle => {
            use pyo3::Python;
            let workspace = load_workspace(&config)?;
            let ws_json = py_workspace_json(&workspace);
            Python::with_gil(|py| -> pyo3::PyResult<()> {
                silentnode_core::python::ensure_path(py)?;
                let module = py.import_bound("silentnode_py.patterns.oracle")?;
                let result: String = module
                    .call_method1("generate_signals", (ws_json.as_str(),))?
                    .extract()?;
                print_py_analysis("Oracle Signals", &result);
                Ok(())
            })
            .unwrap_or_else(|e| eprintln!("Python error: {e}"));
        }

        #[cfg(feature = "python")]
        Command::DeepMirror => {
            use pyo3::Python;
            let workspace = load_workspace(&config)?;
            let ws_json = py_workspace_json(&workspace);
            Python::with_gil(|py| -> pyo3::PyResult<()> {
                silentnode_core::python::ensure_path(py)?;
                let module = py.import_bound("silentnode_py.patterns.mirror")?;
                let result: String = module
                    .call_method1("generate_portrait", (ws_json.as_str(),))?
                    .extract()?;
                print_py_analysis("Cognitive Mirror", &result);
                Ok(())
            })
            .unwrap_or_else(|e| eprintln!("Python error: {e}"));
        }

        #[cfg(feature = "python")]
        Command::DeepHeatmap { window } => {
            use pyo3::Python;
            let workspace = load_workspace(&config)?;
            let ws_json = py_workspace_json(&workspace);
            Python::with_gil(|py| -> pyo3::PyResult<()> {
                silentnode_core::python::ensure_path(py)?;
                let module = py.import_bound("silentnode_py.patterns.heatmap")?;
                let result: String = module
                    .call_method1("compute_heatmap", (ws_json.as_str(), window as i64))?
                    .extract()?;
                print_py_analysis(&format!("Thought Heatmap ({window}d window)"), &result);
                Ok(())
            })
            .unwrap_or_else(|e| eprintln!("Python error: {e}"));
        }

        #[cfg(feature = "python")]
        Command::DeepContracts => {
            use pyo3::Python;
            let workspace = load_workspace(&config)?;
            let ws_json = py_workspace_json(&workspace);
            Python::with_gil(|py| -> pyo3::PyResult<()> {
                silentnode_core::python::ensure_path(py)?;
                let module = py.import_bound("silentnode_py.patterns.contracts")?;
                let result: String = module
                    .call_method1("detect_contracts", (ws_json.as_str(),))?
                    .extract()?;
                print_py_analysis("Silent Contracts", &result);
                Ok(())
            })
            .unwrap_or_else(|e| eprintln!("Python error: {e}"));
        }

        #[cfg(feature = "python")]
        Command::DeepResonances { threshold } => {
            use pyo3::Python;
            let workspace = load_workspace(&config)?;
            let ws_json = py_workspace_json(&workspace);
            Python::with_gil(|py| -> pyo3::PyResult<()> {
                silentnode_core::python::ensure_path(py)?;
                let module = py.import_bound("silentnode_py.patterns.resonance")?;
                let result: String = module
                    .call_method1("find_resonances", (ws_json.as_str(), threshold as f64))?
                    .extract()?;
                print_py_analysis("Deep Resonances (TF-IDF)", &result);
                Ok(())
            })
            .unwrap_or_else(|e| eprintln!("Python error: {e}"));
        }

        #[cfg(feature = "python")]
        Command::LouvainCivs { resolution } => {
            use pyo3::Python;
            let workspace = load_workspace(&config)?;
            let ws_json = py_workspace_json(&workspace);
            Python::with_gil(|py| -> pyo3::PyResult<()> {
                silentnode_core::python::ensure_path(py)?;
                let module = py.import_bound("silentnode_py.clusters.civilization")?;
                let result: String = module
                    .call_method1(
                        "detect_civilizations",
                        (ws_json.as_str(), resolution as f64),
                    )?
                    .extract()?;
                print_py_analysis("Civilization Detection (Louvain)", &result);
                Ok(())
            })
            .unwrap_or_else(|e| eprintln!("Python error: {e}"));
        }

        #[cfg(feature = "python")]
        Command::DeepShadows => {
            use pyo3::Python;
            let workspace = load_workspace(&config)?;
            let ws_json = py_workspace_json(&workspace);
            Python::with_gil(|py| -> pyo3::PyResult<()> {
                silentnode_core::python::ensure_path(py)?;
                let module = py.import_bound("silentnode_py.clusters.shadow")?;
                let result: String = module
                    .call_method1("detect_shadows", (ws_json.as_str(),))?
                    .extract()?;
                print_py_analysis("Digital Shadows", &result);
                Ok(())
            })
            .unwrap_or_else(|e| eprintln!("Python error: {e}"));
        }

        // ── Phase 9: Audio ────────────────────────────────────────────────────
        Command::AudioState => {
            #[cfg(feature = "audio")]
            {
                use silentnode_core::audio::{atmosphere_from_season, AudioEngine};
                use silentnode_core::systems::CognitiveSeason;
                let workspace = load_workspace(&config)?;
                let report = workspace.cognitive_season();
                let season_str = match report.season {
                    CognitiveSeason::Spring => "spring",
                    CognitiveSeason::Summer => "summer",
                    CognitiveSeason::Autumn => "autumn",
                    CognitiveSeason::Winter => "winter",
                };
                let engine = AudioEngine::new();
                engine.set_atmosphere(atmosphere_from_season(season_str));
                let json = engine.to_state_json();
                println!("══ Audio Engine State ══════════════════════════════════════════");
                println!("{json}");
            }
            #[cfg(not(feature = "audio"))]
            {
                eprintln!("Audio commands require --features audio.");
                eprintln!("Build with: cargo run --features audio -- audio-state");
            }
        }

        Command::AudioPlay {
            ref atmosphere,
            seconds,
        } => {
            #[cfg(feature = "audio")]
            {
                use silentnode_core::audio::{AtmosphereKind, AudioEngine, SoundMode};
                let kind = AtmosphereKind::from_str(atmosphere);
                let engine = AudioEngine::new();
                engine.set_atmosphere(kind.clone());
                engine.set_mode(SoundMode::Full);
                println!(
                    "▶  Playing atmosphere: {} — {}",
                    kind.as_str(),
                    kind.description()
                );
                println!("   Duration: {seconds}s  (Ctrl-C to stop early)");
                std::thread::sleep(std::time::Duration::from_secs(seconds));
                println!("■  Done.");
            }
            #[cfg(not(feature = "audio"))]
            {
                eprintln!("Audio commands require --features audio.");
                let _ = (atmosphere, seconds);
            }
        }

        Command::AudioStop => {
            #[cfg(feature = "audio")]
            {
                use silentnode_core::audio::{AudioEngine, SoundMode};
                let engine = AudioEngine::new();
                engine.set_mode(SoundMode::Silence);
                println!("■  Audio stopped.");
                std::thread::sleep(std::time::Duration::from_millis(500));
            }
            #[cfg(not(feature = "audio"))]
            eprintln!("Audio commands require --features audio.");
        }

        Command::AudioList => {
            println!("══ Available Atmospheres ════════════════════════════════════════");
            let all = [
                (
                    "research",
                    "ambient hum + harmonic resonance, cavernous reverb",
                ),
                (
                    "creative",
                    "warm 432Hz tone, organic vibrato, expansive atmosphere",
                ),
                ("technical", "rhythmic 80Hz pulse, 120 BPM gate, tight room"),
                (
                    "personal",
                    "A3 intimate drone, heavy reverb, emotional warmth",
                ),
                ("ghost", "barely-present 85Hz, near-infinite reverb tail"),
                ("void", "absolute silence — felt absence of sound"),
                (
                    "crystal",
                    "pure C4 harmonics, no drift, crystalline clarity",
                ),
                (
                    "entropy",
                    "dissonant harmonics, turbulent fast LFO, fragmented",
                ),
                (
                    "spring",
                    "C3 ascending energy, lively breathing, forward motion",
                ),
                ("summer", "E3 full warm resonance, peak vitality, sustained"),
                ("autumn", "A2 descending warmth, fading complexity"),
                (
                    "winter",
                    "A1 sub-bass incubation, near-silent, infinite decay",
                ),
                ("ambient", "neutral unobtrusive baseline"),
            ];
            for (name, desc) in all {
                println!("  {:<12}  {}", name, desc);
            }
        }

        #[cfg(feature = "python")]
        Command::DeepAudioParams => {
            use pyo3::Python;
            let workspace = load_workspace(&config)?;
            let ws_json = py_workspace_json(&workspace);
            Python::with_gil(|py| -> pyo3::PyResult<()> {
                silentnode_core::python::ensure_path(py)?;
                let module = py.import_bound("silentnode_py.audio.generator")?;
                let result: String = module
                    .call_method1("map_workspace_to_audio", (ws_json.as_str(),))?
                    .extract()?;
                print_py_analysis("Audio State Mapping (Phase 9)", &result);
                Ok(())
            })
            .unwrap_or_else(|e| eprintln!("Python error: {e}"));
        }

        #[cfg(feature = "python")]
        Command::DeepAudioParametric => {
            use pyo3::Python;
            let workspace = load_workspace(&config)?;
            let ws_json = py_workspace_json(&workspace);
            Python::with_gil(|py| -> pyo3::PyResult<()> {
                silentnode_core::python::ensure_path(py)?;
                let module = py.import_bound("silentnode_py.audio.generator")?;
                let result: String = module
                    .call_method1("map_workspace_to_audio_parametric", (ws_json.as_str(),))?
                    .extract()?;
                print_py_analysis("Audio Parametric Mapping (Phase 9.2 roadmap spec)", &result);
                Ok(())
            })
            .unwrap_or_else(|e| eprintln!("Python error: {e}"));
        }

        #[cfg(feature = "python")]
        Command::DeepIngest { activity_json } => {
            use pyo3::Python;
            let workspace = load_workspace(&config)?;
            let ws_json = py_workspace_json(&workspace);
            Python::with_gil(|py| -> pyo3::PyResult<()> {
                silentnode_core::python::ensure_path(py)?;
                let module = py.import_bound("silentnode_py.ingestion.engine")?;
                let result: String = module
                    .call_method1(
                        "ingest_activity",
                        (activity_json.as_str(), ws_json.as_str()),
                    )?
                    .extract()?;
                print_py_analysis("Activity Ingestion (Phase 8)", &result);
                Ok(())
            })
            .unwrap_or_else(|e| eprintln!("Python error: {e}"));
        }

        // ── Phase 10: Identity & Narrative ────────────────────────────────────
        Command::Signature => {
            let mut workspace = load_workspace(&config)?;
            let sig = workspace.derive_living_signature();
            println!("══ Living Signature ════════════════════════════════════════");
            println!("  {}", sig.summary());
            println!();
            println!(
                "  Geometry:  {:<10} — {}",
                sig.geometry.as_str(),
                sig.geometry.description()
            );
            println!(
                "  Symmetry:  {:<10} ({}-fold)",
                sig.symmetry.as_str(),
                sig.symmetry.fold_count()
            );
            println!(
                "  Motion:    {:<10} ({}ms cycle)",
                sig.motion.as_str(),
                sig.motion.animation_duration_ms()
            );
            println!();
            println!(
                "  Complexity: {:.2}  Vitality: {:.2}  Depth: {:.2}",
                sig.complexity, sig.vitality, sig.depth
            );
            let pc = sig.primary_color;
            let sc = sig.secondary_color;
            let ac = sig.accent_color;
            println!("  Colors: primary rgb({:.0},{:.0},{:.0}) / secondary rgb({:.0},{:.0},{:.0}) / accent rgb({:.0},{:.0},{:.0})",
                pc[0]*255.0, pc[1]*255.0, pc[2]*255.0,
                sc[0]*255.0, sc[1]*255.0, sc[2]*255.0,
                ac[0]*255.0, ac[1]*255.0, ac[2]*255.0);
            println!("  Evolution: v{}", sig.evolution_count);
            if !workspace.identity.shift_history.is_empty() {
                println!("  Recent shifts:");
                for s in workspace.identity.shift_history.iter().rev().take(5) {
                    println!(
                        "    {} — {} (magnitude {:.2})",
                        s.at.format("%Y-%m-%d"),
                        s.description,
                        s.magnitude
                    );
                }
            }
        }

        Command::SignatureSvg { path } => {
            #[cfg(feature = "python")]
            {
                use pyo3::Python;
                let workspace = load_workspace(&config)?;
                let ws_json = py_workspace_json(&workspace);
                let out_path = path.unwrap_or_else(|| config.data_dir.join("signature.svg"));
                Python::with_gil(|py| -> pyo3::PyResult<()> {
                    silentnode_core::python::ensure_path(py)?;
                    let module = py.import_bound("silentnode_py.identity.signature")?;
                    let svg: String = module
                        .call_method1("render_signature_svg", (ws_json.as_str(), 320))?
                        .extract()?;
                    std::fs::write(&out_path, &svg).ok();
                    println!("Living Signature SVG written to {}", out_path.display());
                    Ok(())
                })
                .unwrap_or_else(|e| eprintln!("Python error: {e}"));
            }
            #[cfg(not(feature = "python"))]
            {
                let _ = path;
                eprintln!("signature-svg requires --features python (SVG rendering is in Python).");
            }
        }

        Command::ShadowProjects => {
            let workspace = load_workspace(&config)?;
            let shadows = workspace.detect_shadow_projects();
            println!("══ Shadow Projects ═════════════════════════════════════════");
            println!("  \"What you chose not to build reveals as much as what you did.\"");
            println!();
            if shadows.is_empty() {
                println!("  No shadow projects detected — no abandoned or unrealized paths yet.");
            } else {
                println!("  {} shadow project(s) detected:", shadows.len());
                for sp in &shadows {
                    println!("  {}", sp.summary());
                    println!("      {}", sp.description);
                }
            }
        }

        #[cfg(feature = "python")]
        Command::DeepSignature => {
            use pyo3::Python;
            let workspace = load_workspace(&config)?;
            let ws_json = py_workspace_json(&workspace);
            Python::with_gil(|py| -> pyo3::PyResult<()> {
                silentnode_core::python::ensure_path(py)?;
                let module = py.import_bound("silentnode_py.identity.signature")?;
                let gen = module.getattr("LivingSignatureGenerator")?.call0()?;
                let report: String = gen
                    .call_method1("full_report", (ws_json.as_str(),))?
                    .extract()?;
                if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&report) {
                    if let Some(ascii) = parsed.get("ascii").and_then(|v| v.as_str()) {
                        println!("══ Living Signature (rendered) ═════════════════════════════");
                        println!("{ascii}");
                        println!();
                    }
                    if let Some(desc) = parsed.get("description") {
                        if let Some(summary) = desc.get("summary").and_then(|v| v.as_str()) {
                            println!("  {summary}");
                        }
                        if let Some(geo) = desc.get("geometry").and_then(|v| v.as_str()) {
                            println!("  {geo}");
                        }
                        if let Some(m) = desc.get("motion").and_then(|v| v.as_str()) {
                            println!("  {m}");
                        }
                    }
                } else {
                    print_py_analysis("Living Signature (Phase 10)", &report);
                }
                Ok(())
            })
            .unwrap_or_else(|e| eprintln!("Python error: {e}"));
        }

        Command::LoreChronicle => {
            #[cfg(feature = "python")]
            {
                use pyo3::Python;
                let workspace = load_workspace(&config)?;
                let ws_json = py_workspace_json(&workspace);
                Python::with_gil(|py| -> pyo3::PyResult<()> {
                    silentnode_core::python::ensure_path(py)?;
                    let module = py.import_bound("silentnode_py.identity.chronicle")?;
                    let chronicle: String = module
                        .call_method1("generate_chronicle", (ws_json.as_str(),))?
                        .extract()?;
                    println!("{chronicle}");
                    Ok(())
                })
                .unwrap_or_else(|e| eprintln!("Python error: {e}"));
            }
            #[cfg(not(feature = "python"))]
            {
                // Rust-only fallback: show lore arcs without narrative
                let workspace = load_workspace(&config)?;
                let arcs = workspace.detect_lore(&[]);
                println!("══ LORE CHRONICLE ══════════════════════════════════════════");
                println!("  (Full narrative requires --features python)");
                println!();
                if arcs.is_empty() {
                    println!("  No lore arcs detected yet.");
                    println!("  Arcs emerge from tectonic events, long seasons,");
                    println!("  crystallizations, and temporal patterns.");
                } else {
                    for (i, arc) in arcs.iter().enumerate() {
                        println!(
                            "  {:2}. [{:?}] {} (sig={:.2})",
                            i + 1,
                            arc.arc_type,
                            arc.title,
                            arc.significance
                        );
                        if !arc.narrative.is_empty() {
                            println!("      {}", arc.narrative);
                        }
                        println!();
                    }
                }
            }
        }

        Command::HeroesJourney => {
            #[cfg(feature = "python")]
            {
                use pyo3::Python;
                let workspace = load_workspace(&config)?;
                let ws_json = py_workspace_json(&workspace);
                Python::with_gil(|py| -> pyo3::PyResult<()> {
                    silentnode_core::python::ensure_path(py)?;
                    let module = py.import_bound("silentnode_py.identity.chronicle")?;
                    let narrative: String = module
                        .call_method1("heroes_journey_narrative", (ws_json.as_str(),))?
                        .extract()?;
                    println!("{narrative}");
                    Ok(())
                })
                .unwrap_or_else(|e| eprintln!("Python error: {e}"));
            }
            #[cfg(not(feature = "python"))]
            {
                // Rust-only fallback: summarise arc distribution
                let workspace = load_workspace(&config)?;
                let arcs = workspace.detect_lore(&[]);
                println!("══ HERO'S JOURNEY MAPPING ══════════════════════════════════");
                println!("  (Full narrative requires --features python)");
                println!();
                if arcs.is_empty() {
                    println!("  No lore arcs detected. The journey has not yet begun.");
                } else {
                    use std::collections::HashMap;
                    let mut stage_counts: HashMap<String, usize> = HashMap::new();
                    for arc in &arcs {
                        let stage = match arc.arc_type {
                            silentnode_core::domain::ArcType::Origin => "THE CALL",
                            silentnode_core::domain::ArcType::Tectonic => "THE THRESHOLD",
                            silentnode_core::domain::ArcType::Conflict => "THE TRIALS",
                            silentnode_core::domain::ArcType::Revelation => "TRANSFORMATION",
                            silentnode_core::domain::ArcType::Transformation => "TRANSFORMATION",
                            silentnode_core::domain::ArcType::Resolution => "THE RETURN",
                            silentnode_core::domain::ArcType::Legacy => "THE RETURN",
                        };
                        *stage_counts.entry(stage.to_string()).or_insert(0) += 1;
                    }
                    for stage in &[
                        "THE CALL",
                        "THE THRESHOLD",
                        "THE TRIALS",
                        "TRANSFORMATION",
                        "THE RETURN",
                    ] {
                        let count = stage_counts.get(*stage).copied().unwrap_or(0);
                        let sym = if count > 0 { "▶" } else { "○" };
                        println!("  {sym} {stage:<16} ({count} arc(s))");
                    }
                    println!();
                    println!("  Total arcs: {}", arcs.len());
                }
            }
        }
    }

    Ok(())
}

#[derive(Debug, Clone)]
struct Paths {
    data_dir: PathBuf,
    sqlite_path: PathBuf,
    surreal_path: PathBuf,
    html_path: PathBuf,
    vault_dir: PathBuf,
    backend: BackendKind,
}

impl Paths {
    fn new() -> Self {
        let data_dir = PathBuf::from("data");
        Self {
            sqlite_path: data_dir.join("silentnode.sqlite"),
            surreal_path: data_dir.join("silentnode.surkv"),
            html_path: data_dir.join("universe.html"),
            vault_dir: data_dir.join("vault"),
            backend: BackendKind::from_env(),
            data_dir,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BackendKind {
    Sqlite,
    Surreal,
}

impl BackendKind {
    fn from_env() -> Self {
        match env::var("SILENTNODE_BACKEND") {
            Ok(value) if value.eq_ignore_ascii_case("surreal") => Self::Surreal,
            _ => Self::Sqlite,
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Sqlite => "sqlite",
            Self::Surreal => "surreal",
        }
    }
}

#[derive(Debug, Clone)]
enum Command {
    Help,
    Status,
    ListNodes {
        query: Option<String>,
    },
    ShowNode {
        node_id: String,
    },
    Seed,
    MigrateToSurreal,
    SurrealHealth,
    Render,
    Recompute,
    Simulate {
        steps: usize,
        delta_time: f32,
    },
    AddThought {
        text: String,
    },
    Connect {
        source_id: String,
        target_id: String,
        weight: f32,
        edge_type: String,
    },
    Disconnect {
        source_id: String,
        target_id: String,
    },
    DeleteNode {
        node_id: String,
    },
    ReviveNode {
        node_id: String,
    },
    Journal {
        text: String,
        season: Option<String>,
    },
    JournalSearch {
        query: String,
    },
    JournalList {
        days: i64,
    },
    JournalRange {
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    },
    Focus {
        node_id: String,
        seconds: f32,
        depth: String,
    },
    ExportEncrypted {
        password: String,
        output: PathBuf,
    },
    ImportEncrypted {
        password: String,
        input: PathBuf,
    },
    RotateVaultPassword {
        current_password: String,
        new_password: String,
    },
    Trail {
        hours: i64,
    },
    NodeTrail {
        node_id: String,
        hours: i64,
    },
    NodeJournal {
        node_id: String,
        days: i64,
    },
    Heatmap {
        days: i64,
    },
    Launch {
        width: u32,
        height: u32,
    },
    Weather,
    Weight,
    Souls,
    Tectonics,
    // Phase 6 — Pattern Recognition
    Season,
    Oracle,
    Rituals,
    Mirror {
        days: u32,
    },
    Heatmap7 {
        days: u32,
    },
    Contracts,
    Resonances,
    // Phase 7 — Social Graph
    Civilizations,
    Crystallize,
    ShatterCrystal {
        node_id: String,
    }, // shatters the crystal containing this node
    Shadows,
    IlluminateShadow {
        node_id: String,
    },
    NameShadow {
        node_id: String,
        name: String,
    },
    ReleaseShadow {
        node_id: String,
    },
    VoidNode {
        node_id: String,
    },
    UnvoidNode {
        node_id: String,
    },
    VoidCheck {
        node_id: String,
    },
    FulfillContract {
        node_id: String,
    },
    ReleaseContract {
        node_id: String,
    },
    OpenChambers {
        threshold: f32,
    },
    // Phase 8 — External World Integration
    MembraneStatus,
    MembraneAllow {
        pattern: String,
    },
    MembraneBlock {
        pattern: String,
    },
    MembraneLog {
        last: usize,
    },
    PortalStatus,
    PortalAddFilesystem {
        path: String,
    },
    PortalAddExternal {
        name: String,
        kind: String,
        url: String,
    },
    ProcessStatus,
    ProcessLink {
        pid: i64,
        node_id: String,
    },
    CalendarStatus,
    CalendarAdd {
        title: String,
        days_ahead: i64,
        category: String,
    },
    CalendarPrep {
        event_id: String,
    },
    // Sync
    SyncServe {
        port: u16,
    },
    SyncPull {
        addr: String,
    },
    SyncPush {
        addr: String,
    },
    // Phase 9 — Intelligence + Export + API
    Suggestions {
        limit: usize,
    },
    Related {
        node_id: String,
        limit: usize,
    },
    Clusters {
        k: usize,
    },
    ExportDot {
        path: Option<PathBuf>,
    },
    ExportCsv {
        path: Option<PathBuf>,
    },
    ExportEdgesCsv {
        path: Option<PathBuf>,
    },
    ExportMarkdown {
        path: Option<PathBuf>,
    },
    ImportText {
        file: PathBuf,
    },
    Api {
        port: u16,
    },
    // Phase 10 + 11
    MlTrain,
    MlStatus,
    MlGhostRisk,
    Tui,
    Dashboard {
        path: Option<PathBuf>,
    },
    // Phase 12 — Analytics
    Analytics,
    Pagerank {
        limit: usize,
    },
    Bridges,
    Health,
    // Phase 13 — Dream + Synthesis
    Dream,
    Synthesize {
        text: String,
    },
    KnowledgeGaps {
        topic: String,
    },
    ThoughtChain {
        from: String,
        to: String,
    },
    // Phase 6 — Python Pattern Intelligence
    DeepRituals,
    DeepSeason,
    DeepOracle,
    DeepMirror,
    DeepHeatmap {
        #[allow(dead_code)]
        window: usize,
    },
    DeepContracts,
    DeepResonances {
        #[allow(dead_code)]
        threshold: f32,
    },
    // Phase 7 — Python Social Graph
    LouvainCivs {
        #[allow(dead_code)]
        resolution: f32,
    },
    DeepShadows,
    // Phase 9 — Audio
    AudioState,
    AudioPlay {
        atmosphere: String,
        seconds: u64,
    },
    AudioStop,
    AudioList,
    DeepAudioParams,
    DeepAudioParametric,
    DeepIngest {
        #[allow(dead_code)]
        activity_json: String,
    },
    // Phase 10 — Identity & Narrative
    Signature,
    SignatureSvg {
        path: Option<PathBuf>,
    },
    ShadowProjects,
    DeepSignature,
    LoreChronicle,
    HeroesJourney,
    // Phase 5 — Temporal
    TemporalStatus,
    SnapshotAll,
    RecordChange {
        node_id: String,
    },
    Archaeology {
        node_id: String,
    },
    ReconstructDay {
        date: NaiveDate,
    },
    CompareDays {
        day_a: NaiveDate,
        day_b: NaiveDate,
    },
    FossilCheck {
        node_id: String,
    },
    Fossilize {
        node_id: String,
    },
    Excavate {
        node_id: String,
    },
    Lore,
}

impl Command {
    fn parse(args: &[String]) -> Result<Self, Box<dyn std::error::Error>> {
        if args.is_empty() {
            return Ok(Self::Status);
        }

        match args[0].as_str() {
            "help" | "--help" | "-h" => Ok(Self::Help),
            "status" => Ok(Self::Status),
            "list-nodes" => Ok(Self::ListNodes {
                query: args.get(1).map(|_| args[1..].join(" ")),
            }),
            "show-node" => {
                if args.len() < 2 {
                    return Err("show-node requires <node-id>".into());
                }
                Ok(Self::ShowNode {
                    node_id: args[1].clone(),
                })
            }
            "seed" => Ok(Self::Seed),
            "migrate-to-surreal" => Ok(Self::MigrateToSurreal),
            "surreal-health" => Ok(Self::SurrealHealth),
            "render" => Ok(Self::Render),
            "recompute" => Ok(Self::Recompute),
            "simulate" => Ok(Self::Simulate {
                steps: args
                    .get(1)
                    .map(|value| value.parse())
                    .transpose()?
                    .unwrap_or(12),
                delta_time: args
                    .get(2)
                    .map(|value| value.parse())
                    .transpose()?
                    .unwrap_or(1.0),
            }),
            "add-thought" => {
                let text = join_required(args, 1, "add-thought requires text")?;
                Ok(Self::AddThought { text })
            }
            "connect" => {
                if args.len() < 3 {
                    return Err(
                        "connect requires <source-id> <target-id> [weight] [edge-type]".into(),
                    );
                }
                Ok(Self::Connect {
                    source_id: args[1].clone(),
                    target_id: args[2].clone(),
                    weight: args
                        .get(3)
                        .map(|value| value.parse())
                        .transpose()?
                        .unwrap_or(0.7),
                    edge_type: args
                        .get(4)
                        .cloned()
                        .unwrap_or_else(|| "connection".to_string()),
                })
            }
            "disconnect" => {
                if args.len() < 3 {
                    return Err("disconnect requires <source-id> <target-id>".into());
                }
                Ok(Self::Disconnect {
                    source_id: args[1].clone(),
                    target_id: args[2].clone(),
                })
            }
            "delete-node" => {
                if args.len() < 2 {
                    return Err("delete-node requires <node-id>".into());
                }
                Ok(Self::DeleteNode {
                    node_id: args[1].clone(),
                })
            }
            "revive-node" => {
                if args.len() < 2 {
                    return Err("revive-node requires <node-id>".into());
                }
                Ok(Self::ReviveNode {
                    node_id: args[1].clone(),
                })
            }
            "journal" => {
                let text = join_required(args, 1, "journal requires text")?;
                Ok(Self::Journal { text, season: None })
            }
            "journal-season" => {
                if args.len() < 3 {
                    return Err("journal-season requires <season> <text>".into());
                }
                Ok(Self::Journal {
                    season: Some(args[1].clone()),
                    text: args[2..].join(" "),
                })
            }
            "journal-search" => {
                let query = join_required(args, 1, "journal-search requires text")?;
                Ok(Self::JournalSearch { query })
            }
            "journal-list" => Ok(Self::JournalList {
                days: args
                    .get(1)
                    .map(|value| value.parse())
                    .transpose()?
                    .unwrap_or(30),
            }),
            "journal-range" => {
                if args.len() < 3 {
                    return Err("journal-range requires <from-rfc3339> <to-rfc3339>".into());
                }
                Ok(Self::JournalRange {
                    from: DateTime::parse_from_rfc3339(&args[1])?.with_timezone(&Utc),
                    to: DateTime::parse_from_rfc3339(&args[2])?.with_timezone(&Utc),
                })
            }
            "focus" => {
                if args.len() < 4 {
                    return Err(
                        "focus requires <node-id> <seconds> <glance|read|edit|deep_work>".into(),
                    );
                }
                Ok(Self::Focus {
                    node_id: args[1].clone(),
                    seconds: args[2].parse()?,
                    depth: args[3].clone(),
                })
            }
            "export-encrypted" => {
                if args.len() < 2 {
                    return Err("export-encrypted requires <password> [output-path]".into());
                }
                Ok(Self::ExportEncrypted {
                    password: args[1].clone(),
                    output: args
                        .get(2)
                        .map(PathBuf::from)
                        .unwrap_or_else(|| PathBuf::from("data/workspace.snapshot.enc")),
                })
            }
            "import-encrypted" => {
                if args.len() < 2 {
                    return Err("import-encrypted requires <password> [input-path]".into());
                }
                Ok(Self::ImportEncrypted {
                    password: args[1].clone(),
                    input: args
                        .get(2)
                        .map(PathBuf::from)
                        .unwrap_or_else(|| PathBuf::from("data/workspace.snapshot.enc")),
                })
            }
            "rotate-vault-password" => {
                if args.len() < 3 {
                    return Err(
                        "rotate-vault-password requires <current-password> <new-password>".into(),
                    );
                }
                Ok(Self::RotateVaultPassword {
                    current_password: args[1].clone(),
                    new_password: args[2].clone(),
                })
            }
            "trail" => Ok(Self::Trail {
                hours: args
                    .get(1)
                    .map(|value| value.parse())
                    .transpose()?
                    .unwrap_or(24),
            }),
            "node-trail" => {
                if args.len() < 2 {
                    return Err("node-trail requires <node-id> [hours]".into());
                }
                Ok(Self::NodeTrail {
                    node_id: args[1].clone(),
                    hours: args
                        .get(2)
                        .map(|value| value.parse())
                        .transpose()?
                        .unwrap_or(72),
                })
            }
            "node-journal" => {
                if args.len() < 2 {
                    return Err("node-journal requires <node-id> [days]".into());
                }
                Ok(Self::NodeJournal {
                    node_id: args[1].clone(),
                    days: args
                        .get(2)
                        .map(|value| value.parse())
                        .transpose()?
                        .unwrap_or(30),
                })
            }
            "heatmap" => Ok(Self::Heatmap {
                days: args
                    .get(1)
                    .map(|value| value.parse())
                    .transpose()?
                    .unwrap_or(7),
            }),
            "launch" => Ok(Self::Launch {
                width: args.get(1).map(|v| v.parse()).transpose()?.unwrap_or(1280),
                height: args.get(2).map(|v| v.parse()).transpose()?.unwrap_or(720),
            }),
            "weather" => Ok(Self::Weather),
            "weight" => Ok(Self::Weight),
            "souls" => Ok(Self::Souls),
            "tectonics" => Ok(Self::Tectonics),
            // Phase 6
            "season" => Ok(Self::Season),
            "oracle" => Ok(Self::Oracle),
            "rituals" => Ok(Self::Rituals),
            "mirror" => Ok(Self::Mirror {
                days: args.get(1).map(|v| v.parse()).transpose()?.unwrap_or(30),
            }),
            "heatmap7" => Ok(Self::Heatmap7 {
                days: args.get(1).map(|v| v.parse()).transpose()?.unwrap_or(14),
            }),
            "contracts" => Ok(Self::Contracts),
            "resonances" => Ok(Self::Resonances),
            // Phase 7
            "civilizations" => Ok(Self::Civilizations),
            "crystallize" => Ok(Self::Crystallize),
            "shatter-crystal" => {
                if args.len() < 2 {
                    return Err("shatter-crystal requires <node-id>".into());
                }
                Ok(Self::ShatterCrystal {
                    node_id: args[1].clone(),
                })
            }
            "shadows" => Ok(Self::Shadows),
            "illuminate-shadow" => {
                if args.len() < 2 {
                    return Err("illuminate-shadow requires <node-id>".into());
                }
                Ok(Self::IlluminateShadow {
                    node_id: args[1].clone(),
                })
            }
            "name-shadow" => {
                if args.len() < 3 {
                    return Err("name-shadow requires <node-id> <name>".into());
                }
                Ok(Self::NameShadow {
                    node_id: args[1].clone(),
                    name: args[2..].join(" "),
                })
            }
            "release-shadow" => {
                if args.len() < 2 {
                    return Err("release-shadow requires <node-id>".into());
                }
                Ok(Self::ReleaseShadow {
                    node_id: args[1].clone(),
                })
            }
            "fulfill-contract" => {
                if args.len() < 2 {
                    return Err("fulfill-contract requires <node-id>".into());
                }
                Ok(Self::FulfillContract {
                    node_id: args[1].clone(),
                })
            }
            "release-contract" => {
                if args.len() < 2 {
                    return Err("release-contract requires <node-id>".into());
                }
                Ok(Self::ReleaseContract {
                    node_id: args[1].clone(),
                })
            }
            "open-chambers" => Ok(Self::OpenChambers {
                threshold: args.get(1).and_then(|v| v.parse().ok()).unwrap_or(0.45),
            }),
            "void-node" => {
                if args.len() < 2 {
                    return Err("void-node requires <node-id>".into());
                }
                Ok(Self::VoidNode {
                    node_id: args[1].clone(),
                })
            }
            "unvoid-node" => {
                if args.len() < 2 {
                    return Err("unvoid-node requires <node-id>".into());
                }
                Ok(Self::UnvoidNode {
                    node_id: args[1].clone(),
                })
            }
            "void-check" => {
                if args.len() < 2 {
                    return Err("void-check requires <node-id>".into());
                }
                Ok(Self::VoidCheck {
                    node_id: args[1].clone(),
                })
            }
            // Phase 5
            "temporal-status" => Ok(Self::TemporalStatus),
            "snapshot-all" => Ok(Self::SnapshotAll),
            "record-change" => {
                if args.len() < 2 {
                    return Err("record-change requires <node-id>".into());
                }
                Ok(Self::RecordChange {
                    node_id: args[1].clone(),
                })
            }
            "archaeology" => {
                if args.len() < 2 {
                    return Err("archaeology requires <node-id>".into());
                }
                Ok(Self::Archaeology {
                    node_id: args[1].clone(),
                })
            }
            "reconstruct-day" => {
                if args.len() < 2 {
                    return Err("reconstruct-day requires <YYYY-MM-DD>".into());
                }
                let date = NaiveDate::parse_from_str(&args[1], "%Y-%m-%d")
                    .map_err(|e| format!("invalid date: {e}"))?;
                Ok(Self::ReconstructDay { date })
            }
            "compare-days" => {
                if args.len() < 3 {
                    return Err("compare-days requires <YYYY-MM-DD> <YYYY-MM-DD>".into());
                }
                let day_a = NaiveDate::parse_from_str(&args[1], "%Y-%m-%d")
                    .map_err(|e| format!("invalid date: {e}"))?;
                let day_b = NaiveDate::parse_from_str(&args[2], "%Y-%m-%d")
                    .map_err(|e| format!("invalid date: {e}"))?;
                Ok(Self::CompareDays { day_a, day_b })
            }
            "fossil-check" => {
                if args.len() < 2 {
                    return Err("fossil-check requires <node-id>".into());
                }
                Ok(Self::FossilCheck {
                    node_id: args[1].clone(),
                })
            }
            "fossilize" => {
                if args.len() < 2 {
                    return Err("fossilize requires <node-id>".into());
                }
                Ok(Self::Fossilize {
                    node_id: args[1].clone(),
                })
            }
            "excavate" => {
                if args.len() < 2 {
                    return Err("excavate requires <node-id>".into());
                }
                Ok(Self::Excavate {
                    node_id: args[1].clone(),
                })
            }
            "lore" => Ok(Self::Lore),
            // Phase 8
            "membrane-status" => Ok(Self::MembraneStatus),
            "membrane-allow" => {
                if args.len() < 2 {
                    return Err("membrane-allow requires <pattern>".into());
                }
                Ok(Self::MembraneAllow {
                    pattern: args[1].clone(),
                })
            }
            "membrane-block" => {
                if args.len() < 2 {
                    return Err("membrane-block requires <pattern>".into());
                }
                Ok(Self::MembraneBlock {
                    pattern: args[1].clone(),
                })
            }
            "membrane-log" => Ok(Self::MembraneLog {
                last: args.get(1).and_then(|v| v.parse().ok()).unwrap_or(20),
            }),
            "portal-status" => Ok(Self::PortalStatus),
            "portal-add-fs" => {
                if args.len() < 2 {
                    return Err("portal-add-fs requires <path>".into());
                }
                Ok(Self::PortalAddFilesystem {
                    path: args[1].clone(),
                })
            }
            "portal-add" => {
                if args.len() < 4 {
                    return Err("portal-add requires <name> <kind> <url>".into());
                }
                Ok(Self::PortalAddExternal {
                    name: args[1].clone(),
                    kind: args[2].clone(),
                    url: args[3].clone(),
                })
            }
            "ps" | "process-status" => Ok(Self::ProcessStatus),
            "process-link" => {
                if args.len() < 3 {
                    return Err("process-link requires <pid> <node-id>".into());
                }
                Ok(Self::ProcessLink {
                    pid: args[1].parse()?,
                    node_id: args[2].clone(),
                })
            }
            "cal" | "calendar-status" => Ok(Self::CalendarStatus),
            "calendar-add" => {
                if args.len() < 4 {
                    return Err("calendar-add requires <title> <days-ahead> <category>".into());
                }
                Ok(Self::CalendarAdd {
                    title: args[1].clone(),
                    days_ahead: args[2].parse()?,
                    category: args[3].clone(),
                })
            }
            "calendar-prep" => {
                if args.len() < 2 {
                    return Err("calendar-prep requires <event-id>".into());
                }
                Ok(Self::CalendarPrep {
                    event_id: args[1].clone(),
                })
            }
            "sync-serve" => Ok(Self::SyncServe {
                port: args.get(1).map(|s| s.parse()).transpose()?.unwrap_or(7979),
            }),
            "sync-pull" => {
                let addr = args
                    .get(1)
                    .cloned()
                    .unwrap_or_else(|| "127.0.0.1:7979".to_string());
                Ok(Self::SyncPull { addr })
            }
            "sync-push" => {
                let addr = args
                    .get(1)
                    .cloned()
                    .unwrap_or_else(|| "127.0.0.1:7979".to_string());
                Ok(Self::SyncPush { addr })
            }
            // Phase 9
            "suggestions" => Ok(Self::Suggestions {
                limit: args.get(1).map(|v| v.parse()).transpose()?.unwrap_or(10),
            }),
            "related" => {
                if args.len() < 2 {
                    return Err("related requires <node-id> [limit]".into());
                }
                Ok(Self::Related {
                    node_id: args[1].clone(),
                    limit: args.get(2).map(|v| v.parse()).transpose()?.unwrap_or(10),
                })
            }
            "clusters" => Ok(Self::Clusters {
                k: args.get(1).map(|v| v.parse()).transpose()?.unwrap_or(5),
            }),
            "export-dot" => Ok(Self::ExportDot {
                path: args.get(1).map(PathBuf::from),
            }),
            "export-csv" => Ok(Self::ExportCsv {
                path: args.get(1).map(PathBuf::from),
            }),
            "export-edges-csv" => Ok(Self::ExportEdgesCsv {
                path: args.get(1).map(PathBuf::from),
            }),
            "export-markdown" => Ok(Self::ExportMarkdown {
                path: args.get(1).map(PathBuf::from),
            }),
            "import-text" => {
                if args.len() < 2 {
                    return Err("import-text requires <file>".into());
                }
                Ok(Self::ImportText {
                    file: PathBuf::from(&args[1]),
                })
            }
            "api" => Ok(Self::Api {
                port: args.get(1).map(|v| v.parse()).transpose()?.unwrap_or(3030),
            }),
            "ml-train" => Ok(Self::MlTrain),
            "ml-status" => Ok(Self::MlStatus),
            "ml-ghost-risk" => Ok(Self::MlGhostRisk),
            "tui" => Ok(Self::Tui),
            "dashboard" => Ok(Self::Dashboard {
                path: args.get(1).map(PathBuf::from),
            }),
            // Phase 12
            "analytics" => Ok(Self::Analytics),
            "pagerank" => Ok(Self::Pagerank {
                limit: args.get(1).map(|v| v.parse()).transpose()?.unwrap_or(20),
            }),
            "bridges" => Ok(Self::Bridges),
            "health" => Ok(Self::Health),
            // Phase 13
            "dream" => Ok(Self::Dream),
            "synthesize" => {
                let text = join_required(args, 1, "synthesize requires <text>")?;
                Ok(Self::Synthesize { text })
            }
            "knowledge-gaps" => {
                let topic = join_required(args, 1, "knowledge-gaps requires <topic>")?;
                Ok(Self::KnowledgeGaps { topic })
            }
            "thought-chain" => {
                if args.len() < 3 {
                    return Err("thought-chain requires <from-id> <to-id>".into());
                }
                Ok(Self::ThoughtChain {
                    from: args[1].clone(),
                    to: args[2].clone(),
                })
            }
            // Phase 6 — Python Pattern Intelligence
            "deep-rituals" => Ok(Self::DeepRituals),
            "deep-season" => Ok(Self::DeepSeason),
            "deep-oracle" => Ok(Self::DeepOracle),
            "deep-mirror" => Ok(Self::DeepMirror),
            "deep-heatmap" => Ok(Self::DeepHeatmap {
                window: args.get(1).map(|v| v.parse()).transpose()?.unwrap_or(30),
            }),
            "deep-contracts" => Ok(Self::DeepContracts),
            "deep-resonances" => Ok(Self::DeepResonances {
                threshold: args.get(1).map(|v| v.parse()).transpose()?.unwrap_or(0.35),
            }),
            // Phase 7 — Python Social Graph
            "louvain" => Ok(Self::LouvainCivs {
                resolution: args.get(1).map(|v| v.parse()).transpose()?.unwrap_or(1.2),
            }),
            "deep-shadows" => Ok(Self::DeepShadows),
            // Phase 9 — Audio
            "audio-state" => Ok(Self::AudioState),
            "audio-play" => Ok(Self::AudioPlay {
                atmosphere: args
                    .get(1)
                    .cloned()
                    .unwrap_or_else(|| "ambient".to_string()),
                seconds: args.get(2).and_then(|v| v.parse().ok()).unwrap_or(30),
            }),
            "audio-stop" => Ok(Self::AudioStop),
            "audio-list" => Ok(Self::AudioList),
            "deep-audio-params" => Ok(Self::DeepAudioParams),
            "deep-audio-parametric" => Ok(Self::DeepAudioParametric),
            "deep-ingest" => {
                if args.len() < 2 {
                    return Err("deep-ingest requires <activity-json>".into());
                }
                Ok(Self::DeepIngest {
                    activity_json: args[1].clone(),
                })
            }
            // Phase 10 — Identity & Narrative
            "lore-chronicle" => Ok(Self::LoreChronicle),
            "heroes-journey" => Ok(Self::HeroesJourney),
            "signature" => Ok(Self::Signature),
            "signature-svg" => Ok(Self::SignatureSvg {
                path: args.get(1).map(PathBuf::from),
            }),
            "shadow-projects" => Ok(Self::ShadowProjects),
            "deep-signature" => Ok(Self::DeepSignature),
            other => Err(format!("unknown command: {other}").into()),
        }
    }
}

fn join_required(
    args: &[String],
    start: usize,
    message: &'static str,
) -> Result<String, Box<dyn std::error::Error>> {
    if args.len() <= start {
        return Err(message.into());
    }
    Ok(args[start..].join(" "))
}

/// Serialize workspace to JSON for Python analysis modules.
#[cfg(feature = "python")]
fn py_workspace_json(workspace: &SilentNodeWorkspace) -> String {
    use serde_json::{json, Value};

    let nodes: Vec<Value> = workspace
        .graph
        .nodes()
        .map(|n| {
            json!({
                "id": n.id.to_string(),
                "content": n.content,
                "entropy": n.entropy,
                "gravity": n.gravity,
                "velocity": n.velocity,
                "is_ghost": n.is_ghost,
                "is_fossil": n.is_fossil,
                "is_void": n.is_void,
                "created_at": n.created_at.to_rfc3339(),
                "accessed_at": n.accessed_at.to_rfc3339(),
                "civilization_id": n.civilization_id.map(|id| id.to_string()),
            })
        })
        .collect();

    let edges: Vec<Value> = workspace
        .graph
        .edges()
        .map(|e| {
            json!({
                "source_id": e.source_id.to_string(),
                "target_id": e.target_id.to_string(),
                "weight": e.weight,
                "edge_type": format!("{:?}", e.edge_type).to_lowercase(),
            })
        })
        .collect();

    let focus_events: Vec<Value> = workspace
        .focus
        .events()
        .iter()
        .map(|e| {
            json!({
                "node_id": e.node_id.to_string(),
                "timestamp": e.timestamp.to_rfc3339(),
                "duration_seconds": e.duration_seconds,
                "depth": match e.depth {
                    FocusDepth::Glance   => "glance",
                    FocusDepth::Read     => "read",
                    FocusDepth::Edit     => "edit",
                    FocusDepth::DeepWork => "deep_work",
                },
            })
        })
        .collect();

    let journal_entries: Vec<Value> = workspace
        .journal
        .entries()
        .iter()
        .map(|j| {
            json!({
                "id": j.id.to_string(),
                "content": j.content,
                "timestamp": j.timestamp.to_rfc3339(),
                "linked_nodes": j.linked_nodes.iter().map(|id| id.to_string()).collect::<Vec<_>>(),
            })
        })
        .collect();

    serde_json::to_string(&json!({
        "nodes": nodes,
        "edges": edges,
        "focus_events": focus_events,
        "journal_entries": journal_entries,
    }))
    .unwrap_or_default()
}

/// Pretty-print Python analysis JSON result to stdout.
#[cfg(feature = "python")]
fn print_py_analysis(title: &str, json_str: &str) {
    println!("═══ {title} ═══");
    match serde_json::from_str::<serde_json::Value>(json_str) {
        Ok(v) => println!("{}", serde_json::to_string_pretty(&v).unwrap_or_default()),
        Err(_) => println!("{json_str}"),
    }
    println!();
}

fn load_workspace(config: &Paths) -> Result<SilentNodeWorkspace, Box<dyn std::error::Error>> {
    match config.backend {
        BackendKind::Sqlite => {
            let store = SqliteWorkspaceStore::new(&config.sqlite_path)?;
            match store.load_snapshot()? {
                Some(snapshot) => Ok(SilentNodeWorkspace::from_snapshot(snapshot)?),
                None => Ok(SilentNodeWorkspace::new()),
            }
        }
        BackendKind::Surreal => {
            let runtime = Runtime::new()?;
            let store = runtime.block_on(SurrealWorkspaceStore::new_local(&config.surreal_path))?;
            let snapshot = runtime.block_on(store.load_snapshot())?;
            if snapshot.graph.nodes.is_empty()
                && snapshot.graph.edges.is_empty()
                && snapshot.focus_events.is_empty()
                && snapshot.journal_entries.is_empty()
            {
                Ok(SilentNodeWorkspace::new())
            } else {
                Ok(SilentNodeWorkspace::from_snapshot(snapshot)?)
            }
        }
    }
}

fn persist_and_render(
    workspace: &SilentNodeWorkspace,
    config: &Paths,
) -> Result<(), Box<dyn std::error::Error>> {
    match config.backend {
        BackendKind::Sqlite => {
            let mut store = SqliteWorkspaceStore::new(&config.sqlite_path)?;
            store.save_snapshot(&workspace.snapshot())?;
        }
        BackendKind::Surreal => {
            let runtime = Runtime::new()?;
            let store = runtime.block_on(SurrealWorkspaceStore::new_local(&config.surreal_path))?;
            runtime.block_on(store.save_snapshot(&workspace.snapshot()))?;
        }
    }
    workspace.render_html(&VisualizationEngine::new(), &config.html_path)?;
    Ok(())
}

fn migrate_default_sqlite_to_surreal(config: &Paths) -> Result<(), Box<dyn std::error::Error>> {
    let runtime = Runtime::new()?;
    runtime.block_on(migrate_sqlite_to_surreal_path(
        &config.sqlite_path,
        &config.surreal_path,
    ))?;
    Ok(())
}

fn surreal_health(
    config: &Paths,
) -> Result<silentnode_core::SurrealTableCounts, Box<dyn std::error::Error>> {
    let runtime = Runtime::new()?;
    let store = runtime.block_on(SurrealWorkspaceStore::new_local(&config.surreal_path))?;
    Ok(runtime.block_on(store.healthcheck())?)
}

fn rotate_vault_password(
    config: &Paths,
    current_password: &str,
    new_password: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut vault = Vault::open(current_password, &config.vault_dir)?;
    vault.change_password(new_password)?;
    Ok(())
}

fn recompute_workspace(workspace: &mut SilentNodeWorkspace) {
    workspace.tick_entropy(&EntropyEngine::new());
    workspace.step_gravity(&GravityEngine::new(), 1.0);
}

fn make_node(nt: NodeType, content: &str, x: f32, y: f32, z: f32) -> NodeData {
    let mut n = NodeData::new(nt, content);
    n.position = Position3 { x, y, z };
    n
}

fn seed_workspace(workspace: &mut SilentNodeWorkspace) -> Result<(), Box<dyn std::error::Error>> {
    if workspace.graph.node_count() > 0 {
        return Ok(());
    }

    // ── core cluster (centre) ────────────────────────────────
    let core = workspace.graph.add_node(make_node(
        NodeType::Project,
        "SilentNode Core",
        0.0,
        0.0,
        0.0,
    ))?;
    let graph = workspace.graph.add_node(make_node(
        NodeType::Idea,
        "Cognitive Graph Engine",
        6.0,
        3.0,
        0.0,
    ))?;
    let vault = workspace.graph.add_node(make_node(
        NodeType::Artifact,
        "Encryption Vault",
        -6.0,
        3.0,
        0.0,
    ))?;
    let store = workspace.graph.add_node(make_node(
        NodeType::Artifact,
        "SQLite + Surreal Storage",
        0.0,
        7.0,
        0.0,
    ))?;

    // ── physics cluster (right) ──────────────────────────────
    let grav = workspace.graph.add_node(make_node(
        NodeType::Idea,
        "Barnes-Hut Gravity",
        14.0,
        0.0,
        2.0,
    ))?;
    let entr = workspace.graph.add_node(make_node(
        NodeType::Process,
        "Entropy Engine",
        18.0,
        4.0,
        0.0,
    ))?;
    let cont = workspace.graph.add_node(make_node(
        NodeType::Process,
        "Contagion BFS",
        18.0,
        -4.0,
        0.0,
    ))?;

    // ── analysis cluster (left) ──────────────────────────────
    let sil = workspace.graph.add_node(make_node(
        NodeType::Idea,
        "Silence Analyzer TF-IDF",
        -14.0,
        0.0,
        2.0,
    ))?;
    let miss = workspace.graph.add_node(make_node(
        NodeType::Memory,
        "Missing Bridges",
        -18.0,
        4.0,
        0.0,
    ))?;
    let impl_ = workspace.graph.add_node(make_node(
        NodeType::Memory,
        "Implied Concepts",
        -18.0,
        -4.0,
        0.0,
    ))?;

    // ── rendering cluster (top) ──────────────────────────────
    let gpu = workspace.graph.add_node(make_node(
        NodeType::Project,
        "GPU Renderer (wgpu)",
        0.0,
        14.0,
        2.0,
    ))?;
    let cam = workspace.graph.add_node(make_node(
        NodeType::Artifact,
        "Camera Controller",
        5.0,
        18.0,
        0.0,
    ))?;
    let part = workspace.graph.add_node(make_node(
        NodeType::Process,
        "Particle System",
        -5.0,
        18.0,
        0.0,
    ))?;

    // ── person + world ───────────────────────────────────────
    let dev = workspace.graph.add_node(make_node(
        NodeType::Person,
        "NightOS Developer",
        0.0,
        -10.0,
        0.0,
    ))?;
    let night = workspace
        .graph
        .add_node(make_node(NodeType::World, "NightOS", 8.0, -14.0, 0.0))?;
    let ghost = {
        let mut n = make_node(NodeType::Ghost, "Forgotten Idea", -8.0, -14.0, 0.0);
        n.is_ghost = true;
        n.entropy = 0.85;
        workspace.graph.add_node(n)?
    };

    // ── entropy variation (visual diversity) ─────────────────
    if let Some(n) = workspace.graph.get_node_mut(entr) {
        n.entropy = 0.55;
    }
    if let Some(n) = workspace.graph.get_node_mut(cont) {
        n.entropy = 0.40;
    }
    if let Some(n) = workspace.graph.get_node_mut(grav) {
        n.entropy = 0.25;
    }
    if let Some(n) = workspace.graph.get_node_mut(miss) {
        n.entropy = 0.30;
    }
    if let Some(n) = workspace.graph.get_node_mut(part) {
        n.entropy = 0.45;
        n.velocity = 1.8;
    }
    if let Some(n) = workspace.graph.get_node_mut(night) {
        n.entropy = 0.15;
        n.gravity = 2.5;
    }
    if let Some(n) = workspace.graph.get_node_mut(core) {
        n.gravity = 3.5;
    }
    if let Some(n) = workspace.graph.get_node_mut(gpu) {
        n.gravity = 2.0;
    }

    // ── edges ────────────────────────────────────────────────
    // core → clusters
    workspace
        .graph
        .connect(core, graph, EdgeType::Connection, 0.9)?;
    workspace
        .graph
        .connect(core, vault, EdgeType::Connection, 0.8)?;
    workspace
        .graph
        .connect(core, store, EdgeType::Connection, 0.85)?;
    workspace.graph.connect(core, grav, EdgeType::Causal, 0.7)?;
    workspace.graph.connect(core, sil, EdgeType::Causal, 0.7)?;
    workspace.graph.connect(core, gpu, EdgeType::Causal, 0.75)?;
    workspace
        .graph
        .connect(core, dev, EdgeType::Resonance, 0.6)?;

    // physics cluster
    workspace
        .graph
        .connect(grav, entr, EdgeType::Connection, 0.8)?;
    workspace
        .graph
        .connect(grav, cont, EdgeType::Connection, 0.75)?;
    workspace
        .graph
        .connect(entr, cont, EdgeType::Resonance, 0.5)?;

    // analysis cluster
    workspace
        .graph
        .connect(sil, miss, EdgeType::Connection, 0.9)?;
    workspace
        .graph
        .connect(sil, impl_, EdgeType::Connection, 0.85)?;
    workspace
        .graph
        .connect(miss, impl_, EdgeType::Resonance, 0.4)?;

    // rendering cluster
    workspace
        .graph
        .connect(gpu, cam, EdgeType::Connection, 0.9)?;
    workspace
        .graph
        .connect(gpu, part, EdgeType::Connection, 0.85)?;
    workspace
        .graph
        .connect(cam, part, EdgeType::Resonance, 0.3)?;

    // graph ↔ physics
    workspace
        .graph
        .connect(graph, grav, EdgeType::Causal, 0.6)?;
    workspace.graph.connect(graph, sil, EdgeType::Causal, 0.6)?;

    // person → world
    workspace.graph.connect(dev, night, EdgeType::Causal, 0.9)?;
    workspace
        .graph
        .connect(night, core, EdgeType::Resonance, 0.7)?;

    // ghost — lonely edge to analysis
    workspace
        .graph
        .connect(ghost, sil, EdgeType::Temporal, 0.2)?;

    // focus + journal
    workspace.record_focus(core, 600.0, FocusDepth::DeepWork)?;
    workspace.record_focus(gpu, 300.0, FocusDepth::Edit)?;
    workspace.record_focus(graph, 240.0, FocusDepth::Read)?;
    workspace.record_focus(dev, 120.0, FocusDepth::Glance)?;
    workspace.add_journal_entry(
        "SilentNode Phase 3 seeded — GPU renderer, 16 nodes, cognitive graph live.",
        Some("spring".into()),
    );

    // spread via gravity simulation
    let gravity = GravityEngine::new();
    for _ in 0..30 {
        workspace.step_gravity(&gravity, 0.8);
    }
    workspace.tick_entropy(&EntropyEngine::new());
    Ok(())
}

fn parse_focus_depth(input: &str) -> Result<FocusDepth, Box<dyn std::error::Error>> {
    match input {
        "glance" => Ok(FocusDepth::Glance),
        "read" => Ok(FocusDepth::Read),
        "edit" | "think" => Ok(FocusDepth::Edit),
        "deep_work" | "deep-work" => Ok(FocusDepth::DeepWork),
        _ => Err("focus depth must be one of: glance, read, edit, think, deep_work".into()),
    }
}

fn export_encrypted_snapshot(
    snapshot: &WorkspaceSnapshot,
    password: &str,
    output: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)?;
    }
    let vault_root = output
        .parent()
        .map(|parent| parent.join("vault"))
        .unwrap_or_else(|| PathBuf::from("data/vault"));
    let vault = Vault::open(password, &vault_root)?;
    let payload = serde_json::to_vec_pretty(snapshot)?;
    let encrypted = vault.encrypt(&payload)?;
    fs::write(output, encrypted)?;
    Ok(())
}

fn import_encrypted_snapshot(
    password: &str,
    input: &Path,
) -> Result<WorkspaceSnapshot, Box<dyn std::error::Error>> {
    let vault_root = input
        .parent()
        .map(|parent| parent.join("vault"))
        .unwrap_or_else(|| PathBuf::from("data/vault"));
    let vault = Vault::open(password, &vault_root)?;
    let encrypted = fs::read(input)?;
    let payload = vault.decrypt(&encrypted)?;
    Ok(serde_json::from_slice(&payload)?)
}

fn print_status(workspace: &SilentNodeWorkspace, config: &Paths) {
    let stats = workspace.graph.stats();
    println!("SilentNode status");
    println!("Backend: {}", config.backend.as_str());
    println!("SQLite snapshot: {}", config.sqlite_path.display());
    println!("Surreal store: {}", config.surreal_path.display());
    println!("HTML universe: {}", config.html_path.display());
    println!("Vault dir: {}", config.vault_dir.display());
    println!("Nodes: {}", stats.node_count);
    println!("Edges: {}", stats.edge_count);
    println!("Ghosts: {}", stats.ghost_count);
    println!("Fossils: {}", stats.fossil_count);
    println!("Void nodes: {}", stats.void_count);
    println!("Focus events: {}", workspace.focus.events().len());
    println!("Journal entries: {}", workspace.journal.entries().len());
}

fn print_nodes(workspace: &SilentNodeWorkspace, query: Option<&str>) {
    let mut nodes = if let Some(query) = query {
        workspace.search_nodes(query)
    } else {
        workspace.graph.nodes().collect::<Vec<_>>()
    };
    nodes.sort_by(|left, right| right.gravity.total_cmp(&left.gravity));

    if nodes.is_empty() {
        println!("No nodes found.");
        return;
    }

    for node in nodes {
        println!(
            "{}  {:?}  entropy={:.2} gravity={:.2} access={}  {}",
            node.id, node.node_type, node.entropy, node.gravity, node.access_count, node.content
        );
    }
}

fn print_node_details(
    workspace: &SilentNodeWorkspace,
    node_id: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let node_id = Uuid::parse_str(node_id)?;
    let node = workspace.get_node(node_id).ok_or("node not found")?;
    println!("Node {}", node.id);
    println!("Type: {:?}", node.node_type);
    println!("Content: {}", node.content);
    println!(
        "Entropy: {:.3}  Gravity: {:.3}  Velocity: {:.3}",
        node.entropy, node.gravity, node.velocity
    );
    println!(
        "Access count: {}  Ghost: {}  Fossil: {}  Void: {}",
        node.access_count, node.is_ghost, node.is_fossil, node.is_void
    );
    println!(
        "Position: ({:.2}, {:.2}, {:.2})",
        node.position.x, node.position.y, node.position.z
    );
    let neighbors = workspace.graph.neighbors(node_id)?;
    let incoming = workspace.graph.incoming_edges(node_id)?;
    let outgoing = workspace.graph.outgoing_edges(node_id)?;
    println!("Neighbors: {}", neighbors.len());
    for neighbor in neighbors {
        println!("  {}  {:?}", neighbor.content, neighbor.node_type);
    }
    println!("Incoming edges: {}", incoming.len());
    println!("Outgoing edges: {}", outgoing.len());
    let recent_focus = workspace.focus_history_for_node(node_id, 72);
    if !recent_focus.is_empty() {
        println!("Recent focus events: {}", recent_focus.len());
    }
    let linked_journal = workspace.journal_for_node(node_id, 30);
    if !linked_journal.is_empty() {
        println!("Linked journal entries: {}", linked_journal.len());
    }
    Ok(())
}

fn print_linked_nodes(workspace: &SilentNodeWorkspace, linked_nodes: &[Uuid]) {
    if linked_nodes.is_empty() {
        println!("  linked nodes: none");
        return;
    }

    let names = linked_nodes
        .iter()
        .filter_map(|node_id| workspace.get_node(*node_id))
        .map(|node| node.content.as_str())
        .collect::<Vec<_>>();
    println!("  linked nodes: {}", names.join(", "));
}

fn parse_edge_type(input: &str) -> Result<EdgeType, Box<dyn std::error::Error>> {
    match input {
        "connection" => Ok(EdgeType::Connection),
        "resonance" => Ok(EdgeType::Resonance),
        "temporal" => Ok(EdgeType::Temporal),
        "causal" => Ok(EdgeType::Causal),
        _ => Err("edge type must be one of: connection, resonance, temporal, causal".into()),
    }
}

fn python_executable() -> &'static str {
    if Path::new(".venv/bin/python").exists() {
        ".venv/bin/python"
    } else {
        "python3"
    }
}

fn print_help() {
    println!("SilentNode CLI");
    println!("  cargo run -- status");
    println!("  cargo run -- list-nodes [query]");
    println!("  cargo run -- show-node <node-id>");
    println!("  cargo run -- seed");
    println!("  cargo run -- migrate-to-surreal");
    println!("  cargo run -- surreal-health");
    println!("  cargo run -- render");
    println!("  cargo run -- recompute");
    println!("  cargo run -- simulate [steps] [delta-time]");
    println!("  cargo run -- add-thought <text>");
    println!("  cargo run -- connect <source-id> <target-id> [weight] [edge-type]");
    println!("  cargo run -- disconnect <source-id> <target-id>");
    println!("  cargo run -- delete-node <node-id>");
    println!("  cargo run -- revive-node <node-id>");
    println!("  cargo run -- journal <text>");
    println!("  cargo run -- journal-season <season> <text>");
    println!("  cargo run -- journal-search <query>");
    println!("  cargo run -- journal-list [days]");
    println!("  cargo run -- journal-range <from-rfc3339> <to-rfc3339>");
    println!("  cargo run -- focus <node-id> <seconds> <glance|read|edit|deep_work>");
    println!("  cargo run -- trail [hours]");
    println!("  cargo run -- node-trail <node-id> [hours]");
    println!("  cargo run -- node-journal <node-id> [days]");
    println!("  cargo run -- heatmap [days]");
    println!("  cargo run -- export-encrypted <password> [output-path]");
    println!("  cargo run -- import-encrypted <password> [input-path]");
    println!("  cargo run -- rotate-vault-password <current-password> <new-password>");
    println!("  cargo run -- launch [width] [height]");
    println!("  cargo run -- weather");
    println!("  cargo run -- weight");
    println!("  cargo run -- souls");
    println!("  cargo run -- tectonics");
    println!("  cargo run -- temporal-status");
    println!("  cargo run -- snapshot-all");
    println!("  cargo run -- record-change <node-id>");
    println!("  cargo run -- archaeology <node-id>");
    println!("  cargo run -- reconstruct-day <YYYY-MM-DD>");
    println!("  cargo run -- compare-days <YYYY-MM-DD> <YYYY-MM-DD>");
    println!("  cargo run -- fossil-check <node-id>");
    println!("  cargo run -- fossilize <node-id>");
    println!("  cargo run -- excavate <node-id>");
    println!("  cargo run -- lore");
    println!();
    println!("Phase 6 — Pattern Recognition:");
    println!("  cargo run -- season");
    println!("  cargo run -- oracle");
    println!("  cargo run -- rituals");
    println!("  cargo run -- mirror [days]");
    println!("  cargo run -- heatmap7 [days]");
    println!("  cargo run -- contracts");
    println!("  cargo run -- resonances");
    println!();
    println!("Phase 7 — Social Graph:");
    println!("  cargo run -- civilizations");
    println!("  cargo run -- crystallize");
    println!("  cargo run -- shadows");
    println!("  cargo run -- void-node <node-id>");
    println!("  cargo run -- unvoid-node <node-id>");
    println!("  cargo run -- void-check <node-id>");
    println!();
    println!("Multi-device Sync:");
    println!("  cargo run -- sync-serve [port]          # default port 7979");
    println!("  cargo run -- sync-pull [host:port]      # pull + merge from peer");
    println!("  cargo run -- sync-push [host:port]      # push local + merge from peer");
    println!();
    println!("Phase 9 — Intelligence + Export + API:");
    println!("  cargo run -- suggestions [limit]        # focus suggestions ranked by neglect");
    println!("  cargo run -- related <node-id> [limit]  # TF-IDF similar nodes");
    println!("  cargo run -- clusters [k]               # content cluster k groups");
    println!("  cargo run -- export-dot [path]          # GraphViz DOT export");
    println!("  cargo run -- export-csv [path]          # nodes CSV export");
    println!("  cargo run -- export-edges-csv [path]    # edges CSV export");
    println!("  cargo run -- export-markdown [path]     # Markdown export");
    println!("  cargo run -- import-text <file>         # one thought per line");
    println!("  cargo run -- api [port]                 # REST API server (default 3030)");
    println!();
    println!("Phase 10 — Identity & Narrative:");
    println!("  cargo run -- signature                  # Living Signature summary");
    println!("  cargo run -- shadow-projects            # Shadow Projects (roads not taken)");
    println!("  cargo run -- lore-chronicle             # Personal Lore Chronicle narrative");
    println!("  cargo run -- heroes-journey             # Hero's Journey mapping");
    println!("  cargo run --features python -- deep-signature  # ASCII art + full report");
    println!("  cargo run --features python -- signature-svg   # export SVG to file");
    println!();
    println!("TUI Dashboard:");
    println!("  cargo run -- tui                        # rich terminal UI (8 tabs)");
    println!("    Tab/←→ switch tab  ↑↓ nav  Enter select  / search  R refresh  W save  Q quit");
    println!("    Tabs: Overview / Nodes / Journal / Intelligence / Oracle / Analytics / Dream / Identity");
    println!();
    println!("Phase 11 — Web Dashboard:");
    println!("  cargo run -- dashboard [path]           # export standalone HTML");
    println!("  cargo run -- api [port]  →  GET /dashboard  # live polling dashboard");
    println!();
    println!("Phase 12 — Advanced Graph Analytics:");
    println!("  cargo run -- analytics                  # full health report");
    println!("  cargo run -- health                     # one-line health summary");
    println!("  cargo run -- pagerank [limit]           # PageRank influence scores");
    println!("  cargo run -- bridges                    # bridge (critical) edges");
    println!("  cargo run -- api [port]  →  GET /analytics          # health report");
    println!("  cargo run -- api [port]  →  GET /analytics/pagerank # PageRank list");
    println!("  cargo run -- api [port]  →  GET /analytics/bridges  # bridge list");
    println!();
    println!("Phase 13 — Dream Engine + Thought Synthesis:");
    println!("  cargo run -- dream                      # dream proposals");
    println!("  cargo run -- synthesize <text>          # topic narrative");
    println!("  cargo run -- knowledge-gaps <topic>     # missing knowledge terms");
    println!("  cargo run -- thought-chain <id1> <id2>  # trace causal path");
    println!("  cargo run -- api [port]  →  GET  /dream/proposals   # proposals");
    println!("  cargo run -- api [port]  →  POST /synthesize        # narrative");
    println!("  cargo run -- api [port]  →  GET  /synthesis/gaps    # gaps");
    println!("  cargo run -- api [port]  →  POST /synthesis/chain   # path");
    println!();
    println!("Phase 6 — Python Pattern Intelligence (requires --features python):");
    println!("  cargo run --features python -- deep-rituals           # DBSCAN ritual detection");
    println!(
        "  cargo run --features python -- deep-season            # statistical season analysis"
    );
    println!(
        "  cargo run --features python -- deep-oracle            # predictive behavioral signals"
    );
    println!("  cargo run --features python -- deep-mirror            # cognitive self-portrait");
    println!("  cargo run --features python -- deep-heatmap [days]    # exponential decay heatmap");
    println!("  cargo run --features python -- deep-contracts         # silent contract detection");
    println!("  cargo run --features python -- deep-resonances [thr]  # TF-IDF semantic resonance");
    println!();
    println!("Phase 7 — Python Social Graph (requires --features python):");
    println!(
        "  cargo run --features python -- louvain [resolution]   # Louvain civilization detection"
    );
    println!("  cargo run --features python -- deep-shadows           # digital shadow detection");
    println!();
    println!("  env: SILENTNODE_BACKEND=sqlite|surreal");
}
