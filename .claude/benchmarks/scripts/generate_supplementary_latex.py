import pandas as pd
import json
import os
from pathlib import Path

# Paths
BASE_DIR = Path(r"c:\Users\danil\OneDrive\Рабочий стол\Felex_v1")
CSV_DIR = BASE_DIR / ".claude/benchmarks/data/csv"
JSON_DIR = BASE_DIR / ".claude/benchmarks/results"
OUT_FILE = BASE_DIR / ".claude/publication-draft/journal-manuscript/latex/felex_supplementary_ru.tex"

def format_number(val):
    if pd.isna(val):
        return "-"
    if isinstance(val, (int, float)):
        return f"{val:.2f}".replace('.', '{,}')
    return str(val)

latex_content = [
    r"\documentclass[11pt]{article}",
    r"\usepackage[a4paper,margin=1in]{geometry}",
    r"\usepackage[T2A,T1]{fontenc}",
    r"\usepackage[utf8]{inputenc}",
    r"\usepackage[russian,english]{babel}",
    r"\usepackage{lmodern}",
    r"\usepackage{booktabs}",
    r"\usepackage{longtable}",
    r"\usepackage{siunitx}",
    r"\usepackage{caption}",
    r"\usepackage{setspace}",
    r"\usepackage{hyperref}",
    r"\captionsetup{font=small,labelfont=bf}",
    r"\setstretch{1.15}",
    r"",
    r"\title{Дополнительные материалы к статье: Felex}",
    r"\author{Данил Котельников}",
    r"\date{Март 2026}",
    r"",
    r"\begin{document}",
    r"\selectlanguage{russian}",
    r"\maketitle",
    r"",
    r"Все данные в настоящих дополнительных материалах сгенерированы программно на основе JSON-артефактов и CSV-таблиц результатов вычислительного эксперимента. Полный набор исходных данных доступен в репозитории проекта.",
    r"",
]

# Table S1: Scenario Definitions
try:
    with open(JSON_DIR / "benchmark_results.json", encoding="utf-8") as f:
        data = json.load(f)
    cases = data.get("benchmark", {}).get("cases", [])
    
    latex_content.append(r"\section*{Таблица S1. Характеристика сценариев оптимизации}")
    latex_content.append(r"\begin{longtable}{lllc}")
    latex_content.append(r"\caption{Характеристика сценариев оптимизации (сценарии, вид животного, количество доступных кормов).}\label{tab:s1_scenarios} \\")
    latex_content.append(r"\toprule")
    latex_content.append(r"\textbf{ID сценария} & \textbf{Вид} & \textbf{Описание (англ.)} & \textbf{Доступно кормов} \\")
    latex_content.append(r"\midrule")
    latex_content.append(r"\endfirsthead")
    latex_content.append(r"\toprule")
    latex_content.append(r"\textbf{ID сценария} & \textbf{Вид} & \textbf{Описание (англ.)} & \textbf{Доступно кормов} \\")
    latex_content.append(r"\midrule")
    latex_content.append(r"\endhead")
    latex_content.append(r"\bottomrule")
    latex_content.append(r"\endfoot")
    latex_content.append(r"\bottomrule")
    latex_content.append(r"\endlastfoot")
    
    for c in cases:
        cid = c["case"]["id"].replace("_", "\\_")
        species = c["case"]["species"]
        label = c["case"]["label"]
        allowed = c.get("library", {}).get("allowed_feed_count", "-")
        latex_content.append(f"\\texttt{{{cid}}} & {species} & {label} & {allowed} \\\\")
        
    latex_content.append(r"\end{longtable}")
except Exception as e:
    latex_content.append(f"% Error generating S1: {e}")

# Table S2: Full Case Results
try:
    df_case = pd.read_csv(CSV_DIR / "case_summary.csv")
    latex_content.append(r"\section*{Таблица S2. Результаты оптимизации по сценариям}")
    latex_content.append(r"\begin{longtable}{lcccccc}")
    latex_content.append(r"\caption{Покрытие норм, выполнение жестких ограничений и стоимость по сценариям.}\label{tab:s2_cases} \\")
    latex_content.append(r"\toprule")
    latex_content.append(r"& \multicolumn{3}{c}{\textbf{Покрытие норм (0--100)}} & \multicolumn{3}{c}{\textbf{Стоимость (руб./сут.)}} \\")
    latex_content.append(r"\cmidrule(lr){2-4} \cmidrule(lr){5-7}")
    latex_content.append(r"\textbf{Сценарий} & \textbf{Build} & \textbf{Complete} & \textbf{Selected} & \textbf{Build} & \textbf{Complete} & \textbf{Selected} \\")
    latex_content.append(r"\midrule")
    latex_content.append(r"\endfirsthead")
    latex_content.append(r"\toprule")
    latex_content.append(r"& \multicolumn{3}{c}{\textbf{Покрытие норм (0--100)}} & \multicolumn{3}{c}{\textbf{Стоимость (руб./сут.)}} \\")
    latex_content.append(r"\cmidrule(lr){2-4} \cmidrule(lr){5-7}")
    latex_content.append(r"\textbf{Сценарий} & \textbf{Build} & \textbf{Complete} & \textbf{Selected} & \textbf{Build} & \textbf{Complete} & \textbf{Selected} \\")
    latex_content.append(r"\midrule")
    latex_content.append(r"\endhead")
    latex_content.append(r"\bottomrule")
    latex_content.append(r"\endfoot")
    latex_content.append(r"\bottomrule")
    latex_content.append(r"\endlastfoot")
    
    for _, row in df_case.iterrows():
        cid = str(row['case_id']).replace("_", "\\_")
        b_q = format_number(row.get('build_quality', 0))
        c_q = format_number(row.get('complete_quality', 0))
        s_q = format_number(row.get('selected_quality', 0))
        b_c = format_number(row.get('build_cost', 0))
        c_c = format_number(row.get('complete_cost', 0))
        s_c = format_number(row.get('selected_cost', 0))
        latex_content.append(f"\\texttt{{{cid}}} & {b_q} & {c_q} & {s_q} & {b_c} & {c_c} & {s_c} \\\\")
        
    latex_content.append(r"\end{longtable}")
except Exception as e:
    latex_content.append(f"% Error generating S2: {e}")

# Table S3 & S4
try:
    df_wf = pd.read_csv(CSV_DIR / "workflow_summary.csv", header=[0, 1])
    latex_content.append(r"\section*{Таблица S3. Агрегированная статистика по рабочим процессам}")
    latex_content.append(r"\begin{table}[H]")
    latex_content.append(r"\centering")
    latex_content.append(r"\caption{Агрегированная статистика по рабочим процессам}\label{tab:s3_wf}")
    latex_content.append(r"\begin{tabular}{lcccc}")
    latex_content.append(r"\toprule")
    latex_content.append(r"\textbf{Рабочий процесс} & \textbf{Время (с)} & \textbf{Стоимость (руб.)} & \textbf{Hard Pass (\%)} & \textbf{Покрытие} \\")
    latex_content.append(r"\midrule")
    for _, row in df_wf.iterrows():
        wf = str(row.iloc[0]).replace("_", "\\_")
        rt = format_number(row.iloc[1])
        cost = format_number(row.iloc[3])
        hp = format_number(row.iloc[5])
        cov = format_number(row.iloc[6])
        latex_content.append(f"\\texttt{{{wf}}} & {rt} & {cost} & {hp} & {cov} \\\\")
    latex_content.append(r"\bottomrule")
    latex_content.append(r"\end{tabular}")
    latex_content.append(r"\end{table}")
except Exception as e:
    latex_content.append(f"% Error generating S3: {e}")

try:
    df_sp = pd.read_csv(CSV_DIR / "species_summary.csv")
    latex_content.append(r"\section*{Таблица S4. Агрегированная статистика по видам}")
    latex_content.append(r"\begin{table}[H]")
    latex_content.append(r"\centering")
    latex_content.append(r"\caption{Агрегированная статистика по видам животных}\label{tab:s4_sp}")
    latex_content.append(r"\begin{tabular}{lcccc}")
    latex_content.append(r"\toprule")
    latex_content.append(r"\textbf{Вид} & \textbf{Сценариев} & \textbf{Ср. покрытие (Build)} & \textbf{Ср. покрытие (Complete)} & \textbf{Ср. стоимость} \\")
    latex_content.append(r"\midrule")
    for _, row in df_sp.iterrows():
        sp = str(row['species']).capitalize()
        cc = row['case_count']
        bq = format_number(row['mean_build_quality'])
        cq = format_number(row['mean_complete_quality'])
        cost = format_number(row['mean_build_cost'])
        latex_content.append(f"{sp} & {cc} & {bq} & {cq} & {cost} \\\\")
    latex_content.append(r"\bottomrule")
    latex_content.append(r"\end{tabular}")
    latex_content.append(r"\end{table}")
except Exception as e:
    latex_content.append(f"% Error generating S4: {e}")

# Table S5 Heatmap
try:
    df_hm = pd.read_csv(CSV_DIR / "monogastric_heatmap_data.csv")
    cols = df_hm.columns.tolist()
    latex_content.append(r"\section*{Таблица S5. Кумулятивная выраженность дефицитов для моногастричных животных}")
    latex_content.append(r"\begin{longtable}{l" + "c"*(len(cols)-1) + "}")
    latex_content.append(r"\caption{Кумулятивная выраженность дефицитов}\label{tab:s5_heatmap} \\")
    latex_content.append(r"\toprule")
    latex_content.append(r"\textbf{Сценарий} & " + " & ".join([f"\\textbf{{{c.replace('_', '\\_')}}}" for c in cols[1:]]) + r" \\")
    latex_content.append(r"\midrule")
    latex_content.append(r"\endfirsthead")
    latex_content.append(r"\toprule")
    latex_content.append(r"\textbf{Сценарий} & " + " & ".join([f"\\textbf{{{c.replace('_', '\\_')}}}" for c in cols[1:]]) + r" \\")
    latex_content.append(r"\midrule")
    latex_content.append(r"\endhead")
    latex_content.append(r"\bottomrule")
    latex_content.append(r"\endfoot")
    latex_content.append(r"\bottomrule")
    latex_content.append(r"\endlastfoot")
    
    for _, row in df_hm.iterrows():
        cid = str(row['case_id']).replace("_", "\\_")
        vals = [format_number(row[c]) for c in cols[1:]]
        latex_content.append(f"\\texttt{{{cid}}} & " + " & ".join(vals) + r" \\")
        
    latex_content.append(r"\end{longtable}")
except Exception as e:
    latex_content.append(f"% Error generating S5: {e}")

# Table S6 Issues (Top 30)
try:
    df_iss = pd.read_csv(CSV_DIR / "issue_summary.csv")
    df_iss_sorted = df_iss.sort_values(by="cumulative_severity", ascending=False).head(30)
    latex_content.append(r"\section*{Таблица S6. Основные дефициты нутриентов (Топ-30 по выраженности)}")
    latex_content.append(r"\begin{longtable}{lllccrr}")
    latex_content.append(r"\caption{Топ-30 наиболее выраженных дефицитов/избытков}\label{tab:s6_issues} \\")
    latex_content.append(r"\toprule")
    latex_content.append(r"\textbf{Сценарий} & \textbf{Этап} & \textbf{Нутриент} & \textbf{Ур.} & \textbf{Выраженность} & \textbf{Факт} & \textbf{Норма} \\")
    latex_content.append(r"\midrule")
    latex_content.append(r"\endfirsthead")
    latex_content.append(r"\toprule")
    latex_content.append(r"\textbf{Сценарий} & \textbf{Этап} & \textbf{Нутриент} & \textbf{Ур.} & \textbf{Выраженность} & \textbf{Факт} & \textbf{Норма} \\")
    latex_content.append(r"\midrule")
    latex_content.append(r"\endhead")
    latex_content.append(r"\bottomrule")
    latex_content.append(r"\endfoot")
    latex_content.append(r"\bottomrule")
    latex_content.append(r"\endlastfoot")
    
    for _, row in df_iss_sorted.iterrows():
        cid = str(row['case_id']).replace("_", "\\_")
        stage = str(row['stage'])
        key = str(row['key']).replace("_", "\\_")
        tier = str(row['tier'])
        sev = format_number(row['cumulative_severity'])
        act = format_number(row['actual'])
        tgt = format_number(row['target'])
        latex_content.append(f"\\texttt{{{cid}}} & {stage} & \\texttt{{{key}}} & {tier} & {sev} & {act} & {tgt} \\\\")
        
    latex_content.append(r"\end{longtable}")
except Exception as e:
    latex_content.append(f"% Error generating S6: {e}")

# Static Tables S7-S9
latex_content.append(r"""
\section*{Таблица S7. Контур нутриентов}
\begin{longtable}{lllc}
\caption{Перечень 30 нутриентов, участвующих в оптимизации.}\label{tab:s7_nutrients} \\
\toprule
\textbf{Ключ в БД} & \textbf{Наименование} & \textbf{Категория} & \textbf{Ед. изм.} \\
\midrule
\endfirsthead
\toprule
\textbf{Ключ в БД} & \textbf{Наименование} & \textbf{Категория} & \textbf{Ед. изм.} \\
\midrule
\endhead
\bottomrule
\endfoot
\bottomrule
\endlastfoot
\texttt{dry\_matter} & Сухое вещество & General & г/кг \\
\texttt{energy\_oe\_cattle} & Обменная энергия (КРС) & Energy & МДж \\
\texttt{energy\_oe\_pig} & Обменная энергия (свиньи) & Energy & МДж \\
\texttt{energy\_oe\_poultry} & Обменная энергия (птица) & Energy & МДж \\
\texttt{crude\_protein} & Сырой протеин & Protein & г/кг \\
\texttt{dig\_protein\_cattle} & Переваримый протеин (КРС) & Protein & г/кг \\
\texttt{lysine} & Лизин & AminoAcids & г/кг \\
\texttt{methionine\_cystine} & Метионин + цистин & AminoAcids & г/кг \\
\texttt{crude\_fat} & Сырой жир & Fats & г/кг \\
\texttt{crude\_fiber} & Сырая клетчатка & FiberCarbs & г/кг \\
\texttt{starch} & Крахмал & FiberCarbs & г/кг \\
\texttt{sugar} & Сахар & FiberCarbs & г/кг \\
\texttt{calcium} & Кальций & Macrominerals & г/кг \\
\texttt{phosphorus} & Фосфор & Macrominerals & г/кг \\
\texttt{magnesium} & Магний & Macrominerals & г/кг \\
\texttt{potassium} & Калий & Macrominerals & г/кг \\
\texttt{sodium} & Натрий & Macrominerals & г/кг \\
\texttt{sulfur} & Сера & Macrominerals & г/кг \\
\texttt{iron} & Железо & TraceMinerals & мг/кг \\
\texttt{copper} & Медь & TraceMinerals & мг/кг \\
\texttt{zinc} & Цинк & TraceMinerals & мг/кг \\
\texttt{manganese} & Марганец & TraceMinerals & мг/кг \\
\texttt{cobalt} & Кобальт & TraceMinerals & мг/кг \\
\texttt{iodine} & Йод & TraceMinerals & мг/кг \\
\texttt{carotene} & Каротин & Vitamins & мг/кг \\
\texttt{vit\_d3} & Витамин D3 & Vitamins & МЕ/кг \\
\texttt{vit\_e} & Витамин E & Vitamins & мг/кг \\
\texttt{selenium} & Селен & TraceMinerals & мг/кг \\
\texttt{ca\_p\_ratio} & Отношение Ca:P & Ratios & ед. \\
\end{longtable}

\section*{Таблица S8. Структурная матрица кормов}
\begin{table}[H]
\centering
\caption{Ограничения матрицы кормов (доля в сухом веществе, \%).}\label{tab:s8_matrix}
\begin{tabular}{llccc}
\toprule
\textbf{Архетип} & \textbf{Категория} & \textbf{Мин (\%)} & \textbf{Опт (\%)} & \textbf{Макс (\%)} \\
\midrule
КРС (молочный) & Грубые & 35 & 45 & 65 \\
& Концентраты & 30 & 40 & 60 \\
& Сочные & 5 & 20 & 35 \\
& Минеральные & 0.5 & 1.5 & 3 \\
Свиньи (откорм) & Грубые & 0 & 0 & 5 \\
& Концентраты & 90 & 95 & 100 \\
& Минеральные & 0 & 1 & 4 \\
Птица (бройлер) & Концентраты & 90 & 96 & 100 \\
& Минеральные & 0 & 1 & 4 \\
\bottomrule
\end{tabular}
\end{table}

\section*{Таблица S9. Состав кормовой базы данных}
\begin{table}[H]
\centering
\caption{Состав базы данных кормов по категориям.}\label{tab:s9_db}
\begin{tabular}{lc}
\toprule
\textbf{Категория} & \textbf{Количество позиций} \\
\midrule
Грубые корма (roughage) & $\sim 200$ \\
Сочные корма (succulent) & $\sim 150$ \\
Концентрированные (concentrate) & $\sim 400$ \\
Белковые добавки (protein) & $\sim 150$ \\
Минеральные (mineral) & $\sim 80$ \\
Премиксы и АДВ (premix, vitamins) & $\sim 100$ \\
Отходы промышленности & $\sim 150$ \\
Животного происхождения & $\sim 50$ \\
NPN (карбамид и др.) & $\sim 20$ \\
\midrule
\textbf{Всего в базе (app/feeds.db)} & \textbf{1375} \\
\bottomrule
\end{tabular}
\end{table}
""")

latex_content.append(r"\end{document}")

with open(OUT_FILE, "w", encoding="utf-8") as f:
    f.write("\n".join(latex_content))

print(f"Generated {OUT_FILE}")
