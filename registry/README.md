# EvaProject Registry Schemas 🌌

This directory contains the core configuration files that define the capabilities, models, and execution graphs (pipelines) for the Eva Eva. These files act as the "source of truth" for the AI Operating System.

## 1. `cat_registry.yaml` (Context-Augmenting Tools)

Defines the strictly allowed Read-Only tools that the Data Gatherer (Phase 1) can use to enrich the context.

```yaml
tools:
  [tool_name]: # e.g., 'ls', 'grep'
    description: "Human and AI-readable description of what this tool does."
    allowed_flags:
      "[flag]": "Description of the flag" # e.g., "-l": "Long format"
```

## 2. `model_registry.yaml` (Hardware & Model Capabilities)

Defines the neural networks available to the system, including their hardware footprint and qualitative traits. Used by the Pipeline Architect and the Scheduler.

```yaml
models:
  [model_id]: # e.g., 'phi-4', 'gemma-3-1b'
    vram_mb: [int]               # Estimated VRAM usage in MB
    context_window: [int]        # Maximum context window size
    supports_thinking: [bool]    # Whether the model natively supports <think> CoT tags
    tags: ["[string]", ...]      # Semantic tags (e.g., 'router', 'architect')
    green_flags: "[string]"      # What the model excels at
    red_flags: "[string]"        # What the model fails at

default_params:
  temperature: [float]           # Default temperature for inference
  top_p: [float]                 # Default top_p sampling
  repetition_penalty: [float]    # Penalty for token repetition
  max_loop_iterations: [int]     # Hard limit for DAG cycle prevention
```

## 3. `pipelines/*.yaml` (DAG Execution Graphs)

Defines the execution flow. Pipelines are Directed Acyclic Graphs (DAGs) consisting of nodes. The `Pipeline Architect` AI generates these files dynamically, or they can be statically defined (like `default_ingress.yaml`).

```yaml
id: "[string]"                   # Unique identifier for the pipeline
description: "[string]"          # Human-readable description

nodes:
  - id: "[string]"               # Unique node ID
    node_type: "[string]"        # See "Node Types" below
    
    # --- For 'inference' nodes ---
    model: "[model_id]"          # Must match an ID from model_registry.yaml
    thinking_mode: [bool]        # (Optional) If true, prompts the model to use <think> tags
    prompt_template: |           # The system prompt / instructions
      You can use Jinja-like templating here: {{ context }}, {{ registry.cat.tools }}
      
    # --- Common Fields ---
    depends_on: ["[node_id]"]    # (Optional) Array of node IDs that must complete before this one
    next: ["[node_id]", "end"]   # Array of node IDs to execute after this one completes
```

### 🧠 Supported Node Types (`node_type`)
The `node_type` tells the Eva's DAG Executor *how* to process this specific node.
* **`inference`**: The Eva allocates VRAM, spawns a worker, and sends the rendered `prompt_template` to a local LLM via shared memory IPC.
* **`cat_executor`**: The Eva takes the output of the *previous* `inference` node (which must be a JSON array of commands), validates it against `cat_registry.yaml`, and executes it in a secure `nix-shell`. The output is appended to the global context.
* **`rag_search`**: The Eva's `ContextEngine` performs a vector search (HNSW) based on keywords and replaces `<INJECT_RAG_CONTEXT_HERE>` tags in the prompt with retrieved text chunks.
* **`mcp_call`**: The Eva calls a registered Model Context Protocol agent/endpoint (to be implemented).

### 📝 Jinja Context Variables (Templating)
When writing a `prompt_template`, the Eva dynamically interpolates these variables before sending the prompt to the LLM:
* `{{ task.input }}`: The original raw input/request from the user.
* `{{ context }}`: The cumulative text accumulated from previous nodes (e.g., outputs of `cat_executor` or previous inferences).
* `{{ registry.cat.tools }}`: Injects a formatted list of all available CAT tools and their allowed flags from `cat_registry.yaml`. This ensures the LLM knows its exact limits.
* `{{ registry.models }}`: Injects a formatted list of all available neural networks and their red/green flags from `model_registry.yaml`. Used by the Pipeline Architect to build custom DAGs.

---
**Note to AI Agents (Pipeline Architect):**
When constructing a new pipeline DAG, you MUST strictly adhere to the `pipelines/*.yaml` schema defined above. Do not invent new `node_type` values or context variables.
