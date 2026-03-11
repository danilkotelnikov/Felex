# Felex v1.0.1 Technical Report

Дата обновления: 2026-03-11

## Что изменено

В версии `1.0.1` обновлён основной контур оптимизации рационов:

- пустой рацион теперь получает стартовый набор кормов через auto-populate;
- перед балансировкой выполняется screening текущего набора кормов с рекомендациями по добавлению ингредиентов;
- `BalanceNutrients` переведён на многоуровневую приоритетную балансировку без старого штрафа за изменение количеств;
- действительно невыполнимые сценарии возвращают `Infeasible`, а не скрываются как решение без изменений;
- встроенный агент сохраняет thinking mode (`think: true`), применяет реальный `num_ctx` из интерфейса и восстанавливается после локальных ошибок Ollama.

## Бенчмарк 2026-03-11

Метод:
- `cargo run --quiet --bin perf_probe`
- база кормов: 300 записей
- тестовая машина: AMD Ryzen 7 8845HS, 16 GB DDR5, NVIDIA RTX 4060 Laptop 8 GB VRAM, Windows 11 Pro

### Расчёт питательности

| Сценарий | Среднее время | Вызовов/с |
| --- | --- | --- |
| 8 ингредиентов | 0.000385 ms | 2.60 млн |
| 24 ингредиента | 0.001108 ms | 0.90 млн |
| 64 ингредиента | 0.002912 ms | 0.34 млн |

### Оптимизатор

| Сценарий | Режим | Среднее время | Финальный статус |
| --- | --- | --- | --- |
| dairy_low_fiber | minimize_cost | 0.214929 ms | Infeasible |
| dairy_low_fiber | balance | 0.978980 ms | Infeasible |
| dairy_low_fiber | fixed | 1.016516 ms | Infeasible |
| grower_pig_lysine_deficit | minimize_cost | 0.177419 ms | Infeasible |
| grower_pig_lysine_deficit | balance | 0.688114 ms | Infeasible |
| grower_pig_lysine_deficit | fixed | 0.715599 ms | Infeasible |
| layer_low_calcium | minimize_cost | 0.075507 ms | Optimal |
| layer_low_calcium | balance | 0.477134 ms | Optimal |
| layer_low_calcium | fixed | 0.469067 ms | Optimal |

### Рабочие этапы

| Этап | Сценарий | Среднее время |
| --- | --- | --- |
| auto_populate_preview | starter_dairy | 50.688338 ms |
| auto_populate_preview | starter_swine | 31.849788 ms |
| auto_populate_preview | starter_layer | 32.868358 ms |
| screen | dairy_low_fiber | 24.946853 ms |
| screen | grower_pig_lysine_deficit | 24.694417 ms |
| screen | layer_low_calcium | 32.404530 ms |

### ИИ-агент

| Модель | Сценарий | Контекст | Время | Итог |
| --- | --- | --- | --- | --- |
| qwen3.5:4b | dairy_low_fiber | 8192 | 84.23 s | развёрнутый ответ |
| qwen3.5:4b | grower_pig_lysine_deficit | 8192 | 16.75 s | короткое вступление |
| qwen3.5:4b | layer_low_calcium | 8192 | 54.19 s | развёрнутый ответ |
| qwen3.5:9b | dairy_low_fiber | 4096 | 86.26 s | развёрнутый ответ |
| qwen3.5:9b | grower_pig_lysine_deficit | 4096 | 78.57 s | развёрнутый ответ |
| qwen3.5:9b | layer_low_calcium | 4096 | 74.71 s | развёрнутый ответ |

## Выводы

- Сам решатель остаётся очень быстрым; проблемой в сложных сценариях является выполнимость набора кормов, а не скорость вычислений.
- Auto-populate и screening стоят десятки миллисекунд, что приемлемо для интерактивной работы.
- 4B-модель быстрее, но менее стабильна по качеству аналитического ответа.
- 9B-модель в текущем прогоне дала 3 развёрнутых ответа из 3, но остаётся существенно медленнее.

Исходные численные данные сохранены в `docs/benchmarks/2026-03-11-perf_probe.json` внутри исходного проекта.
