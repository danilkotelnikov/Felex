import { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import * as Dialog from '@radix-ui/react-dialog';
import { Milk, Bird, PiggyBank, X, ChevronLeft } from 'lucide-react';
import toast from 'react-hot-toast';
import { workspaceApi } from '@/lib/workspace-api';
import { NORM_PRESETS } from '@/types/nutrient';
import type { RationProject } from '@/types/ration-project';
import { Icon } from '../ui/Icon';
import { Button } from '../ui/Button';
import { Input } from '../ui/Input';

const BREEDS: Record<string, string[]> = {
  cattle: ['Голштинская', 'Симментальская', 'Абердин-ангусская', 'Герефордская', 'Швицкая'],
  swine: ['Крупная белая', 'Ландрас', 'Дюрок', 'Пьетрен'],
  poultry: ['Кобб 500', 'Росс 308', 'Ломанн Браун', 'Хайсекс'],
};

const PRODUCTION_TYPES: Record<string, { id: string; labelKey: string }[]> = {
  cattle: [
    { id: 'dairy', labelKey: 'newRation.dairy' },
    { id: 'beef', labelKey: 'newRation.beef' },
  ],
  swine: [
    { id: 'fattening', labelKey: 'newRation.finisher' },
    { id: 'breeding', labelKey: 'newRation.sow' },
  ],
  poultry: [
    { id: 'broiler', labelKey: 'newRation.broiler' },
    { id: 'layer', labelKey: 'newRation.layer' },
  ],
};

function resolveGroupId(species: string, prodType: string): string {
  const map: Record<string, string> = {
    cattle_dairy: 'cattle_dairy',
    cattle_beef: 'cattle_beef',
    swine_fattening: 'swine_finisher',
    swine_breeding: 'swine_sow',
    swine_finisher: 'swine_finisher',
    swine_sow: 'swine_sow',
    poultry_broiler: 'poultry_broiler',
    poultry_layer: 'poultry_layer',
  };
  return map[`${species}_${prodType}`] || 'cattle_dairy';
}

function getProductionLabelKey(prodType: string): string {
  if (prodType === 'fattening') return 'newRation.finisher';
  if (prodType === 'breeding') return 'newRation.sow';
  return `newRation.${prodType}`;
}

interface Props {
  open: boolean;
  onClose: () => void;
  folderPath: string;
  onCreated: (path: string, project: RationProject) => void;
}

export function NewRationDialog({ open, onClose, folderPath, onCreated }: Props) {
  const { t } = useTranslation();
  const [step, setStep] = useState(1);
  const [species, setSpecies] = useState('');
  const [prodType, setProdType] = useState('');
  const [breed, setBreed] = useState('');
  const [customBreed, setCustomBreed] = useState('');
  const [sex, setSex] = useState<'female' | 'male' | 'mixed'>('female');
  const [ageMonths, setAgeMonths] = useState<number | undefined>();
  const [weight, setWeight] = useState(500);
  const [milkYield, setMilkYield] = useState<number | undefined>();
  const [dailyGain, setDailyGain] = useState<number | undefined>();
  const [eggProduction, setEggProduction] = useState<number | undefined>();
  const [animalCount, setAnimalCount] = useState(1);
  const [fileName, setFileName] = useState('');
  const [destinationPath, setDestinationPath] = useState(folderPath);
  const [creating, setCreating] = useState(false);

  const groupId = resolveGroupId(species, prodType);
  const matchedPreset = NORM_PRESETS
    .filter((preset) => preset.groupId === groupId)
    .sort((left, right) => {
      const currentMetric =
        species === 'cattle' && prodType === 'dairy' ? milkYield :
        species === 'poultry' && prodType === 'layer' ? eggProduction :
        species === 'poultry' && prodType === 'broiler' ? ageMonths :
        weight;

      const leftMetric =
        left.params.milkYield ??
        left.params.eggProduction ??
        left.params.weight ??
        (left.params.age ? Number.parseFloat(left.params.age) : undefined);
      const rightMetric =
        right.params.milkYield ??
        right.params.eggProduction ??
        right.params.weight ??
        (right.params.age ? Number.parseFloat(right.params.age) : undefined);

      const leftDistance = Math.abs((leftMetric ?? 0) - (currentMetric ?? 0));
      const rightDistance = Math.abs((rightMetric ?? 0) - (currentMetric ?? 0));
      return leftDistance - rightDistance;
    })[0];

  const resetForm = () => {
    setStep(1); setSpecies(''); setProdType(''); setBreed(''); setCustomBreed('');
    setSex('female'); setAgeMonths(undefined); setWeight(500); setMilkYield(undefined);
    setDailyGain(undefined); setEggProduction(undefined); setAnimalCount(1); setFileName(''); setDestinationPath(folderPath);
  };

  useEffect(() => {
    if (open) {
      setDestinationPath(folderPath);
    }
  }, [folderPath, open]);

  const handleSelectSpecies = (s: string) => {
    setSpecies(s);
    // Set defaults
    if (s === 'cattle') setWeight(600);
    else if (s === 'swine') setWeight(80);
    else setWeight(2);
    setStep(2);
  };

  const handleSelectProdType = (pt: string) => {
    setProdType(pt);
    if (species === 'cattle' && pt === 'dairy') { setMilkYield(25); setWeight(600); }
    if (species === 'cattle' && pt === 'beef') { setDailyGain(1000); setWeight(450); }
    if (species === 'swine' && pt === 'fattening') { setDailyGain(900); setWeight(80); }
    if (species === 'swine' && pt === 'breeding') { setWeight(210); }
    if (species === 'poultry' && pt === 'broiler') { setDailyGain(55); setWeight(2.4); }
    if (species === 'poultry' && pt === 'layer') { setEggProduction(320); setWeight(1.8); }
    setStep(3);
  };

  const handleCreate = async () => {
    const baseName = fileName.trim() || `${t(getProductionLabelKey(prodType))} ${new Date().toLocaleDateString('ru-RU')}`;
    const autoName = !fileName.trim();

    const projectBase: Omit<RationProject, 'name'> = {
      version: '1.0',
      animalGroupId: groupId,
      animalProperties: {
        species,
        productionType: prodType,
        breed: breed === 'other' ? customBreed : breed,
        sex,
        ageMonths,
        weight,
        milkYieldKg: milkYield,
        dailyGainG: dailyGain,
        eggProduction,
      },
      animalCount,
      items: [],
      normPresetId: matchedPreset?.id,
      createdAt: new Date().toISOString(),
      updatedAt: new Date().toISOString(),
    };

    setCreating(true);
    try {
      const normalizedDestination = destinationPath.trim().replace(/^[\\/]+|[\\/]+$/g, '').replace(/\\/g, '/');
      for (let attempt = 0; attempt < 25; attempt += 1) {
        const candidateName = attempt === 0 ? baseName : `${baseName} (${attempt + 1})`;
        const safeName = candidateName.replace(/[<>:"/\\|?*]/g, '_');
        const path = normalizedDestination ? `${normalizedDestination}/${safeName}.felex.json` : `${safeName}.felex.json`;
        const project: RationProject = {
          ...projectBase,
          name: candidateName,
        };

        try {
          await workspaceApi.createRation(path, project);
          toast.success(t('newRation.createAndOpen'));
          onCreated(path, project);
          resetForm();
          onClose();
          return;
        } catch (error: any) {
          const message = String(error?.message || '');
          if (autoName && message.includes('already exists') && attempt < 24) {
            continue;
          }
          throw error;
        }
      }
    } catch (e: any) {
      toast.error(e.message);
    } finally {
      setCreating(false);
    }
  };

  const speciesIcons = { cattle: Milk, swine: PiggyBank, poultry: Bird };

  return (
    <Dialog.Root open={open} onOpenChange={(o) => { if (!o) { resetForm(); onClose(); } }}>
      <Dialog.Portal>
        <Dialog.Overlay className="fixed inset-0 bg-black/40 z-50" />
        <Dialog.Content className="fixed top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 z-50 bg-[--bg-surface] border border-[--border] rounded-lg shadow-xl w-[480px] max-h-[85vh] overflow-y-auto">
          <div className="flex items-center justify-between px-4 py-3 border-b border-[--border]">
            <div className="flex items-center gap-2">
              {step > 1 && (
                <button onClick={() => setStep(step - 1)} className="p-1 rounded hover:bg-[--bg-hover]">
                  <Icon icon={ChevronLeft} size={16} className="text-[--text-secondary]" />
                </button>
              )}
              <Dialog.Title className="text-sm font-medium text-[--text-primary]">
                {t('newRation.title')} — {t(`newRation.step${step}`)}
              </Dialog.Title>
            </div>
            <Dialog.Close className="p-1 rounded hover:bg-[--bg-hover]">
              <Icon icon={X} size={16} className="text-[--text-disabled]" />
            </Dialog.Close>
          </div>

          <div className="p-4">
            {step === 1 && (
              <div className="grid grid-cols-3 gap-3">
                {(['cattle', 'swine', 'poultry'] as const).map((s) => (
                  <button
                    key={s}
                    onClick={() => handleSelectSpecies(s)}
                    className="flex flex-col items-center gap-2 p-4 rounded-lg border border-[--border] hover:border-[--accent] hover:bg-[--bg-hover] transition-colors"
                  >
                    <Icon icon={speciesIcons[s]} size={32} className="text-[--accent]" />
                    <span className="text-xs font-medium text-[--text-primary]">{t(`newRation.${s}`)}</span>
                  </button>
                ))}
              </div>
            )}

            {step === 2 && (
              <div className="grid grid-cols-2 gap-3">
                {PRODUCTION_TYPES[species]?.map((pt) => (
                  <button
                    key={pt.id}
                    onClick={() => handleSelectProdType(pt.id)}
                    className="flex flex-col items-center gap-2 p-4 rounded-lg border border-[--border] hover:border-[--accent] hover:bg-[--bg-hover] transition-colors"
                  >
                    <span className="text-sm font-medium text-[--text-primary]">{t(pt.labelKey)}</span>
                  </button>
                ))}
              </div>
            )}

            {step === 3 && (
              <div className="space-y-3">
                <div>
                  <label className="text-xs text-[--text-secondary] mb-1 block">{t('newRation.breed')}</label>
                  <select
                    value={breed}
                    onChange={(e) => setBreed(e.target.value)}
                    className="w-full px-2 py-1.5 text-xs bg-[--bg-base] border border-[--border] rounded text-[--text-primary]"
                  >
                    <option value="">—</option>
                    {BREEDS[species]?.map((b) => <option key={b} value={b}>{b}</option>)}
                    <option value="other">{t('animal.otherBreed')}</option>
                  </select>
                  {breed === 'other' && (
                    <Input className="mt-1 text-xs" value={customBreed} onChange={(e) => setCustomBreed(e.target.value)} placeholder={t('animal.otherBreed')} />
                  )}
                </div>

                <div className="grid grid-cols-2 gap-3">
                  <div>
                    <label className="text-xs text-[--text-secondary] mb-1 block">{t('newRation.sex')}</label>
                    <select value={sex} onChange={(e) => setSex(e.target.value as any)} className="w-full px-2 py-1.5 text-xs bg-[--bg-base] border border-[--border] rounded text-[--text-primary]">
                      <option value="female">{t('animal.female')}</option>
                      <option value="male">{t('animal.male')}</option>
                      <option value="mixed">{t('animal.mixed')}</option>
                    </select>
                  </div>
                  <div>
                    <label className="text-xs text-[--text-secondary] mb-1 block">{t('newRation.age')}</label>
                    <Input type="number" min={0} className="text-xs" value={ageMonths ?? ''} onChange={(e) => setAgeMonths(e.target.value ? Number(e.target.value) : undefined)} />
                  </div>
                </div>

                <div className="grid grid-cols-2 gap-3">
                  <div>
                    <label className="text-xs text-[--text-secondary] mb-1 block">{t('newRation.weight')}</label>
                    <Input type="number" min={0} className="text-xs" value={weight} onChange={(e) => setWeight(Number(e.target.value))} />
                  </div>
                  <div>
                    <label className="text-xs text-[--text-secondary] mb-1 block">{t('newRation.animalCount')}</label>
                    <Input type="number" min={1} className="text-xs" value={animalCount} onChange={(e) => setAnimalCount(Number(e.target.value) || 1)} />
                  </div>
                </div>

                {species === 'cattle' && prodType === 'dairy' && (
                  <div>
                    <label className="text-xs text-[--text-secondary] mb-1 block">{t('newRation.milkYield')}</label>
                    <Input type="number" min={0} className="text-xs" value={milkYield ?? ''} onChange={(e) => setMilkYield(Number(e.target.value))} />
                  </div>
                )}

                {((species === 'cattle' && prodType === 'beef') || (species === 'swine') || (species === 'poultry' && prodType === 'broiler')) && (
                  <div>
                    <label className="text-xs text-[--text-secondary] mb-1 block">{t('newRation.dailyGain')}</label>
                    <Input type="number" min={0} className="text-xs" value={dailyGain ?? ''} onChange={(e) => setDailyGain(Number(e.target.value))} />
                  </div>
                )}

                {species === 'poultry' && prodType === 'layer' && (
                  <div>
                    <label className="text-xs text-[--text-secondary] mb-1 block">{t('newRation.eggProduction')}</label>
                    <Input type="number" min={0} className="text-xs" value={eggProduction ?? ''} onChange={(e) => setEggProduction(Number(e.target.value))} />
                  </div>
                )}

                <div>
                  <label className="text-xs text-[--text-secondary] mb-1 block">{t('newRation.fileName')}</label>
                  <Input className="text-xs" value={fileName} onChange={(e) => setFileName(e.target.value)} placeholder={`${t(getProductionLabelKey(prodType))} ${new Date().toLocaleDateString('ru-RU')}`} />
                </div>

                <div>
                  <label className="text-xs text-[--text-secondary] mb-1 block">{t('newRation.destinationFolder')}</label>
                  <Input
                    className="text-xs"
                    value={destinationPath}
                    onChange={(e) => setDestinationPath(e.target.value.replace(/\\/g, '/'))}
                    placeholder={t('newRation.workspaceRoot')}
                  />
                </div>

                {matchedPreset && (
                  <div className="p-2 bg-[--bg-base] rounded border border-[--border] text-xs text-[--text-secondary]">
                    {t('newRation.summary')}: <span className="text-[--text-primary] font-medium">{matchedPreset.label_ru}</span>
                  </div>
                )}

                <Button
                  variant="default"
                  size="sm"
                  className="w-full"
                  onClick={() => void handleCreate()}
                  disabled={creating}
                >
                  {creating ? t('common.loading') : t('newRation.createAndOpen')}
                </Button>
              </div>
            )}
          </div>
        </Dialog.Content>
      </Dialog.Portal>
    </Dialog.Root>
  );
}
