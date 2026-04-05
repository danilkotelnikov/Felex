import Papa from 'papaparse';
import * as XLSX from 'xlsx';
import { invoke } from '@tauri-apps/api/core';
import toast from 'react-hot-toast';
import type { NormRange } from '@/types/nutrient';
import type { Feed } from '@/types/feed';
import type { NutrientSummary } from '@/types/ration';
import type { RationProject } from '@/types/ration-project';
import { isTauriApp } from '@/lib/desktop';
import { getFeedCategoryLabel } from '@/lib/feed-categories';
import { getFeedDisplayName } from '@/lib/feed-display';
import { getNutrientLabel, getNutrientMeta, getOrderedNutrientKeys } from '@/lib/nutrient-registry';
import { getManagedNormEntries, getNutrientDisplayActual, isManagedNutrientKey, nutrientDisplayUnit } from '@/lib/nutrient-display';
import { getNutrientStatus } from '@/lib/nutrient-status';
import { isNotificationEnabled } from '@/lib/preferences';
import { resolveAnimalGroupId, useRationStore } from '@/stores/rationStore';

export type ExportFontFamily = 'sans' | 'serif' | 'mono';
export type ExportAppearance = 'standard' | 'compact' | 'presentation';

export interface ExportPreferences {
  destinationDir: string;
  fileName: string;
  fontFamily: ExportFontFamily;
  appearance: ExportAppearance;
}

export interface ExportReportOptions extends Partial<ExportPreferences> {
  includeNutrients?: boolean;
  includeEconomics?: boolean;
  includeNormsComparison?: boolean;
  includeFeedDetails?: boolean;
  title?: string;
  notes?: string;
}

interface RationLikeItem {
  feed: Feed;
  amount_kg: number;
  is_locked: boolean;
}

interface ExportFeedRowPayload {
  name: string;
  category: string;
  amountKgPerHead: number;
  amountKgTotal: number;
  dryMatterPct: number | null;
  pricePerTon: number | null;
  costPerDayPerHead: number;
  costPerDayTotal: number;
  isLocked: boolean;
}

interface ExportNutrientRowPayload {
  label: string;
  actual: number;
  unit: string;
  norm?: { min?: number; target?: number; max?: number };
  status: string;
}

interface ExportReportPayload {
  title: string;
  subtitle?: string;
  notes: string;
  generatedAt: string;
  includeNutrients: boolean;
  includeEconomics: boolean;
  includeNormsComparison: boolean;
  includeFeedDetails: boolean;
  animalCount: number;
  items: ExportFeedRowPayload[];
  nutrientRows: ExportNutrientRowPayload[];
  totalCostPerHead: number;
  totalCost: number;
  totalKgPerHead: number;
  totalKg: number;
}

interface ExportRequestPayload {
  format: 'csv' | 'xlsx' | 'pdf';
  report: ExportReportPayload;
  options: {
    destinationDir?: string;
    fileName?: string;
    fontFamily?: ExportFontFamily;
    appearance?: ExportAppearance;
  };
}

interface ExportResponsePayload {
  path: string;
}

const EXPORT_PREFERENCES_KEY = 'felex_export_preferences';

export const DEFAULT_EXPORT_PREFERENCES: ExportPreferences = {
  destinationDir: '',
  fileName: '',
  fontFamily: 'sans',
  appearance: 'standard',
};

export function loadExportPreferences(): ExportPreferences {
  if (typeof window === 'undefined') {
    return DEFAULT_EXPORT_PREFERENCES;
  }

  try {
    const raw = localStorage.getItem(EXPORT_PREFERENCES_KEY);
    if (!raw) {
      return DEFAULT_EXPORT_PREFERENCES;
    }

    const parsed = JSON.parse(raw) as Partial<ExportPreferences>;
    return {
      ...DEFAULT_EXPORT_PREFERENCES,
      destinationDir: parsed.destinationDir ?? '',
      fileName: parsed.fileName ?? '',
      fontFamily: normalizeFontFamily(parsed.fontFamily),
      appearance: normalizeAppearance(parsed.appearance),
    };
  } catch {
    return DEFAULT_EXPORT_PREFERENCES;
  }
}

export function saveExportPreferences(preferences: ExportPreferences): void {
  if (typeof window === 'undefined') {
    return;
  }

  const normalized: ExportPreferences = {
    destinationDir: preferences.destinationDir ?? '',
    fileName: preferences.fileName ?? '',
    fontFamily: normalizeFontFamily(preferences.fontFamily),
    appearance: normalizeAppearance(preferences.appearance),
  };

  localStorage.setItem(EXPORT_PREFERENCES_KEY, JSON.stringify(normalized));
}

function normalizeFontFamily(value: unknown): ExportFontFamily {
  return value === 'serif' || value === 'mono' ? value : 'sans';
}

function normalizeAppearance(value: unknown): ExportAppearance {
  return value === 'compact' || value === 'presentation' ? value : 'standard';
}

function currentLanguage(): 'ru' | 'en' {
  if (typeof window === 'undefined') {
    return 'ru';
  }

  const stored = localStorage.getItem('i18nextLng') ?? navigator.language ?? 'ru';
  return stored.toLowerCase().startsWith('en') ? 'en' : 'ru';
}

function buildSubtitle(
  properties: {
    species: string;
    productionType: string;
    breed: string;
    weight: number;
  },
  animalCount: number,
  language: 'ru' | 'en',
): string {
  const speciesLabel = language === 'ru'
    ? ({ cattle: 'КРС', swine: 'Свиньи', poultry: 'Птица' }[properties.species] ?? properties.species)
    : ({ cattle: 'Cattle', swine: 'Swine', poultry: 'Poultry' }[properties.species] ?? properties.species);
  const productionLabel = language === 'ru'
    ? ({
        dairy: 'молочное направление',
        beef: 'мясное направление',
        fattening: 'откорм',
        breeding: 'воспроизводство',
        broiler: 'бройлеры',
        layer: 'несушки',
      }[properties.productionType] ?? properties.productionType)
    : ({
        dairy: 'dairy',
        beef: 'beef',
        fattening: 'fattening',
        breeding: 'breeding',
        broiler: 'broilers',
        layer: 'layers',
      }[properties.productionType] ?? properties.productionType);
  const weightUnit = language === 'ru' ? 'кг' : 'kg';
  const headsLabel = language === 'ru' ? 'гол.' : 'heads';

  return `${speciesLabel} / ${productionLabel} / ${properties.breed} / ${properties.weight} ${weightUnit} / ${animalCount} ${headsLabel}`;
}

function humanizeStatus(status: 'ok' | 'low' | 'high' | 'critical', language: 'ru' | 'en'): string {
  if (language === 'en') {
    return {
      ok: 'Within range',
      low: 'Below norm',
      high: 'Above norm',
      critical: 'Critical',
    }[status];
  }

  return {
    ok: 'В норме',
    low: 'Ниже нормы',
    high: 'Выше нормы',
    critical: 'Критично',
  }[status];
}

function buildExportFileName(title: string, preferredName: string | undefined, extension: string): string {
  const rawName = preferredName?.trim() || title.trim() || `ration_export.${extension}`;
  return rawName.toLowerCase().endsWith(`.${extension}`) ? rawName : `${rawName}.${extension}`;
}

function buildFeedRows(
  items: RationLikeItem[],
  animalCount: number,
  language: 'ru' | 'en',
): ExportFeedRowPayload[] {
  return items.map((item) => {
    const pricePerTon = item.feed.price_per_ton ?? null;
    const pricePerKg = pricePerTon ? pricePerTon / 1000 : 0;
    const costPerDayPerHead = item.amount_kg * pricePerKg;

    return {
      name: getFeedDisplayName(item.feed, language),
      category: getFeedCategoryLabel(item.feed.category ?? 'other', language),
      amountKgPerHead: item.amount_kg,
      amountKgTotal: item.amount_kg * animalCount,
      dryMatterPct: item.feed.dry_matter ?? null,
      pricePerTon,
      costPerDayPerHead,
      costPerDayTotal: costPerDayPerHead * animalCount,
      isLocked: item.is_locked,
    };
  });
}

function buildNutrientRows(
  nutrients: NutrientSummary | null,
  groupId: string,
  norms: Record<string, NormRange> | null | undefined,
  includeNormsComparison: boolean,
  language: 'ru' | 'en',
): ExportNutrientRowPayload[] {
  if (!nutrients) {
    return [];
  }

  const values = nutrients as unknown as Record<string, number | undefined>;
  const normMap = new Map(getManagedNormEntries(groupId, norms ?? {}));
  const nutrientKeys = norms
    ? Array.from(normMap.keys())
    : getOrderedNutrientKeys(Object.keys(values)).filter((key) =>
        isManagedNutrientKey(groupId, key) && Boolean(getNutrientMeta(key)),
      );

  return nutrientKeys
    .map((key) => {
      const actual = getNutrientDisplayActual(nutrients, groupId, key);
      if (!Number.isFinite(actual)) {
        return null;
      }

      const norm = normMap.get(key);
      if (!norm && Math.abs(actual as number) < 1e-9) {
        return null;
      }

      const status = norm
        ? humanizeStatus(getNutrientStatus(actual as number, norm.min, norm.max, norm.target), language)
        : (language === 'ru' ? 'Без нормы' : 'No reference');

      const row: ExportNutrientRowPayload = {
        label: getNutrientLabel(key, language),
        actual: actual as number,
        unit: nutrientDisplayUnit(groupId, key, language),
        norm: includeNormsComparison && norm
          ? { min: norm.min, target: norm.target, max: norm.max }
          : undefined,
        status,
      };

      return row;
    })
    .filter((row): row is ExportNutrientRowPayload => Boolean(row));
}

function buildCurrentRationReport(
  items: RationLikeItem[],
  nutrients: NutrientSummary | null,
  norms: Record<string, NormRange> | null | undefined,
  options: ExportReportOptions,
): ExportReportPayload {
  const language = currentLanguage();
  const state = useRationStore.getState();
  const animalCount = state.animalCount;
  const groupId = resolveAnimalGroupId(state.animalProperties);
  const feedRows = buildFeedRows(items, animalCount, language);
  const totalKgPerHead = feedRows.reduce((sum, row) => sum + row.amountKgPerHead, 0);
  const totalCostPerHead = feedRows.reduce((sum, row) => sum + row.costPerDayPerHead, 0);
  const includeNutrients = options.includeNutrients ?? true;
  const includeEconomics = options.includeEconomics ?? true;
  const includeNormsComparison = options.includeNormsComparison ?? true;
  const includeFeedDetails = options.includeFeedDetails ?? true;

  return {
    title: options.title?.trim() || state.currentProjectName || (language === 'ru' ? 'Рацион' : 'Ration'),
    subtitle: buildSubtitle(
      {
        species: state.animalProperties.species,
        productionType: state.animalProperties.productionType,
        breed: state.animalProperties.breed,
        weight: state.animalProperties.liveWeightKg,
      },
      animalCount,
      language,
    ),
    notes: options.notes?.trim() || '',
    generatedAt: new Date().toLocaleString(language === 'ru' ? 'ru-RU' : 'en-US'),
    includeNutrients,
    includeEconomics,
    includeNormsComparison,
    includeFeedDetails,
    animalCount,
    items: feedRows,
    nutrientRows: includeNutrients
      ? buildNutrientRows(nutrients, groupId, norms, includeNormsComparison, language)
      : [],
    totalCostPerHead,
    totalCost: totalCostPerHead * animalCount,
    totalKgPerHead,
    totalKg: totalKgPerHead * animalCount,
  };
}

function buildProjectReport(project: RationProject): ExportReportPayload {
  const language = currentLanguage();
  const items = project.items.map((item) => ({
    name: item.feedName,
    category: '',
    amountKgPerHead: item.amountKg,
    amountKgTotal: item.amountKg * project.animalCount,
    dryMatterPct: null,
    pricePerTon: null,
    costPerDayPerHead: 0,
    costPerDayTotal: 0,
    isLocked: item.isLocked,
  }));
  const totalKgPerHead = items.reduce((sum, row) => sum + row.amountKgPerHead, 0);

  return {
    title: project.name,
    subtitle: buildSubtitle(
      {
        species: project.animalProperties.species,
        productionType: project.animalProperties.productionType,
        breed: project.animalProperties.breed,
        weight: project.animalProperties.weight,
      },
      project.animalCount,
      language,
    ),
    notes: '',
    generatedAt: new Date().toLocaleString(language === 'ru' ? 'ru-RU' : 'en-US'),
    includeNutrients: false,
    includeEconomics: false,
    includeNormsComparison: false,
    includeFeedDetails: true,
    animalCount: project.animalCount,
    items,
    nutrientRows: [],
    totalCostPerHead: 0,
    totalCost: 0,
    totalKgPerHead,
    totalKg: totalKgPerHead * project.animalCount,
  };
}

async function saveDesktopExport(payload: ExportRequestPayload): Promise<ExportResponsePayload> {
  return invoke<ExportResponsePayload>('save_ration_export', { request: payload });
}

function formatCsvValue(value: string | number | null | undefined): string {
  if (value == null) {
    return '';
  }
  return typeof value === 'number' ? String(value) : value;
}

function renderCsvText(report: ExportReportPayload): string {
  const lines: Array<Array<string>> = [
    [report.title],
    ...(report.subtitle ? [[report.subtitle]] : []),
    ['Дата', report.generatedAt],
    ['Число голов', String(report.animalCount)],
    [],
    ['Состав рациона'],
    report.includeFeedDetails
      ? ['Корм', 'Категория', 'кг/гол./сут', 'кг/группу/сут', 'СВ %', '₽/т', '₽/гол./сут', '₽/группу/сут', 'Фикс.']
      : ['Корм', 'Категория', 'кг/гол./сут', 'кг/группу/сут', '₽/гол./сут', '₽/группу/сут'],
  ];

  for (const item of report.items) {
    lines.push(
      report.includeFeedDetails
        ? [
            item.name,
            item.category,
            formatCsvValue(item.amountKgPerHead),
            formatCsvValue(item.amountKgTotal),
            formatCsvValue(item.dryMatterPct),
            formatCsvValue(item.pricePerTon),
            formatCsvValue(item.costPerDayPerHead),
            formatCsvValue(item.costPerDayTotal),
            item.isLocked ? 'Да' : 'Нет',
          ]
        : [
            item.name,
            item.category,
            formatCsvValue(item.amountKgPerHead),
            formatCsvValue(item.amountKgTotal),
            formatCsvValue(item.costPerDayPerHead),
            formatCsvValue(item.costPerDayTotal),
          ],
    );
  }

  lines.push(
    report.includeFeedDetails
      ? ['Итого', '', String(report.totalKgPerHead), String(report.totalKg), '', '', String(report.totalCostPerHead), String(report.totalCost), '']
      : ['Итого', '', String(report.totalKgPerHead), String(report.totalKg), String(report.totalCostPerHead), String(report.totalCost)],
  );

  if (report.includeNutrients && report.nutrientRows.length > 0) {
    lines.push([]);
    lines.push(['Питательность']);
    lines.push(
      report.includeNormsComparison
        ? ['Показатель', 'Факт', 'Ед.', 'Мин.', 'Цель', 'Макс.', 'Статус']
        : ['Показатель', 'Факт', 'Ед.', 'Статус'],
    );

    for (const nutrient of report.nutrientRows) {
      lines.push(
        report.includeNormsComparison
          ? [
              nutrient.label,
              String(nutrient.actual),
              nutrient.unit,
              nutrient.norm?.min != null ? String(nutrient.norm.min) : '',
              nutrient.norm?.target != null ? String(nutrient.norm.target) : '',
              nutrient.norm?.max != null ? String(nutrient.norm.max) : '',
              nutrient.status,
            ]
          : [nutrient.label, String(nutrient.actual), nutrient.unit, nutrient.status],
      );
    }
  }

  return Papa.unparse(lines);
}

function saveBrowserFile(filename: string, blob: Blob): void {
  const url = URL.createObjectURL(blob);
  const link = document.createElement('a');
  link.href = url;
  link.download = filename;
  document.body.appendChild(link);
  link.click();
  document.body.removeChild(link);
  URL.revokeObjectURL(url);
}

function saveBrowserCsv(report: ExportReportPayload, fileName: string): void {
  const csv = renderCsvText(report);
  saveBrowserFile(fileName, new Blob(['\ufeff' + csv], { type: 'text/csv;charset=utf-8;' }));
}

function saveBrowserXlsx(report: ExportReportPayload, fileName: string): void {
  const rationHeader = report.includeFeedDetails
    ? ['Корм', 'Категория', 'кг/гол./сут', 'кг/группу/сут', 'СВ %', '₽/т', '₽/гол./сут', '₽/группу/сут', 'Фикс.']
    : ['Корм', 'Категория', 'кг/гол./сут', 'кг/группу/сут', '₽/гол./сут', '₽/группу/сут'];
  const workbook = XLSX.utils.book_new();
  const rationSheet = XLSX.utils.aoa_to_sheet([
    [report.title],
    ...(report.subtitle ? [[report.subtitle]] : []),
    ['Дата', report.generatedAt],
    ['Число голов', report.animalCount],
    [],
    rationHeader,
    ...report.items.map((item) =>
      report.includeFeedDetails
        ? [
            item.name,
            item.category,
            item.amountKgPerHead,
            item.amountKgTotal,
            item.dryMatterPct ?? '',
            item.pricePerTon ?? '',
            item.costPerDayPerHead,
            item.costPerDayTotal,
            item.isLocked ? 'Да' : 'Нет',
          ]
        : [
            item.name,
            item.category,
            item.amountKgPerHead,
            item.amountKgTotal,
            item.costPerDayPerHead,
            item.costPerDayTotal,
          ],
    ),
  ]);
  XLSX.utils.book_append_sheet(workbook, rationSheet, 'Рацион');

  if (report.includeNutrients && report.nutrientRows.length > 0) {
    const nutrientSheet = XLSX.utils.aoa_to_sheet([
      report.includeNormsComparison
        ? ['Показатель', 'Факт', 'Ед.', 'Мин.', 'Цель', 'Макс.', 'Статус']
        : ['Показатель', 'Факт', 'Ед.', 'Статус'],
      ...report.nutrientRows.map((item) =>
        report.includeNormsComparison
          ? [
              item.label,
              item.actual,
              item.unit,
              item.norm?.min ?? '',
              item.norm?.target ?? '',
              item.norm?.max ?? '',
              item.status,
            ]
          : [item.label, item.actual, item.unit, item.status],
      ),
    ]);
    XLSX.utils.book_append_sheet(workbook, nutrientSheet, 'Питательность');
  }

  const data = XLSX.write(workbook, { bookType: 'xlsx', type: 'array' });
  saveBrowserFile(fileName, new Blob([data], { type: 'application/vnd.openxmlformats-officedocument.spreadsheetml.sheet' }));
}

async function runExport(payload: ExportRequestPayload): Promise<void> {
  if (isTauriApp) {
    const result = await saveDesktopExport(payload);
    if (isNotificationEnabled('exportReady')) {
      toast.success(currentLanguage() === 'ru' ? `Файл сохранен: ${result.path}` : `File saved: ${result.path}`);
    }
    return;
  }

  const filename = buildExportFileName(payload.report.title, payload.options.fileName, payload.format);
  if (payload.format === 'csv') {
    saveBrowserCsv(payload.report, filename);
  } else if (payload.format === 'xlsx') {
    saveBrowserXlsx(payload.report, filename);
  } else {
    throw new Error(currentLanguage() === 'ru' ? 'PDF экспорт доступен в десктопной версии.' : 'PDF export is available in the desktop build.');
  }

  if (isNotificationEnabled('exportReady')) {
    toast.success(currentLanguage() === 'ru' ? `Файл сохранен: ${filename}` : `File saved: ${filename}`);
  }
}

async function runExportWithFeedback(payload: ExportRequestPayload): Promise<void> {
  try {
    await runExport(payload);
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    toast.error(message);
  }
}

function mergePreferences(options?: Partial<ExportPreferences>): ExportPreferences {
  const stored = loadExportPreferences();
  return {
    destinationDir: options?.destinationDir ?? stored.destinationDir,
    fileName: options?.fileName ?? stored.fileName,
    fontFamily: normalizeFontFamily(options?.fontFamily ?? stored.fontFamily),
    appearance: normalizeAppearance(options?.appearance ?? stored.appearance),
  };
}

export async function exportCsv(
  items: RationLikeItem[],
  nutrients: NutrientSummary | null,
  options: ExportReportOptions = {},
): Promise<void> {
  const preferences = mergePreferences(options);
  const report = buildCurrentRationReport(items, nutrients, null, {
    ...options,
    ...preferences,
  });

  await runExportWithFeedback({
    format: 'csv',
    report,
    options: {
      destinationDir: preferences.destinationDir || undefined,
      fileName: preferences.fileName || options.title || report.title,
      fontFamily: preferences.fontFamily,
      appearance: preferences.appearance,
    },
  });
}

export async function exportPdf(
  items: RationLikeItem[],
  nutrients: NutrientSummary | null,
  norms: Record<string, NormRange> | null | undefined,
  options: ExportReportOptions = {},
): Promise<void> {
  const preferences = mergePreferences(options);
  const report = buildCurrentRationReport(items, nutrients, norms, {
    ...options,
    ...preferences,
  });

  await runExportWithFeedback({
    format: 'pdf',
    report,
    options: {
      destinationDir: preferences.destinationDir || undefined,
      fileName: preferences.fileName || options.title || report.title,
      fontFamily: preferences.fontFamily,
      appearance: preferences.appearance,
    },
  });
}

export async function exportExcel(
  items: RationLikeItem[],
  nutrients: NutrientSummary | null,
  norms: Record<string, NormRange> | null | undefined,
  options: ExportReportOptions = {},
): Promise<void> {
  const preferences = mergePreferences(options);
  const report = buildCurrentRationReport(items, nutrients, norms, {
    ...options,
    ...preferences,
  });

  await runExportWithFeedback({
    format: 'xlsx',
    report,
    options: {
      destinationDir: preferences.destinationDir || undefined,
      fileName: preferences.fileName || options.title || report.title,
      fontFamily: preferences.fontFamily,
      appearance: preferences.appearance,
    },
  });
}

export async function downloadRationCSV(project: RationProject, filename?: string): Promise<void> {
  const preferences = mergePreferences({ fileName: filename });
  await runExportWithFeedback({
    format: 'csv',
    report: buildProjectReport(project),
    options: {
      destinationDir: preferences.destinationDir || undefined,
      fileName: preferences.fileName || project.name,
      fontFamily: preferences.fontFamily,
      appearance: preferences.appearance,
    },
  });
}

export async function downloadRationXLSX(project: RationProject, filename?: string): Promise<void> {
  const preferences = mergePreferences({ fileName: filename });
  await runExportWithFeedback({
    format: 'xlsx',
    report: buildProjectReport(project),
    options: {
      destinationDir: preferences.destinationDir || undefined,
      fileName: preferences.fileName || project.name,
      fontFamily: preferences.fontFamily,
      appearance: preferences.appearance,
    },
  });
}
