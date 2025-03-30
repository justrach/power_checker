import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import "./App.css";
import { Line } from 'react-chartjs-2';
import {
  Chart as ChartJS,
  CategoryScale,
  LinearScale,
  PointElement,
  LineElement,
  Title,
  Tooltip,
  Legend,
  ChartOptions,
} from 'chart.js';

ChartJS.register(
  CategoryScale,
  LinearScale,
  PointElement,
  LineElement,
  Title,
  Tooltip,
  Legend
);

interface CPUCore {
  id: number;
  frequency: number;
  usage: number;
  temperature: number;
}

interface GPU {
  id: number;
  power: number;
  frequency: number;
  usage: number;
}

interface SystemMetrics {
  timestamp: number;
  cpu_cores: CPUCore[];
  total_cpu_power: number;
  total_gpu_power: number;
  total_gpu_usage: number;
  gpus: GPU[];
  memory_total: number;
  memory_used: number;
  carbon_intensity: number;
}

function formatBytes(bytes: number): string {
  const units = ['B', 'KB', 'MB', 'GB'];
  let value = bytes;
  let unitIndex = 0;
  
  while (value >= 1024 && unitIndex < units.length - 1) {
    value /= 1024;
    unitIndex++;
  }
  
  return `${value.toFixed(2)} ${units[unitIndex]}`;
}

function formatPower(watts: number): string {
  if (watts < 1) {
    return `${(watts * 1000).toFixed(0)} mW`;
  }
  return `${watts.toFixed(2)} W`;
}

function App() {
  const [metrics, setMetrics] = useState<SystemMetrics | null>(null);
  const [history, setHistory] = useState<SystemMetrics[]>([]);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const fetchMetrics = async () => {
      try {
        console.log("Fetching metrics...");
        const newMetrics = await invoke<SystemMetrics>("measure_metrics");
        console.log("Received metrics:", newMetrics);
        
        if (!newMetrics) {
          throw new Error("No metrics data received");
        }
        
        setMetrics(newMetrics);
        setHistory(prev => [...prev, newMetrics].slice(-30)); // Keep last 30 readings
        setError(null);
      } catch (error) {
        console.error("Failed to fetch metrics:", error);
        const errorMessage = error instanceof Error ? error.message : String(error);
        setError(`Failed to fetch metrics: ${errorMessage}`);
        setMetrics(null);
      }
    };

    // Initial fetch
    fetchMetrics();

    // Set up polling every second
    const interval = setInterval(fetchMetrics, 1000);

    return () => clearInterval(interval);
  }, []);

  const chartOptions: ChartOptions<'line'> = {
    responsive: true,
    animation: {
      duration: 0
    },
    scales: {
      y: {
        beginAtZero: true
      }
    },
    plugins: {
      legend: {
        position: 'top' as const,
      }
    }
  };

  const powerData = {
    labels: history.map(m => new Date(m.timestamp * 1000).toLocaleTimeString()),
    datasets: [
      {
        label: 'CPU Power',
        data: history.map(m => m.total_cpu_power),
        borderColor: 'rgb(255, 99, 132)',
        tension: 0.1
      },
      {
        label: 'GPU Power',
        data: history.map(m => m.total_gpu_power),
        borderColor: 'rgb(75, 192, 192)',
        tension: 0.1
      }
    ]
  };

  const cpuData = {
    labels: history.map(m => new Date(m.timestamp * 1000).toLocaleTimeString()),
    datasets: metrics?.cpu_cores.map(core => ({
      label: `Core ${core.id}`,
      data: history.map(m => m.cpu_cores[core.id].usage),
      borderColor: `hsl(${(core.id * 360) / metrics.cpu_cores.length}, 70%, 50%)`,
      tension: 0.1
    })) || []
  };

  return (
    <div className="container">
      <h1>System Monitor</h1>
      
      {error && (
        <div className="error-message">
          {error}
        </div>
      )}

      <div className="metrics-grid">
        {/* Power Section */}
        <div className="metric-card">
          <h2>Power Consumption</h2>
          <div className="power-stats">
            <div className="power-stat">
              <span>CPU Power:</span>
              <span>{metrics ? formatPower(metrics.total_cpu_power) : '0 W'}</span>
            </div>
            <div className="power-stat">
              <span>GPU Power:</span>
              <span>{metrics ? formatPower(metrics.total_gpu_power) : '0 W'}</span>
            </div>
            <div className="power-stat total">
              <span>Total Power:</span>
              <span>{metrics ? formatPower(metrics.total_cpu_power + metrics.total_gpu_power) : '0 W'}</span>
            </div>
          </div>
          <div className="chart-container">
            <Line options={chartOptions} data={powerData} />
          </div>
          <div className="sub-metric">
            Carbon Intensity: {metrics?.carbon_intensity.toFixed(2)} gCO2/kWh
          </div>
        </div>

        {/* CPU Section */}
        <div className="metric-card">
          <h2>CPU Metrics</h2>
          <div className="cpu-grid">
            {metrics?.cpu_cores.map(core => (
              <div key={core.id} className="cpu-core">
                <div className="core-header">Core {core.id}</div>
                <div className="core-stats">
                  <div>{core.usage.toFixed(1)}%</div>
                  <div>{core.frequency.toFixed(0)} MHz</div>
                </div>
                <div 
                  className="core-usage-bar"
                  style={{
                    width: `${core.usage}%`,
                    backgroundColor: `hsl(${(core.id * 360) / metrics.cpu_cores.length}, 70%, 50%)`
                  }}
                />
              </div>
            ))}
          </div>
          <div className="chart-container">
            <Line options={chartOptions} data={cpuData} />
          </div>
        </div>

        {/* Memory Section */}
        <div className="metric-card">
          <h2>Memory Usage</h2>
          <div className="memory-bar">
            <div 
              className="memory-used"
              style={{
                width: `${metrics ? (metrics.memory_used / metrics.memory_total * 100) : 0}%`
              }}
            />
          </div>
          <div className="memory-stats">
            <div>Used: {metrics ? formatBytes(metrics.memory_used) : '0 B'}</div>
            <div>Total: {metrics ? formatBytes(metrics.memory_total) : '0 B'}</div>
          </div>
        </div>

        {/* GPU Section */}
        <div className="metric-card">
          <h2>GPU Metrics</h2>
          {/* Overall GPU Stats */}
          <div className="gpu-overall-stats">
            <h3>Overall GPU</h3>
            <div className="gpu-stat">
              <span>Total Power:</span>
              <span>{metrics ? formatPower(metrics.total_gpu_power) : '0 W'}</span>
            </div>
            <div className="gpu-stat">
              <span>Average Usage:</span>
              <span>{metrics?.total_gpu_usage.toFixed(1)}%</span>
            </div>
            <div className="gpu-usage-bar">
              <div 
                className="gpu-usage"
                style={{
                  width: `${metrics?.total_gpu_usage || 0}%`
                }}
              />
            </div>
          </div>

          {/* Individual GPUs */}
          <div className="gpus-grid">
            {metrics?.gpus.map(gpu => (
              <div key={gpu.id} className="gpu-card">
                <div className="gpu-header">GPU {gpu.id}</div>
                <div className="gpu-stats">
                  <div className="gpu-stat">
                    <span>Usage:</span>
                    <span>{gpu.usage.toFixed(1)}%</span>
                  </div>
                  <div className="gpu-stat">
                    <span>Frequency:</span>
                    <span>{gpu.frequency.toFixed(0)} MHz</span>
                  </div>
                  <div className="gpu-stat">
                    <span>Power:</span>
                    <span>{formatPower(gpu.power)}</span>
                  </div>
                </div>
                <div className="gpu-usage-bar">
                  <div 
                    className="gpu-usage"
                    style={{
                      width: `${gpu.usage}%`,
                      backgroundColor: `hsl(${(gpu.id * 40 + 200) % 360}, 70%, 50%)`
                    }}
                  />
                </div>
              </div>
            ))}
          </div>
        </div>
      </div>
    </div>
  );
}

export default App;
