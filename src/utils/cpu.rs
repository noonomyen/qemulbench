use std::collections::BTreeMap;
use std::fs::File;
use std::io::{self, BufRead, BufReader};

#[derive(Debug, Clone)]
pub struct CpuCluster {
    pub part_id: String,
    pub cores: Vec<usize>,
}

pub fn detect_arm_clusters() -> Vec<CpuCluster> {
    let file = match File::open("/proc/cpuinfo") {
        Ok(f) => f,
        Err(_) => return Vec::new(),
    };

    let reader = BufReader::new(file);
    let mut current_proc: Option<usize> = None;
    let mut core_parts: BTreeMap<String, Vec<usize>> = BTreeMap::new();

    for line in reader.lines().map_while(Result::ok) {
        let trimmed = line.trim();
        if trimmed.starts_with("processor") {
            if let Some(val) = trimmed.split(':').nth(1) {
                if let Ok(id) = val.trim().parse::<usize>() {
                    current_proc = Some(id);
                }
            }
        } else if trimmed.starts_with("CPU part") {
            if let Some(val) = trimmed.split(':').nth(1) {
                let part_raw = val.trim();
                let part_norm = if part_raw.starts_with("0x") || part_raw.starts_with("0X") {
                    part_raw.to_lowercase()
                } else if let Ok(n) = part_raw.parse::<usize>() {
                    format!("0x{:03x}", n)
                } else {
                    part_raw.to_lowercase()
                };

                if let Some(proc_id) = current_proc.take() {
                    core_parts.entry(part_norm).or_default().push(proc_id);
                }
            }
        }
    }

    core_parts
        .into_iter()
        .map(|(part_id, mut cores)| {
            cores.sort_unstable();
            cores.dedup();
            CpuCluster { part_id, cores }
        })
        .collect()
}

pub fn list_arm_clusters() {
    let clusters = detect_arm_clusters();
    if clusters.is_empty() {
        println!("No ARM CPU clusters detected or not running on ARM Linux host.");
        return;
    }
    println!("Detected ARM CPU clusters:");
    for (i, cluster) in clusters.iter().enumerate() {
        let cores_str: Vec<String> = cluster.cores.iter().map(|c| c.to_string()).collect();
        println!(
            "  [{}] CPU Part: {} - Cores: {} ({} cores)",
            i + 1,
            cluster.part_id,
            cores_str.join(","),
            cluster.cores.len()
        );
    }
}

pub fn select_arm_cluster(
    clusters: &[CpuCluster],
    no_cpu_topo: bool,
    preselected_index: Option<usize>,
) -> io::Result<Option<CpuCluster>> {
    if clusters.is_empty() {
        return Ok(None);
    }

    if no_cpu_topo {
        eprintln!("[INFO] CPU topology pinning disabled (--no-cpu-topo).");
        return Ok(None);
    }

    if let Some(idx) = preselected_index {
        if idx == 0 {
            eprintln!("[INFO] CPU topology pinning disabled via --cpu-part 0.");
            return Ok(None);
        }
        if idx >= 1 && idx <= clusters.len() {
            let chosen = &clusters[idx - 1];
            eprintln!("[INFO] Selected CPU Part {}: {} - Cores {:?}", idx, chosen.part_id, chosen.cores);
            return Ok(Some(chosen.clone()));
        } else {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("Invalid --cpu-part index {} (valid range: 0..{})", idx, clusters.len()),
            ));
        }
    }

    let default_cluster = &clusters[0];
    if clusters.len() > 1 {
        eprintln!(
            "[INFO] Multiple ARM CPU core types detected. Defaulting to CPU Part 1: {} - Cores {:?}",
            default_cluster.part_id, default_cluster.cores
        );
    }
    Ok(Some(default_cluster.clone()))
}
