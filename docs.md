### **I. Структура директорий проекта (Rust Workspace)**

Проект оформляется как единый Cargo Workspace для жесткой линковки общих структур (схем графов и IPC) между ядром и воркерами.

Plaintext  
eva-hypervisor/  
├── Cargo.toml                  \# Workspace root (members \= \["core", "worker-candle", "shared-ipc"\])  
├── shared-ipc/                 \# Разделяемая память и контракты (компилируется в оба бинарника)  
│   ├── src/  
│   │   ├── lib.rs              \# Точка входа IPC  
│   │   ├── memory\_map.rs       \# Разметка shmem (Header, Ring Buffer, Atomics)  
│   │   └── protocol.rs         \# Enum флагов (IDLE, EXEC, REQ\_DATA, STREAM)  
├── core/                       \# Демон-гипервизор (Ядро)  
│   ├── src/  
│   │   ├── main.rs  
│   │   ├── api/                \# axum REST/SSE роуты  
│   │   ├── scheduler/          \# Вычисление WSJF, петлевой обход DAG (petgraph)  
│   │   ├── router/             \# Zero-Node, парсер YAML, генератор динамических JSON  
│   │   ├── engine/             \# MCP Runtime, исполнение nix-shell, curl  
│   │   └── state/              \# sled KV-хранилище, константы железа, EMA-метрики  
├── worker-candle/              \# Изолированный процесс инференса (без сети)  
│   ├── src/  
│   │   ├── main.rs             \# Точка входа (читает shmem fd)  
│   │   ├── model\_loader.rs     \# Загрузка .safetensors (свап RAM \<-\> VRAM)  
│   │   └── inference.rs        \# Цикл генерации, запись токенов в кольцевой буфер  
└── .config/eva/                \# Директория конфигураций (на хост-машине пользователя)  
    ├── daemon.toml             \# Настройки портов, лимиты VRAM  
    ├── hardware\_profile.json   \# Результаты Init Benchmark (T\_estimate константы)  
    ├── mcp\_tools/              \# .md файлы с описанием инструментов для Zero-Node  
    └── templates/              \# Директория Fast Track (YAML пайплайны)

### **II. Протокол Shared Memory (IPC Protocol)**

Используем крейт shared\_memory. Ядро аллоцирует статичные сегменты при старте (например, по 256 МБ на воркер) \+ динамический mmap для огромных RAG-контекстов.  
**Разметка сегмента памяти (C-repr Structs):**

> 1. **State Header (Атомарные регистры, 64 байта):**  
   * STATUS\_FLAG: Атомарный u32. Состояния: 0x0 (IDLE), 0x1 (LOAD\_WEIGHTS), 0x2 (EXEC\_INFER), 0x3 (STREAMING), 0x4 (REQ\_DATA), 0x5 (DONE), 0x6 (ERROR).  
   * WORKER\_HEARTBEAT: AtomicU64 (timestamp для watchdog ядра).  
> 2. **Control Block (Метаданные, 1 КБ):**  
   * model\_id: Хэш требуемых весов/LoRA.  
   * context\_length: Размер входящего промпта.  
   * max\_tokens: Лимит генерации.  
> 3. **Input Buffer (Чтение воркером):**  
   * Сырой UTF-8 или токенизированный бинарный массив промпта.  
   * Если контекст больше сегмента — ядро пишет сюда путь к временно смапленному файлу (mmap), воркер читает его напрямую.  
> 4. **Output Ring Buffer (Запись воркером):**  
   * Кольцевой буфер Lock-Free (SPSC \- Single Producer Single Consumer).  
   * Воркер кладет сгенерированные токены, ядро мгновенно их забирает для API (SSE стриминг) или для следующего узла DAG.

### **III. YAML Темплейт Пайплайна (Схема Fast Track)**

Пример декларативного графа для автоматического дебага с условным ветвлением. Этот файл лежит в \~/.config/eva/templates/sys\_debugger.yaml.

YAML  
schema\_version: "1.0"  
task\_priority: 8             \# P\_task для формулы WSJF

nodes:  
  fetch\_error\_logs:  
    dependencies: \[\]         \# Выполняется первым  
    action\_type: MCP\_Call  
    target\_model: null  
    payload: "journalctl \-p 3 \-xb \-n 50 \--output=json"

  analyze\_dump:  
    dependencies: \["fetch\_error\_logs"\]  
    action\_type: LLM\_Inference  
    target\_model: "llama-3-8b-instruct-q4"  \# Легковесная модель для VRAM  
    system\_prompt: |  
      Ты системный аналитик. Определи причину падения на основе логов.   
      Верни ТОЛЬКО одно из трех слов: "OOM", "NETWORK\_TIMEOUT", "UNKNOWN".  
    payload: "{{nodes.fetch\_error\_logs.output}}"

  branching\_logic:  
    dependencies: \["analyze\_dump"\]  
    action\_type: Condition\_If\_Else  
    payload: |  
      if "{{nodes.analyze\_dump.output}}" \== "OOM":  
         activate("restart\_service\_with\_limits")  
      elif "{{nodes.analyze\_dump.output}}" \== "NETWORK\_TIMEOUT":  
         activate("ping\_gateway")  
      else:  
         activate("escalate\_to\_heavy\_model")

  restart\_service\_with\_limits:  
    dependencies: \["branching\_logic"\]  
    action\_type: MCP\_Call  
    payload: "systemctl restart crashed\_service \--drop-in=limit\_ram"

  escalate\_to\_heavy\_model:  
    dependencies: \["branching\_logic"\]  
    action\_type: LLM\_Inference  
    target\_model: "remote\_gpt4\_or\_heavy\_local" \# Запуск глубокого анализа  
    system\_prompt: "Проведи глубокий траблшутинг..."  
    payload: "{{nodes.fetch\_error\_logs.output}}"

### **IV. API Документация (REST/gRPC Эндпоинты)**

Демон слушает локальный сокет или TCP-порт (например, 127.0.0.1:8080). Все вызовы внешних утилит, скриптов, логгеров или пользовательских UI осуществляются сюда.

#### **1\. Ingress: Постановка задачи**

POST /api/v1/task/submit  
**Payload (JSON):**

JSON  
{  
  "task\_id": "optional\_custom\_id\_123",   
  "template\_id": "sys\_debugger",  // Если null \- уходит в Deep Track (Zero-Node)  
  "priority\_override": 9,         // Ручной override P\_task (опционально)  
  "prompt": "Упал nginx, разберись",   
  "context\_attachments": \[  
    {"type": "text/plain", "content": "error.log content..."}  
  \]  
}

**Response:** 202 Accepted

JSON  
{ "job\_id": "uuid-...", "status": "QUEUED" }

#### **2\. Egress: Стриминг выполнения графа (SSE)**

GET /api/v1/task/stream/{job\_id}  
Возвращает Server-Sent Events в реальном времени, транслируя логику ядра и токены воркеров.  
**Stream Data:**

* \[SYS\] Node 'fetch\_error\_logs' completed in 12ms.  
* \[LLM\] \<token\> \<token\> \<token\>...  
* \[SYS\] Branching evaluated: Escalating to heavy model.

#### **3\. Control Plane: Управление планировщиком**

GET /api/v1/hypervisor/queue  
Возвращает текущее состояние DAG-очередей, пулы моделей и текущий WSJF-скор каждой задачи. Отлично подходит для TTY-монитора (htop-like интерфейса).  
POST /api/v1/hypervisor/preempt  
Принудительно приостанавливает выполнение текущего пула GPU, заставляет воркер сбросить контекст в CPU RAM и загрузить модель для задачи с priority: 9\.

#### **4\. Hardware Calibration: Перерасчет Cost Units ($T\_{estimate}$)**

POST /api/v1/system/benchmark  
Триггерит перекалибровку.

* Демон прогоняет стресс-тесты mmap, DMA-передач.  
* Пингует удаленные API.  
* Обновляет hardware\_profile.json и перезагружает коэффициенты $\\alpha$ для EMA.

#### **5\. MCP Registration: Динамическое добавление утилит**

POST /api/v1/mcp/register  
Позволяет внешним скриптам или плагинам регистрировать новые .md мануалы в context\_engine без перезапуска демона, расширяя кругозор Zero-Node.