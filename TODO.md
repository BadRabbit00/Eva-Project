# TODO / Журнал Действий (Eva-Hypervisor)

## Выполнено:
- [x] Прочитан и утвержден `eva_daemon_implementation_plan.md` с архитектором (Пользователем).
- [x] Зафиксированы ключевые технические решения:
  - Vector DB: встроенная `hnsw_rs`.
  - Веса: Предзагруженные локальные `.safetensors`.
  - CUDA: Включена в NixOS flake с самого старта для работы с VRAM и WSJF-штрафами контекста.
- [x] Удален старый монолит: папки `eva-daemon`, `eva-cli`, `docs` и Ollama `Modelfile`.
- [x] Отредактирован `flake.nix`: добавлена переменная `cudaSupport = true` и библиотеки `cudaPackages.cudatoolkit`, `cudaPackages.cudnn`.
- [x] Инициализирован новый Rust Cargo Workspace.
- [x] Описаны структуры Shared Memory (`STATUS_FLAG`, Control Block) в `shared-ipc`.

## В планах (Backlog):
- [ ] Интегрировать загрузку железа в `core` (базовые тесты для WSJF).
- [ ] Начать проброс моделей из `models.md` в `worker-candle` (загрузка весов Gemma 3, Aya, Phi-4).
