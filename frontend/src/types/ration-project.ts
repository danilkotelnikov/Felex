export interface RationProject {
  version: string;
  name: string;
  animalGroupId: string;
  animalProperties: {
    species: string;
    productionType: string;
    breed: string;
    sex: 'male' | 'female' | 'mixed';
    ageMonths?: number;
    weight: number;
    milkYieldKg?: number;
    milkFatPercent?: number;
    dailyGainG?: number;
    eggProduction?: number;
    litterSize?: number;
    stage?: string;
  };
  animalCount: number;
  items: RationProjectItem[];
  normPresetId?: string;
  customNorms?: Record<string, { min?: number; target?: number; max?: number }>;
  createdAt: string;
  updatedAt: string;
}

export interface RationProjectItem {
  feedId: number;
  feedName: string;
  amountKg: number;
  isLocked: boolean;
}

export interface FileNode {
  name: string;
  path: string;
  isDir: boolean;
  children?: FileNode[];
  animalGroup?: string;
  modifiedAt?: string;
}
