import type { RationProject } from '@/types/ration-project';

function formatDate(): string {
  return new Date().toISOString().slice(0, 10);
}

function escapeCsv(value: string | number | null | undefined): string {
  const text = value == null ? '' : String(value);
  if (/[",\n;]/.test(text)) {
    return `"${text.replace(/"/g, '""')}"`;
  }
  return text;
}

function formatNum(value: number, digits = 2): string {
  return value.toLocaleString('ru-RU', {
    minimumFractionDigits: digits,
    maximumFractionDigits: digits,
  });
}

export function exportRationCSV(project: RationProject): string {
  const lines: string[] = [];

  lines.push(escapeCsv(project.name));
  lines.push(`Дата;${formatDate()}`);
  lines.push(
    `Животное;${escapeCsv(project.animalProperties.species)};${escapeCsv(project.animalProperties.productionType)};${escapeCsv(project.animalProperties.breed)}`
  );
  lines.push(`Масса;${formatNum(project.animalProperties.weight, 1)} кг`);
  lines.push(`Кол-во голов;${project.animalCount}`);
  lines.push('');

  lines.push('Состав рациона');
  lines.push('Корм;кг/сут на голову;кг/сут на группу;Зафиксирован');

  for (const item of project.items) {
    lines.push(
      [
        escapeCsv(item.feedName),
        formatNum(item.amountKg),
        formatNum(item.amountKg * project.animalCount),
        item.isLocked ? 'Да' : 'Нет',
      ].join(';')
    );
  }

  const totalKg = project.items.reduce((sum, i) => sum + i.amountKg, 0);
  lines.push(
    ['ИТОГО', formatNum(totalKg), formatNum(totalKg * project.animalCount), ''].join(';')
  );

  return lines.join('\n');
}

export function downloadRationCSV(project: RationProject, filename: string): void {
  const csv = exportRationCSV(project);
  const blob = new Blob(['\ufeff' + csv], { type: 'text/csv;charset=utf-8;' });
  const url = URL.createObjectURL(blob);
  const a = document.createElement('a');
  a.href = url;
  a.download = filename;
  document.body.appendChild(a);
  a.click();
  document.body.removeChild(a);
  URL.revokeObjectURL(url);
}

export function exportRationXLSX(project: RationProject, filename: string): void {
  // Build an HTML-based .xls file (same approach as existing export.ts)
  const rows = project.items.map(
    (item) =>
      `<tr><td>${item.feedName}</td><td>${formatNum(item.amountKg)}</td><td>${formatNum(item.amountKg * project.animalCount)}</td><td>${item.isLocked ? 'Да' : 'Нет'}</td></tr>`
  );

  const totalKg = project.items.reduce((sum, i) => sum + i.amountKg, 0);

  const propsRows = [
    `<tr><td>Вид</td><td>${project.animalProperties.species}</td></tr>`,
    `<tr><td>Тип продукции</td><td>${project.animalProperties.productionType}</td></tr>`,
    `<tr><td>Порода</td><td>${project.animalProperties.breed}</td></tr>`,
    `<tr><td>Пол</td><td>${project.animalProperties.sex}</td></tr>`,
    `<tr><td>Масса (кг)</td><td>${formatNum(project.animalProperties.weight, 1)}</td></tr>`,
    project.animalProperties.milkYieldKg != null
      ? `<tr><td>Удой (кг/сут)</td><td>${formatNum(project.animalProperties.milkYieldKg, 1)}</td></tr>`
      : '',
    project.animalProperties.dailyGainG != null
      ? `<tr><td>Привес (г/сут)</td><td>${project.animalProperties.dailyGainG}</td></tr>`
      : '',
    project.animalProperties.eggProduction != null
      ? `<tr><td>Яйценоскость (яиц/год)</td><td>${project.animalProperties.eggProduction}</td></tr>`
      : '',
    `<tr><td>Кол-во голов</td><td>${project.animalCount}</td></tr>`,
  ]
    .filter(Boolean)
    .join('');

  const html = `<html xmlns:o="urn:schemas-microsoft-com:office:office" xmlns:x="urn:schemas-microsoft-com:office:excel">
<head><meta charset="UTF-8"/></head>
<body>
<h2>${project.name}</h2>
<p>Дата: ${formatDate()}</p>

<h3>Рацион</h3>
<table border="1">
  <thead><tr><th>Корм</th><th>кг/сут (голова)</th><th>кг/сут (группа)</th><th>Фиксирован</th></tr></thead>
  <tbody>
    ${rows.join('\n')}
    <tr><td><b>ИТОГО</b></td><td><b>${formatNum(totalKg)}</b></td><td><b>${formatNum(totalKg * project.animalCount)}</b></td><td></td></tr>
  </tbody>
</table>

<h3>Параметры животного</h3>
<table border="1">
  <tbody>${propsRows}</tbody>
</table>
</body>
</html>`;

  const blob = new Blob(['\ufeff', html], { type: 'application/vnd.ms-excel;charset=utf-8' });
  const url = URL.createObjectURL(blob);
  const a = document.createElement('a');
  a.href = url;
  a.download = filename;
  document.body.appendChild(a);
  a.click();
  document.body.removeChild(a);
  URL.revokeObjectURL(url);
}
