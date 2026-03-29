<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { open } from "@tauri-apps/plugin-dialog";
  import { listen } from "@tauri-apps/api/event";
  import { onMount, onDestroy } from 'svelte';
  
  interface Config {
    project: {
      name: string;
      evidence_root: string;
      registry_db: string;
      intelligence_db: string;
    };
    model: {
      id: string;
      local_path: string;
      context_length: number;
    };
  }
  
  interface RegistryProgress {
    total: number;
    processed: number;
    current: number;
    current_file: string;
    phase: string;
  }
  
  let config = $state<Config | null>(null);
  let modelLoaded = $state(false);
  let scanning = $state(false);
  let analyzing = $state(false);
  let progress = $state<RegistryProgress>({
    total: 0,
    processed: 0,
    current: 0,
    current_file: '',
    phase: ''
  });
  
  let unlistenProgress: (() => void) | null = null;
  let unlistenComplete: (() => void) | null = null;
  
  onMount(async () => {
    try {
      config = await invoke<Config>('load_config');
      modelLoaded = await invoke<boolean>('is_model_loaded');
    } catch (e) {
      console.error('Failed to load config:', e);
    }
    
    unlistenProgress = await listen<RegistryProgress>('registry_progress', (event) => {
      progress = event.payload;
    });
    
    unlistenComplete = await listen<number>('registry_complete', (event) => {
      progress.phase = 'complete';
      progress.processed = event.payload;
      scanning = false;
    });
  });
  
  onDestroy(() => {
    if (unlistenProgress) unlistenProgress();
    if (unlistenComplete) unlistenComplete();
  });
  
  async function configureFolders() {
    try {
      const selected = await open({
        directory: true,
        multiple: false,
        title: 'Select Evidence Folder',
      });
      
      if (selected && config) {
        config.project.evidence_root = selected as string;
        await invoke('save_config', { config });
        config = await invoke<Config>('load_config');
      }
    } catch (e) {
      console.error('Error selecting folder:', e);
    }
  }
  
  async function startScan() {
    if (!config?.project.evidence_root) {
      progress.phase = 'error';
      progress.current_file = 'Please configure evidence folder first';
      return;
    }
    
    scanning = true;
    progress = { phase: 'Initializing...', current: 0, processed: 0, total: 0, current_file: '' };
    
    try {
      await invoke('start_registry');
    } catch (e) {
      console.error('Scan error:', e);
      progress.phase = 'error';
      progress.current_file = `Error: ${e}`;
      scanning = false;
    }
  }
  
  async function initAndAnalyze() {
    if (!config?.model.local_path) {
      progress.phase = 'error';
      progress.current_file = 'No model configured. Please download a model in Settings.';
      return;
    }
    
    analyzing = true;
    progress = { phase: 'Loading model...', current: 0, processed: 0, total: 0, current_file: '' };
    
    try {
      if (!modelLoaded) {
        await invoke('init_reasoner', {
          modelPath: config.model.local_path,
          contextSize: config.model.context_length || 16384
        });
        modelLoaded = true;
      }
      
      progress.phase = 'Analyzing files...';
      
      const unprocessed = await invoke<any[]>('get_unprocessed_files', { limit: 10 });
      
      if (unprocessed.length === 0) {
        progress.phase = 'complete';
        progress.current_file = 'No files to analyze';
        analyzing = false;
        return;
      }
      
      progress.total = unprocessed.length;
      
      for (let i = 0; i < unprocessed.length; i++) {
        const file = unprocessed[i];
        progress.current_file = file.path;
        progress.processed = i + 1;
        
        try {
          await invoke('analyze_file', { path: file.path });
          await invoke('mark_processed', { fingerprint: file.fingerprint });
        } catch (e) {
          console.error('Analysis error for', file.path, e);
        }
      }
      
      progress.phase = 'complete';
    } catch (e) {
      console.error('Analysis error:', e);
      progress.phase = 'error';
      progress.current_file = `Error: ${e}`;
    } finally {
      analyzing = false;
    }
  }
</script>

<div class="analysis">
  <h1>Analysis</h1>
  
  <div class="analysis-grid">
    <section class="panel">
      <h2>Project Setup</h2>
      
      <div class="setup-info">
        <div class="info-row">
          <span class="info-label">Evidence Root:</span>
          <span class="info-value">{config?.project.evidence_root || 'Not configured'}</span>
        </div>
        <div class="info-row">
          <span class="info-label">Registry DB:</span>
          <span class="info-value">{config?.project.registry_db ? 'Configured' : 'Not configured'}</span>
        </div>
        <div class="info-row">
          <span class="info-label">Intelligence DB:</span>
          <span class="info-value">{config?.project.intelligence_db ? 'Configured' : 'Not configured'}</span>
        </div>
      </div>
      
      <button class="action-btn secondary" onclick={configureFolders}>
        Configure Folders
      </button>
    </section>
    
    <section class="panel">
      <h2>Registry Scanner</h2>
      <p class="description">
        Scan the evidence folder and create fingerprints of all files using SHA-256 hashing. Duplicate files are automatically skipped.
      </p>
      
      <div class="progress-section">
        {#if scanning}
          <div class="progress-bar">
            <div class="progress-fill" style="width: {progress.total > 0 ? (progress.processed / progress.total * 100) : 50}%"></div>
          </div>
          <div class="progress-text">
            {progress.phase} - {progress.processed}/{progress.total || '...'}
          </div>
          {#if progress.current_file}
            <div class="current-file">{progress.current_file}</div>
          {/if}
        {:else if progress.phase === 'complete'}
          <div class="idle-text">Scan complete - {progress.processed} files processed</div>
        {:else if progress.phase === 'error'}
          <div class="error-text">{progress.current_file}</div>
        {:else}
          <div class="idle-text">Ready to scan</div>
        {/if}
      </div>
      
      <button 
        class="action-btn primary" 
        onclick={startScan}
        disabled={scanning || analyzing}
      >
        {scanning ? 'Scanning...' : 'Start Fingerprinting'}
      </button>
    </section>
    
    <section class="panel">
      <h2>Neural Reasoner</h2>
      <p class="description">
        Extract facts from processed files using local LLM inference. The model is cached in memory for fast processing.
      </p>
      
      <div class="model-info">
        <div class="info-row">
          <span class="info-label">Model:</span>
          <span class="info-value" class:loaded={modelLoaded}>
            {modelLoaded ? 'Loaded' : (config?.model.local_path ? 'Not loaded' : 'No model')}
          </span>
        </div>
        <div class="info-row">
          <span class="info-label">Model Path:</span>
          <span class="info-value path">{config?.model.local_path || 'Not set'}</span>
        </div>
        <div class="info-row">
          <span class="info-label">Context:</span>
          <span class="info-value">{config?.model.context_length || 16384} tokens</span>
        </div>
      </div>
      
      <div class="progress-section">
        {#if analyzing}
          <div class="progress-bar">
            <div class="progress-fill indeterminate"></div>
          </div>
          <div class="progress-text">{progress.phase}</div>
          {#if progress.current_file}
            <div class="current-file">{progress.current_file}</div>
          {/if}
        {:else if progress.phase === 'complete'}
          <div class="idle-text">Analysis complete</div>
        {:else if progress.phase === 'error'}
          <div class="error-text">{progress.current_file}</div>
        {:else}
          <div class="idle-text">Ready to analyze</div>
        {/if}
      </div>
      
      <button 
        class="action-btn primary" 
        onclick={initAndAnalyze}
        disabled={scanning || analyzing || !config?.model.local_path}
      >
        {analyzing ? 'Analyzing...' : 'Start Neural Reasoner'}
      </button>
    </section>
  </div>
</div>

<style>
  .analysis {
    max-width: 1200px;
  }
  
  h1 {
    font-size: 1.75rem;
    margin-bottom: 1.5rem;
    color: #eaeaea;
  }
  
  h2 {
    font-size: 1.25rem;
    margin-bottom: 0.75rem;
    color: #e94560;
  }
  
  .analysis-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(350px, 1fr));
    gap: 1.5rem;
  }
  
  .panel {
    background-color: #16213e;
    padding: 1.5rem;
    border-radius: 8px;
    border: 1px solid #0f3460;
  }
  
  .description {
    color: #9ca3af;
    font-size: 0.875rem;
    margin-bottom: 1rem;
    line-height: 1.5;
  }
  
  .setup-info,
  .model-info {
    margin-bottom: 1rem;
  }
  
  .info-row {
    display: flex;
    justify-content: space-between;
    padding: 0.5rem 0;
    border-bottom: 1px solid #0f3460;
  }
  
  .info-label {
    color: #9ca3af;
    font-size: 0.875rem;
  }
  
  .info-value {
    color: #eaeaea;
    font-size: 0.875rem;
    font-weight: 500;
  }
  
  .info-value.loaded {
    color: #4ade80;
  }
  
  .info-value.path {
    max-width: 200px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  
  .progress-section {
    margin: 1.5rem 0;
    min-height: 80px;
  }
  
  .progress-bar {
    height: 8px;
    background-color: #1a1a2e;
    border-radius: 4px;
    overflow: hidden;
    margin-bottom: 0.5rem;
  }
  
  .progress-fill {
    height: 100%;
    background-color: #e94560;
    transition: width 0.3s ease;
  }
  
  .progress-fill.indeterminate {
    width: 30%;
    animation: indeterminate 1.5s infinite linear;
  }
  
  @keyframes indeterminate {
    0% { transform: translateX(-100%); }
    100% { transform: translateX(400%); }
  }
  
  .progress-text {
    font-size: 0.875rem;
    color: #9ca3af;
  }
  
  .current-file {
    font-size: 0.75rem;
    color: #6b7280;
    margin-top: 0.25rem;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  
  .idle-text {
    color: #4ade80;
    font-size: 0.875rem;
  }
  
  .error-text {
    color: #ef4444;
    font-size: 0.875rem;
  }
  
  .action-btn {
    width: 100%;
    padding: 0.875rem 1rem;
    border: none;
    border-radius: 6px;
    font-size: 1rem;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.2s;
  }
  
  .action-btn.primary {
    background-color: #e94560;
    color: #ffffff;
  }
  
  .action-btn.primary:hover:not(:disabled) {
    background-color: #d13650;
  }
  
  .action-btn.secondary {
    background-color: #0f3460;
    color: #eaeaea;
  }
  
  .action-btn.secondary:hover:not(:disabled) {
    background-color: #1a4a7a;
  }
  
  .action-btn:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }
</style>
