#!/usr/bin/env bash

# ==============================================================================
# Eva Hypervisor - Model Downloader
# ==============================================================================
# Скрипт автоматической загрузки весов моделей (.safetensors и .json).
# Поддерживает кастомный путь сохранения и валидацию ошибок HF.
# ==============================================================================

# Строгий режим обработки ошибок и пайпов
set -o pipefail

# Цветовые коды для вывода
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Включаем Rust-ускоритель для максимальной утилизации канала
export HF_HUB_ENABLE_HF_TRANSFER=1
export HF_HUB_DISABLE_TELEMETRY=1

DEFAULT_TARGET_DIR="$HOME/.eva/models"

echo -e "${CYAN}======================================================${NC}"
echo -e "${CYAN}   Инициализация загрузки моделей для Eva Hypervisor  ${NC}"
echo -e "${CYAN}======================================================${NC}"

# Интерактивный запрос пути с дефолтным значением
echo -e "${CYAN}Укажите директорию для сохранения весов моделей.${NC}"
read -r -p "Путь [Enter для ~/.eva/models]: " USER_INPUT_DIR

if [ -z "$USER_INPUT_DIR" ]; then
    TARGET_DIR="$DEFAULT_TARGET_DIR"
else
    # Раскрываем тильду (~), если пользователь вписал путь вручную через неё
    TARGET_DIR="${USER_INPUT_DIR/#\~/$HOME}"
fi

echo -e "${GREEN}Используется директория: ${TARGET_DIR}${NC}\n"

echo -e "${YELLOW}ВНИМАНИЕ:${NC}"
echo -e "1. Убедитесь, что вы авторизованы в Hugging Face:"
echo -e "   Выполните: ${GREEN}nix shell nixpkgs#python3Packages.huggingface-hub -c hf auth login${NC}"
echo -e "2. Модели от Google, Cohere и Mistral требуют принятия лицензии!"
echo -e "   Вам нужно зайти на страницу модели на huggingface.co и нажать 'Acknowledge license'."
echo -e "${CYAN}======================================================${NC}\n"

mkdir -p "$TARGET_DIR"

# Массив всех моделей проекта
MODELS=(
    "google/gemma-3-1b-it"                 # Узел Зеро (Ingress Router)
    "CohereForAI/aya-expanse-8b"           # Context Architect (Мультиязычный RAG)
    "microsoft/phi-4"                      # Pipeline Architect (Reasoning/Графы)
    "mistralai/Ministral-8B-Instruct-2410"         # Worker (Light - Быстрые задачи)
    "deepseek-ai/DeepSeek-Coder-V2-Lite-Instruct"  # Worker (Heavy - MoE, Кодинг)
    "facebook/nllb-200-distilled-1.3B"             # Worker (Универсальный Переводчик)
)

for MODEL in "${MODELS[@]}"; do
    MODEL_NAME=$(basename "$MODEL")
    echo -e "${CYAN}>>> Проверка и загрузка: ${NC}${MODEL}"
    
    TMP_LOG=$(mktemp)

    # Запускаем загрузку и стримим вывод в логи и на экран
    nix shell nixpkgs#python3Packages.huggingface-hub nixpkgs#python3Packages.hf-transfer -c \
        hf download "$MODEL" \
        --local-dir "$TARGET_DIR/$MODEL_NAME" \
        --include "*.safetensors" \
        --include "*.json" 2>&1 | tee "$TMP_LOG"
    
    EXIT_CODE=${PIPESTATUS[0]}

    if [ $EXIT_CODE -ne 0 ]; then
        echo -e "\n${RED}✘ Ошибка при загрузке модели: $MODEL${NC}"
        
        # Парсинг критических ошибок
        if grep -qiE "401|unauthorized" "$TMP_LOG"; then
            echo -e "${YELLOW}Причина:${NC} Отсутствует или недействителен токен авторизации."
            echo -e "${YELLOW}Решение:${NC} Выполните команду 'hf auth login' и вставьте ваш токен (Read) с сайта Hugging Face."
        elif grep -qiE "403|gated|access denied" "$TMP_LOG"; then
            echo -e "${YELLOW}Причина:${NC} Доступ запрещен (Gated Repository)."
            echo -e "${YELLOW}Решение:${NC} Перейдите по ссылке https://huggingface.co/$MODEL, авторизуйтесь и примите лицензионное соглашение (кнопка 'Acknowledge license')."
        elif grep -qiE "404|not found" "$TMP_LOG"; then
            echo -e "${YELLOW}Причина:${NC} Репозиторий не найден или файлы *.safetensors отсутствуют."
        else
            echo -e "${YELLOW}Причина:${NC} Сетевой сбой или неизвестная ошибка (Код: $EXIT_CODE). Попробуйте перезапустить скрипт."
        fi
        
        rm -f "$TMP_LOG"
        echo -e "${RED}Скрипт остановлен, чтобы вы могли устранить ошибку.${NC}"
        exit 1
    fi
    
    rm -f "$TMP_LOG"
    echo -e "${GREEN}✔ Модель $MODEL_NAME успешно синхронизирована!${NC}\n"
done

echo -e "${GREEN}======================================================${NC}"
echo -e "${GREEN}Все модели успешно загружены и готовы к инференсу!${NC}"
echo -e "Директория хранения: $TARGET_DIR"
echo -e "${GREEN}======================================================${NC}"
