/// Phase 8.4 — Process Sovereignty
///
/// SilentNode knows what is running. No process runs invisibly. No process
/// exists outside the universe's awareness.
///
/// Every running process is represented as a ProcessNode linked to its
/// parent project or idea in the cognitive graph.
///
/// Vision.md:
///   - real-time status: running, idle, completed, failed
///   - resource consumption: CPU, memory, duration
///   - temporal record: when it ran, how long, what it produced
///   - linked to codebase node / architecture idea / test runner / build
use crate::domain::ProcessRecord;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ── RunningProcess ────────────────────────────────────────────────────────────

/// A live snapshot of a running process.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunningProcess {
    pub pid: i64,
    pub name: String,
    pub command: String,
    pub cpu_usage: f32,
    pub memory_mb: f32,
    /// How long this process has been running (seconds).
    pub uptime_seconds: f32,
    pub status: ProcessStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProcessStatus {
    Running,
    Sleeping,
    Idle,
    Stopped,
    Zombie,
    Unknown,
}

impl ProcessStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Running => "running",
            Self::Sleeping => "sleeping",
            Self::Idle => "idle",
            Self::Stopped => "stopped",
            Self::Zombie => "zombie",
            Self::Unknown => "unknown",
        }
    }
}

impl RunningProcess {
    /// Convert to a ProcessRecord for storage in the graph.
    pub fn to_record(&self, linked_node: Uuid) -> ProcessRecord {
        let mut rec = ProcessRecord::new(self.pid, &self.name, linked_node);
        rec.cpu_usage = self.cpu_usage;
        rec.memory_mb = self.memory_mb;
        rec
    }

    pub fn summary(&self) -> String {
        format!(
            "PID {:6} | {:20} | CPU {:5.1}% | MEM {:6.1}MB | {} | {}",
            self.pid,
            truncate(&self.name, 20),
            self.cpu_usage,
            self.memory_mb,
            self.status.as_str(),
            truncate(&self.command, 30),
        )
    }
}

fn truncate(s: &str, max: usize) -> &str {
    let end = s.char_indices().nth(max).map(|(i, _)| i).unwrap_or(s.len());
    &s[..end]
}

// ── ProcessSovereignty ────────────────────────────────────────────────────────

/// Central process monitoring and governance system.
#[derive(Debug, Clone)]
pub struct ProcessSovereignty {
    /// Filter: only report processes whose name contains one of these strings.
    /// Empty = report all processes.
    pub watch_patterns: Vec<String>,
    /// Processes that were last seen linked to a specific cognitive node.
    linked: std::collections::HashMap<i64, Uuid>, // pid → node_id
}

impl Default for ProcessSovereignty {
    fn default() -> Self {
        Self::new()
    }
}

impl ProcessSovereignty {
    pub fn new() -> Self {
        Self {
            watch_patterns: Vec::new(),
            linked: std::collections::HashMap::new(),
        }
    }

    pub fn watch(mut self, pattern: impl Into<String>) -> Self {
        self.watch_patterns.push(pattern.into());
        self
    }

    /// Link a PID to a cognitive graph node.
    pub fn link_to_node(&mut self, pid: i64, node_id: Uuid) {
        self.linked.insert(pid, node_id);
    }

    /// Return the node_id linked to this PID (if any).
    pub fn linked_node(&self, pid: i64) -> Option<Uuid> {
        self.linked.get(&pid).copied()
    }

    /// Scan currently running processes.
    /// When built with `--features process` this uses sysinfo.
    /// Otherwise falls back to a lightweight /proc scanner on Linux,
    /// or returns an empty list on other platforms.
    pub fn scan(&self) -> Vec<RunningProcess> {
        #[cfg(feature = "process")]
        {
            return self.scan_with_sysinfo();
        }
        #[cfg(not(feature = "process"))]
        {
            return self.scan_fallback();
        }
    }

    #[cfg(feature = "process")]
    fn scan_with_sysinfo(&self) -> Vec<RunningProcess> {
        use sysinfo::{ProcessStatus as SysStatus, System};
        let mut sys = System::new_all();
        sys.refresh_all();

        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);

        sys.processes()
            .values()
            .filter(|p| self.matches_filter(p.name().to_string_lossy().as_ref()))
            .map(|p| {
                let t = p.start_time();
                let started_ms = t * 1000;
                let uptime = if now_ms > started_ms {
                    (now_ms - started_ms) as f32 / 1000.0
                } else {
                    0.0
                };

                let status = match p.status() {
                    SysStatus::Run => ProcessStatus::Running,
                    SysStatus::Sleep => ProcessStatus::Sleeping,
                    SysStatus::Idle => ProcessStatus::Idle,
                    SysStatus::Stop => ProcessStatus::Stopped,
                    SysStatus::Zombie => ProcessStatus::Zombie,
                    _ => ProcessStatus::Unknown,
                };

                RunningProcess {
                    pid: p.pid().as_u32() as i64,
                    name: p.name().to_string_lossy().to_string(),
                    command: p
                        .cmd()
                        .iter()
                        .map(|s| s.to_string_lossy().to_string())
                        .collect::<Vec<_>>()
                        .join(" "),
                    cpu_usage: p.cpu_usage(),
                    memory_mb: p.memory() as f32 / 1_048_576.0,
                    uptime_seconds: uptime,
                    status,
                }
            })
            .collect()
    }

    fn scan_fallback(&self) -> Vec<RunningProcess> {
        // Lightweight Linux /proc reader — zero dependencies
        #[cfg(target_os = "linux")]
        {
            return self.scan_proc_fs();
        }
        // Non-Linux without sysinfo: return empty
        #[allow(unreachable_code)]
        Vec::new()
    }

    #[cfg(target_os = "linux")]
    fn scan_proc_fs(&self) -> Vec<RunningProcess> {
        let mut processes = Vec::new();
        let proc_dir = std::path::Path::new("/proc");
        if let Ok(entries) = std::fs::read_dir(proc_dir) {
            for entry in entries.flatten() {
                let name = entry.file_name();
                let pid_str = name.to_string_lossy();
                if let Ok(pid) = pid_str.parse::<i64>() {
                    if let Some(proc) = self.read_proc_entry(pid) {
                        if self.matches_filter(&proc.name) {
                            processes.push(proc);
                        }
                    }
                }
            }
        }
        processes.sort_by(|a, b| {
            b.cpu_usage
                .partial_cmp(&a.cpu_usage)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        processes
    }

    #[cfg(target_os = "linux")]
    fn read_proc_entry(&self, pid: i64) -> Option<RunningProcess> {
        let stat_path = format!("/proc/{}/stat", pid);
        let stat = std::fs::read_to_string(&stat_path).ok()?;
        let fields: Vec<&str> = stat.split_whitespace().collect();
        if fields.len() < 14 {
            return None;
        }

        // Name is fields[1] with parens stripped
        let name = fields[1]
            .trim_start_matches('(')
            .trim_end_matches(')')
            .to_string();

        let status = match fields.get(2).copied().unwrap_or("?") {
            "R" => ProcessStatus::Running,
            "S" => ProcessStatus::Sleeping,
            "D" => ProcessStatus::Idle,
            "T" => ProcessStatus::Stopped,
            "Z" => ProcessStatus::Zombie,
            _ => ProcessStatus::Unknown,
        };

        // Read command from cmdline
        let cmdline = std::fs::read_to_string(format!("/proc/{}/cmdline", pid))
            .unwrap_or_default()
            .replace('\0', " ")
            .trim()
            .chars()
            .take(80)
            .collect::<String>();

        // Memory from status file
        let memory_mb = self.read_proc_status_mem(pid);

        Some(RunningProcess {
            pid,
            name,
            command: cmdline,
            cpu_usage: 0.0, // /proc/stat requires two reads; skip for simplicity
            memory_mb,
            uptime_seconds: 0.0,
            status,
        })
    }

    #[cfg(target_os = "linux")]
    fn read_proc_status_mem(&self, pid: i64) -> f32 {
        let status = std::fs::read_to_string(format!("/proc/{}/status", pid)).unwrap_or_default();
        for line in status.lines() {
            if line.starts_with("VmRSS:") {
                let kb: f32 = line
                    .split_whitespace()
                    .nth(1)
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(0.0);
                return kb / 1024.0;
            }
        }
        0.0
    }

    fn matches_filter(&self, name: &str) -> bool {
        if self.watch_patterns.is_empty() {
            return true;
        }
        self.watch_patterns
            .iter()
            .any(|p| name.contains(p.as_str()))
    }

    /// Generate ProcessRecord entries for all scanned processes that have a
    /// linked cognitive node.
    pub fn to_linked_records(&self) -> Vec<(RunningProcess, Uuid)> {
        self.scan()
            .into_iter()
            .filter_map(|p| self.linked_node(p.pid).map(|node_id| (p, node_id)))
            .collect()
    }

    pub fn print_status(&self) {
        let procs = self.scan();
        println!("══ Process Sovereignty ═════════════════════════════════════");
        println!("  Visible processes: {}", procs.len());
        println!("  Linked to nodes:   {}", self.linked.len());
        let top: Vec<&RunningProcess> = {
            let mut v: Vec<&RunningProcess> = procs.iter().collect();
            v.sort_by(|a, b| {
                b.memory_mb
                    .partial_cmp(&a.memory_mb)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
            v.truncate(10);
            v
        };
        for p in &top {
            println!("  {}", p.summary());
        }
    }
}

// ── ProcessActivityReport ─────────────────────────────────────────────────────

/// Summary of process activity for a time window.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessActivityReport {
    pub generated_at: DateTime<Utc>,
    pub total_tracked: usize,
    pub linked_count: usize,
    pub top_cpu: Vec<(String, f32)>,
    pub top_memory: Vec<(String, f32)>,
}

impl ProcessActivityReport {
    pub fn from_scan(procs: &[RunningProcess], linked_count: usize) -> Self {
        let mut by_cpu = procs.to_vec();
        by_cpu.sort_by(|a, b| {
            b.cpu_usage
                .partial_cmp(&a.cpu_usage)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        let mut by_mem = procs.to_vec();
        by_mem.sort_by(|a, b| {
            b.memory_mb
                .partial_cmp(&a.memory_mb)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        Self {
            generated_at: Utc::now(),
            total_tracked: procs.len(),
            linked_count,
            top_cpu: by_cpu
                .iter()
                .take(5)
                .map(|p| (p.name.clone(), p.cpu_usage))
                .collect(),
            top_memory: by_mem
                .iter()
                .take(5)
                .map(|p| (p.name.clone(), p.memory_mb))
                .collect(),
        }
    }
}
