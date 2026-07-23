# Eva OS Registry Schemas 🌌

This directory contains the core configuration files that define the capabilities, models, and execution graphs (pipelines) for the Eva Hypervisor. These files act as the "source of truth" for the AI Operating System.

## 1. `cat_registry.yaml` (Context-Augmenting Tools)

Defines the strictly allowed Read-Only tools that the Data Gatherer (Phase 1) can use to enrich the context.

```yaml
tools:
  [tool_name]: # e.g., 'ls', 'grep'
    description: "Human and AI-readable description of what this tool does."
    allowed_flags:
      "[flag]": "Description of the flag" # e.g., "-l": "Long format"
```
* **Security Note:** The Hypervisor will strictly validate any LLM tool call against this registry. If an LLM hallucinates a flag not present here, the execution will be blocked and the LLM will be reprompted.

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
    node_type: "[string]"        # Type of node: 'inference', 'cat_executor', 'rag_search', 'mcp_call'
    
    # --- For 'inference' nodes ---
    model: "[model_id]"          # Must match an ID from model_registry.yaml
    thinking_mode: [bool]        # (Optional) If true, prompts the model to use <think> tags
    prompt_template: |           # The system prompt / instructions
      You can use Jinja-like templating here: {{ context }}, {{ registry.cat.tools }}
      
    # --- Common Fields ---
    depends_on: ["[node_id]"]    # (Optional) Array of node IDs that must complete before this one
    next: ["[node_id]", "end"]   # Array of node IDs to execute after this one completes
```

### Cognitive Pipeline Injection Tags
In `prompt_template` or system prompts, specific tags can be used to instruct the Hypervisor to dynamically inject data:
* `<INJECT_RAG_CONTEXT_HERE>`: The `rag_search` node will replace this tag with vectorized, semantic search results.

---
**Note to AI Agents (Pipeline Architect):**
When constructing a new pipeline DAG, you MUST strictly adhere to the `pipelines/*.yaml` schema defined above. Do not invent new `node_type` values.
