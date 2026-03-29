<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from 'svelte';
  
  interface Stats {
    registry_count: number;
    intelligence_count: number;
  }
  
  interface HardwareStatus {
    cpu_threads: number;
    total_memory_gb: number;
    available_memory_gb: number;
    recommended_backend: string;
    scaling: {
      batch_size: number;
      cpu_workers: number;
    };
    gpu_info: Array<{
      name: string;
      vendor: string;
      vram_mb: number;
    }>;
  }
  
  let stats = $state<Stats>({
    registry_count: 0,
    intelligence_count: 0
  });
  
  let hardware = $state<HardwareStatus | null>(null);
  let modelLoaded = $state(false);
  let modelPath = $state('');
  let loading = $state(true);
  
  onMount(async () => {
    try {
      stats = await invoke<Stats>('get_stats');
      
      hardware = await invoke<HardwareStatus>('detect_hardware');
      
      const config = await invoke<any>('load_config');
      modelPath = config.model?.local_path || '';
      modelLoaded = await invoke<boolean>('is_model_loaded');
    } catch (e) {
      console.error('Failed to load data:', e);
    } finally {
      loading = false;
    }
  });
</script>

<div class="dashboard">
  <h1>Dashboard</h1>
  
  <div class="cards">
    <div class="card">
      <svg class="card-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"/>
      </svg>
      <div class="card-content">
        <div class="card-value">{loading ? '...' : stats.registry_count}</div>
        <div class="card-label">Files Registered</div>
      </div>
    </div>
    
    <div class="card">
      <svg class="card-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <path d="M22 11.08V12a10 10 0 1 1-5.93-9.14"/>
        <polyline points="22 4 12 14.01 9 11.01"/>
      </svg>
      <div class="card-content">
        <div class="card-value">{loading ? '...' : stats.intelligence_count}</div>
        <div class="card-label">Facts Extracted</div>
      </div>
    </div>
    
    <div class="card">
      <svg class="card-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <rect x="4" y="4" width="16" height="16" rx="2" ry="2"/>
        <rect x="9" y="9" width="6" height="6"/>
        <line x1="9" y1="1" x2="9" y2="4"/>
        <line x1="15" y1="1" x2="15" y2="4"/>
        <line x1="9" y1="20" x2="9" y2="23"/>
        <line x1="15" y1="20" x2="15" y2="23"/>
        <line x1="20" y1="9" x2="23" y2="9"/>
        <line x1="20" y1="14" x2="23" y2="14"/>
        <line x1="1" y1="9" x2="4" y2="9"/>
        <line x1="1" y1="14" x2="4" y2="14"/>
      </svg>
      <div class="card-content">
        <div class="card-value">{hardware?.scaling?.cpu_workers || '...'}</div>
        <div class="card-label">CPU Workers</div>
      </div>
    </div>
  </div>
  
  {#if hardware}
    <div class="info-section">
      <h2>Hardware Status</h2>
      <div class="info-grid">
        <div class="info-card">
          <span class="info-label">CPU</span>
          <span class="info-value">{hardware.cpu_threads} cores</span>
        </div>
        <div class="info-card">
          <span class="info-label">Memory</span>
          <span class="info-value">{hardware.total_memory_gb.toFixed(1)} GB total</span>
        </div>
        <div class="info-card">
          <span class="info-label">Available</span>
          <span class="info-value">{hardware.available_memory_gb.toFixed(1)} GB</span>
        </div>
        <div class="info-card">
          <span class="info-label">Backend</span>
          <span class="info-value">{hardware.recommended_backend}</span>
        </div>
        {#if hardware.gpu_info.length > 0}
          <div class="info-card full-width">
            <span class="info-label">GPU</span>
            <span class="info-value">{hardware.gpu_info[0].name} ({hardware.gpu_info[0].vram_mb} MB)</span>
          </div>
        {/if}
      </div>
    </div>
  {/if}
  
  <div class="model-status">
    <h2>Model Status</h2>
    <div class="status-row">
      <span class="status-label">Model:</span>
      <span class="status-value" class:loaded={modelLoaded}>
        {modelLoaded ? 'Loaded' : (modelPath ? 'Not loaded' : 'No model')}
      </span>
    </div>
    {#if modelPath}
      <div class="status-row">
        <span class="status-label">Path:</span>
        <span class="status-value path">{modelPath}</span>
      </div>
    {/if}
  </div>
  
  <div class="quick-actions">
    <h2>Quick Actions</h2>
    <div class="action-buttons">
      <a href="/analysis" class="action-btn">
        <svg class="action-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <circle cx="11" cy="11" r="8"/>
          <path d="M21 21l-4.35-4.35"/>
        </svg>
        <span class="action-label">Start Analysis</span>
      </a>
      <a href="/results" class="action-btn">
        <svg class="action-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <line x1="8" y1="6" x2="21" y2="6"/>
          <line x1="8" y1="12" x2="21" y2="12"/>
          <line x1="8" y1="18" x2="21" y2="18"/>
          <circle cx="4" cy="6" r="1" fill="currentColor"/>
          <circle cx="4" cy="12" r="1" fill="currentColor"/>
          <circle cx="4" cy="18" r="1" fill="currentColor"/>
        </svg>
        <span class="action-label">View Results</span>
      </a>
      <a href="/settings" class="action-btn">
        <svg class="action-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <circle cx="12" cy="12" r="3"/>
          <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z"/>
        </svg>
        <span class="action-label">Settings</span>
      </a>
    </div>
  </div>
</div>

<style>
  .dashboard {
    max-width: 1200px;
  }
  
  h1 {
    font-size: 1.75rem;
    margin-bottom: 1.5rem;
    color: #eaeaea;
  }
  
  h2 {
    font-size: 1.25rem;
    margin-bottom: 1rem;
    color: #9ca3af;
  }
  
  .cards {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(250px, 1fr));
    gap: 1rem;
    margin-bottom: 2rem;
  }
  
  .card {
    display: flex;
    align-items: center;
    gap: 1rem;
    padding: 1.5rem;
    background-color: #16213e;
    border-radius: 8px;
    border: 1px solid #0f3460;
  }
  
  .card-icon {
    width: 40px;
    height: 40px;
    color: #e94560;
  }
  
  .card-value {
    font-size: 2rem;
    font-weight: 700;
    color: #e94560;
  }
  
  .card-label {
    font-size: 0.875rem;
    color: #9ca3af;
  }
  
  .quick-actions {
    margin-top: 2rem;
  }
  
  .action-buttons {
    display: flex;
    gap: 1rem;
    flex-wrap: wrap;
  }
  
  .action-btn {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    padding: 1rem 1.5rem;
    background-color: #0f3460;
    border-radius: 8px;
    text-decoration: none;
    color: #eaeaea;
    transition: all 0.2s;
  }
  
  .action-btn:hover {
    background-color: #e94560;
    transform: translateY(-2px);
  }
  
  .action-icon {
    width: 20px;
    height: 20px;
  }
  
  .action-label {
    font-size: 1rem;
    font-weight: 500;
  }
  
  .info-section {
    margin-top: 2rem;
  }
  
  .info-section h2 {
    margin-bottom: 1rem;
  }
  
  .info-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(150px, 1fr));
    gap: 1rem;
  }
  
  .info-card {
    display: flex;
    flex-direction: column;
    padding: 1rem;
    background-color: #16213e;
    border-radius: 8px;
    border: 1px solid #0f3460;
  }
  
  .info-card.full-width {
    grid-column: 1 / -1;
  }
  
  .info-card .info-label {
    font-size: 0.75rem;
    color: #9ca3af;
    margin-bottom: 0.25rem;
  }
  
  .info-card .info-value {
    font-size: 0.875rem;
    color: #eaeaea;
    font-weight: 500;
  }
  
  .model-status {
    margin-top: 2rem;
    padding: 1.5rem;
    background-color: #16213e;
    border-radius: 8px;
    border: 1px solid #0f3460;
  }
  
  .model-status h2 {
    margin-bottom: 1rem;
  }
  
  .status-row {
    display: flex;
    align-items: center;
    gap: 1rem;
    padding: 0.5rem 0;
  }
  
  .status-label {
    font-size: 0.875rem;
    color: #9ca3af;
    min-width: 60px;
  }
  
  .status-value {
    font-size: 0.875rem;
    color: #eaeaea;
  }
  
  .status-value.loaded {
    color: #4ade80;
  }
  
  .status-value.path {
    font-family: 'SF Mono', Monaco, monospace;
    font-size: 0.75rem;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    max-width: 400px;
  }
</style>
