<script lang="ts">
  import { SvelteFlow, Background, Controls } from '@xyflow/svelte';
  import '@xyflow/svelte/dist/style.css';
  import { Send, Paperclip, Activity, Cpu, HardDrive } from 'lucide-svelte';
  import { onMount } from 'svelte';

  // State
  let selectedModel = 'auto'; // 'auto' means Deep Track, otherwise Fast Track pipeline
  let contextWindow = 4096;
  let priority = 9;
  let chatInput = '';
  
  // Chat History
  let chatHistory = [
    { role: 'system', content: 'Eva Hypervisor is online. Ready for tasks.' }
  ];

  // DAG Nodes (Mock for now)
  let nodes = [];
  let edges = [];

  // Active models (mocked for now until backend sends it)
  let activeModels = "[]";

  async function submitTask() {
    if (!chatInput.trim()) return;
    
    const userMessage = chatInput;
    chatHistory = [...chatHistory, { role: 'user', content: userMessage }];
    chatInput = '';
    
    // Send to eva-api
    try {
      const res = await fetch('/api/v1/tasks', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          prompt: userMessage,
          template_id: selectedModel === 'auto' ? null : selectedModel,
          priority: priority
        })
      });

      if (!res.ok) throw new Error(`HTTP error! status: ${res.status}`);
      const data = await res.json();
      
      chatHistory = [...chatHistory, { role: 'system', content: `Task ${data.task_id} queued.` }];
      
      // In a real implementation we would open an SSE/WebSocket to /api/v1/tasks/${data.task_id}/stream here
    } catch (err) {
      chatHistory = [...chatHistory, { role: 'assistant', content: `[ERROR]: Failed to connect to API Gateway. ${err}` }];
    }
  }

  // Polling for telemetry
  onMount(() => {
    const interval = setInterval(async () => {
      try {
        const qRes = await fetch('/api/v1/scheduler/queue');
        if (qRes.ok) {
          const qData = await qRes.json();
          // Map backend tasks to Svelte Flow nodes
          if (qData && qData.tasks && Array.isArray(qData.tasks)) {
            let newNodes = [];
            let newEdges = [];
            
            // Ingress node
            newNodes.push({ id: 'ingress', type: 'input', data: { label: 'Ingress Queue' }, position: { x: 250, y: 50 } });
            
            qData.tasks.forEach((task, index) => {
               const yPos = 150 + (index * 100);
               newNodes.push({
                 id: task.id,
                 data: { label: `[${task.priority}] ${task.node} (${task.status})` },
                 position: { x: 250, y: yPos }
               });
               
               // Connect to previous
               const sourceId = index === 0 ? 'ingress' : qData.tasks[index - 1].id;
               newEdges.push({
                 id: `e-${sourceId}-${task.id}`,
                 source: sourceId,
                 target: task.id,
                 animated: task.status === 'RUNNING'
               });
            });
            
            nodes = newNodes;
            edges = newEdges;
          } else {
            // Default empty graph state
            nodes = [{ id: 'empty', type: 'input', data: { label: 'No Active Tasks' }, position: { x: 250, y: 150 } }];
            edges = [];
          }
        }
        
        // Also fetch active models registry
        const mRes = await fetch('/api/v1/registry/models');
        if (mRes.ok) {
           const mData = await mRes.json();
           if (mData.models) activeModels = mData.models;
        }
      } catch (err) {
        // Silent fail on telemetry
      }
    }, 2000);

    return () => clearInterval(interval);
  });
</script>

<main class="app-container">
  <!-- Left Panel: Chat Interface -->
  <section class="chat-panel glass-panel">
    <div class="chat-header">
      <div class="logo">
        <Activity size={24} color="var(--neon-green)" />
        <h2>Eva OS <span class="badge">v1.1.0</span></h2>
      </div>
      
      <div class="settings">
        <div class="setting-item">
          <label for="model">Track:</label>
          <select id="model" bind:value={selectedModel}>
            <option value="auto">Deep Track (Auto)</option>
            <option value="sys_debugger">Pipeline: sys_debugger</option>
            <option value="data_rag">Pipeline: data_rag</option>
          </select>
        </div>
        <div class="setting-row">
          <div class="setting-item">
            <label for="ctx">Context:</label>
            <input id="ctx" type="number" bind:value={contextWindow} step="1024" />
          </div>
          <div class="setting-item">
            <label for="prio">Prio:</label>
            <input id="prio" type="number" bind:value={priority} min="0" max="9" />
          </div>
        </div>
      </div>
    </div>

    <div class="chat-messages">
      {#each chatHistory as msg}
        <div class="message {msg.role}">
          <div class="bubble">
            {#if msg.role === 'system'}
               <span class="mono">[{msg.role.toUpperCase()}]</span> {msg.content}
            {:else}
               {msg.content}
            {/if}
          </div>
        </div>
      {/each}
    </div>

    <div class="chat-input-area">
      <button class="icon-btn" title="Attach Context">
        <Paperclip size={20} />
      </button>
      <input 
        type="text" 
        placeholder="Enter task or instruction..." 
        bind:value={chatInput} 
        on:keydown={(e) => e.key === 'Enter' && submitTask()}
      />
      <button class="icon-btn neon" on:click={submitTask}>
        <Send size={20} />
      </button>
    </div>
  </section>

  <!-- Right Panel: DAG Monitor & Telemetry -->
  <section class="monitor-panel">
    <!-- Telemetry Header -->
    <div class="telemetry-bar glass-panel">
      <div class="stat">
        <Cpu size={16} />
        <span>RAM: 4.2 / 32 GB</span>
      </div>
      <div class="stat">
        <HardDrive size={16} />
        <span>VRAM: 8.1 / 16 GB</span>
      </div>
      <div class="stat highlight">
        <span>Models API: {activeModels === '' ? 'Empty' : 'Connected'}</span>
      </div>
      <div class="stat">
        <span>Scheduler: WSJF Active</span>
      </div>
    </div>

    <!-- DAG Flow Canvas -->
    <div class="dag-canvas glass-panel">
      <div class="canvas-title">Task DAG Monitor (Read-Only)</div>
      <SvelteFlow {nodes} {edges} fitView theme="dark">
        <Background bgColor="transparent" patternColor="var(--border-glass)" />
        <Controls />
      </SvelteFlow>
    </div>
  </section>
</main>

<style>
  .app-container {
    display: flex;
    height: 100vh;
    width: 100vw;
    padding: 1rem;
    gap: 1rem;
  }

  /* Chat Panel */
  .chat-panel {
    width: 400px;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .chat-header {
    padding: 1.5rem;
    border-bottom: 1px solid var(--border-glass);
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }

  .logo {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .logo h2 {
    margin: 0;
    font-size: 1.25rem;
    font-weight: 600;
  }

  .badge {
    font-size: 0.7rem;
    background: var(--neon-green-dim);
    color: var(--neon-green);
    padding: 0.2rem 0.5rem;
    border-radius: 4px;
    vertical-align: middle;
  }

  .settings {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .setting-row {
    display: flex;
    gap: 0.5rem;
  }

  .setting-item {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
    flex: 1;
  }

  .setting-item label {
    font-size: 0.75rem;
    color: var(--text-muted);
    text-transform: uppercase;
    font-family: var(--font-mono);
  }

  .chat-messages {
    flex: 1;
    overflow-y: auto;
    padding: 1.5rem;
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }

  .message {
    display: flex;
    width: 100%;
  }

  .message.user {
    justify-content: flex-end;
  }

  .message.assistant {
    justify-content: flex-start;
  }

  .message.system {
    justify-content: center;
  }

  .bubble {
    max-width: 85%;
    padding: 0.75rem 1rem;
    border-radius: 8px;
    font-size: 0.9rem;
    line-height: 1.4;
  }

  .message.user .bubble {
    background: var(--bg-glass-hover);
    border: 1px solid var(--border-glass);
  }

  .message.assistant .bubble {
    background: rgba(0, 255, 128, 0.05);
    border: 1px solid var(--neon-green-dim);
    color: var(--neon-green);
  }

  .message.system .bubble {
    background: transparent;
    border: 1px dashed var(--text-muted);
    color: var(--text-muted);
    font-size: 0.8rem;
  }

  .mono {
    font-family: var(--font-mono);
    color: var(--neon-green);
  }

  .chat-input-area {
    padding: 1rem;
    border-top: 1px solid var(--border-glass);
    display: flex;
    gap: 0.5rem;
    align-items: center;
  }

  .chat-input-area input {
    flex: 1;
    background: rgba(0,0,0,0.4);
  }

  .icon-btn {
    background: transparent;
    border: none;
    color: var(--text-muted);
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 0.5rem;
    border-radius: 6px;
    transition: all 0.2s;
  }

  .icon-btn:hover {
    color: var(--text-main);
    background: var(--bg-glass-hover);
  }

  .icon-btn.neon {
    color: var(--neon-green);
  }

  .icon-btn.neon:hover {
    background: var(--neon-green-dim);
    box-shadow: 0 0 10px var(--neon-green-dim);
  }

  /* Monitor Panel */
  .monitor-panel {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 1rem;
    min-width: 0;
  }

  .telemetry-bar {
    display: flex;
    align-items: center;
    padding: 1rem 1.5rem;
    gap: 2rem;
    font-family: var(--font-mono);
    font-size: 0.85rem;
  }

  .stat {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    color: var(--text-muted);
  }

  .stat.highlight {
    color: var(--neon-green);
  }

  .dag-canvas {
    flex: 1;
    position: relative;
    overflow: hidden;
  }

  .canvas-title {
    position: absolute;
    top: 1rem;
    left: 1rem;
    z-index: 10;
    font-family: var(--font-mono);
    font-size: 0.8rem;
    color: var(--neon-green);
    text-transform: uppercase;
    letter-spacing: 1px;
  }
</style>
