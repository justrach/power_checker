// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use serde::{Deserialize, Serialize};
use std::time::SystemTime;
use log::info;

#[derive(Debug, Serialize, Deserialize, Clone)]
struct CPUCore {
    id: u32,
    frequency: f64,
    usage: f64,
    temperature: f64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct GPU {
    id: u32,
    power: f64,
    frequency: f64,
    usage: f64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct SystemMetrics {
    timestamp: u64,
    cpu_cores: Vec<CPUCore>,
    total_cpu_power: f64,
    // Overall GPU stats
    total_gpu_power: f64,
    total_gpu_usage: f64,
    // Individual GPU stats
    gpus: Vec<GPU>,
    memory_total: u64,
    memory_used: u64,
    carbon_intensity: f64,
}

fn parse_cpu_core(text: &str, core_id: u32) -> Option<CPUCore> {
    let mut frequency = 0.0;
    let mut usage = 0.0;

    for line in text.lines() {
        if line.starts_with(&format!("CPU {} frequency:", core_id)) {
            if let Some(freq_str) = line.split(':').nth(1) {
                frequency = freq_str.trim().split_whitespace().next()
                    .and_then(|s| s.parse::<f64>().ok())
                    .unwrap_or(0.0);
            }
        } else if line.starts_with(&format!("CPU {} active residency:", core_id)) {
            if let Some(usage_str) = line.split(':').nth(1) {
                usage = usage_str.trim().split('%').next()
                    .and_then(|s| s.trim().parse::<f64>().ok())
                    .unwrap_or(0.0);
            }
        }
    }

    Some(CPUCore {
        id: core_id,
        frequency,
        usage,
        temperature: 0.0, // Temperature per core not available in powermetrics
    })
}

fn parse_gpu_metrics(text: &str) -> (Vec<GPU>, f64, f64) {
    let mut power = 0.0;
    let mut current_frequency = 0.0;
    let mut max_frequency: f64 = 0.0;
    let mut active_residency = 0.0;

    // Parse the GPU metrics section
    for line in text.lines() {
        if line.starts_with("GPU Power:") {
            if let Some(power_str) = line.split(':').nth(1) {
                power = power_str.trim().split_whitespace().next()
                    .and_then(|s| s.parse::<f64>().ok())
                    .map(|p| p / 1000.0) // Convert mW to W
                    .unwrap_or(0.0);
            }
        } else if line.starts_with("GPU HW active frequency:") {
            if let Some(freq_str) = line.split(':').nth(1) {
                current_frequency = freq_str.trim().split_whitespace().next()
                    .and_then(|s| s.parse::<f64>().ok())
                    .unwrap_or(0.0);
            }
        } else if line.contains("GPU HW active residency:") {
            // Find the maximum frequency in the residency line
            if let Some(residency_str) = line.split(':').nth(1) {
                for part in residency_str.split(')') {
                    if let Some(freq_str) = part.split("MHz:").next() {
                        if let Some(freq) = freq_str.trim().split_whitespace().next() {
                            if let Ok(freq_val) = freq.parse::<f64>() {
                                max_frequency = max_frequency.max(freq_val);
                            }
                        }
                    }
                }
                // Get the active residency percentage
                active_residency = 100.0 - (line.contains("idle residency:")
                    .then(|| {
                        line.split("idle residency:").nth(1)
                            .and_then(|s| s.trim().split('%').next())
                            .and_then(|s| s.trim().parse::<f64>().ok())
                            .unwrap_or(0.0)
                    })
                    .unwrap_or(0.0));
            }
        }
    }

    // Calculate overall GPU usage as a combination of frequency and activity
    let frequency_utilization = if max_frequency > 0.0 {
        current_frequency / max_frequency * 100.0
    } else {
        0.0
    };
    
    // Combine frequency utilization and active residency for overall usage
    let usage = (frequency_utilization * active_residency / 100.0).min(100.0);

    // Create a single GPU instance since macOS only reports one GPU
    let gpu = GPU {
        id: 0,
        power,
        frequency: current_frequency,
        usage,
    };

    (vec![gpu], power, usage)
}

fn get_memory_info() -> (u64, u64) {
    // Get total physical memory using sysctl
    let total = if let Ok(output) = std::process::Command::new("sysctl")
        .args(["-n", "hw.memsize"])
        .output() {
        String::from_utf8_lossy(&output.stdout)
            .trim()
            .parse::<u64>()
            .unwrap_or(0)
    } else {
        0
    };

    // Get memory usage from vm_stat
    let used = if let Ok(output) = std::process::Command::new("vm_stat")
        .output() {
        let text = String::from_utf8_lossy(&output.stdout);
        let mut app_memory = 0;
        let mut wired = 0;

        // Page size is 16384 bytes on Apple Silicon Macs
        const PAGE_SIZE: u64 = 16384;

        for line in text.lines() {
            if line.contains("Pages active:") || 
               line.contains("Pages anonymous:") ||
               line.contains("Pages occupied by compressor:") {
                if let Some(value) = line.split(':').nth(1) {
                    if let Ok(pages) = value.trim().replace('.', "").parse::<u64>() {
                        app_memory += pages * PAGE_SIZE;
                    }
                }
            } else if line.contains("Pages wired down:") {
                if let Some(value) = line.split(':').nth(1) {
                    if let Ok(pages) = value.trim().replace('.', "").parse::<u64>() {
                        wired += pages * PAGE_SIZE;
                    }
                }
            }
        }
        
        // Total used memory = App Memory + Wired Memory
        app_memory + wired
    } else {
        0
    };

    (total, used)
}

#[tauri::command]
async fn measure_metrics() -> Result<SystemMetrics, String> {
    info!("Measuring system metrics");
    
    // Always use sudo with -S flag to read password from stdin if needed
    let output = std::process::Command::new("sudo")
        .args(["powermetrics", "--samplers", "cpu_power,gpu_power", "-i", "1000", "-n", "1"])
        .output()
        .map_err(|e| {
            let msg = format!("Failed to execute powermetrics: {}", e);
            info!("{}", msg);
            msg
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let msg = if stderr.contains("must be invoked as the superuser") {
            "Please run 'sudo powermetrics' in terminal first to grant permissions".to_string()
        } else if !stderr.is_empty() {
            format!("powermetrics failed: {}", stderr)
        } else if !stdout.is_empty() {
            format!("powermetrics failed: {}", stdout)
        } else {
            "powermetrics failed with no output".to_string()
        };
        info!("{}", msg);
        return Err(msg);
    }

    let text = String::from_utf8_lossy(&output.stdout);
    if text.trim().is_empty() {
        return Err("powermetrics produced no output".to_string());
    }
    info!("Got powermetrics output");
    
    // Get memory information first since it doesn't require sudo
    let (memory_total, memory_used) = get_memory_info();
    
    // Parse CPU information for each core
    let mut cpu_cores = Vec::new();
    let mut total_cpu_power = 0.0;

    for core_id in 0..28 { // M3 Max has 28 cores
        if let Some(core) = parse_cpu_core(&text, core_id) {
            cpu_cores.push(core);
        }
    }

    // Find CPU power
    for line in text.lines() {
        if line.starts_with("CPU Power:") {
            if let Some(power_str) = line.split(':').nth(1) {
                total_cpu_power = power_str.trim().split_whitespace().next()
                    .and_then(|s| s.parse::<f64>().ok())
                    .map(|p| p / 1000.0) // Convert mW to W
                    .unwrap_or(0.0);
            }
        }
    }

    // Get GPU metrics
    let (gpus, total_gpu_power, total_gpu_usage) = parse_gpu_metrics(&text);

    let metrics = SystemMetrics {
        timestamp: SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map_err(|e| format!("Failed to get timestamp: {}", e))?.as_secs(),
        cpu_cores,
        total_cpu_power,
        total_gpu_power,
        total_gpu_usage,
        gpus,
        memory_total,
        memory_used,
        carbon_intensity: 100.0, // TODO: Integrate with real carbon intensity API
    };

    info!("System metrics measurement complete");
    Ok(metrics)
}

fn main() {
    env_logger::init();
    info!("Starting Power Monitor application");

    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![measure_metrics])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

