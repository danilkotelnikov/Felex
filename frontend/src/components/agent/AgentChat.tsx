import { useEffect, useMemo, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import {
  Send,
  Bot,
  User,
  Paperclip,
  Loader2,
  AlertCircle,
  Globe,
  Trash2,
  RefreshCw,
  ChevronDown,
  CheckCircle2,
} from 'lucide-react';
import { Icon } from '../ui/Icon';
import { Button } from '../ui/Button';
import { Input } from '../ui/Input';
import { MarkdownContent } from '../ui/MarkdownContent';
import { cn } from '@/lib/utils';
import { useAgentStore } from '@/stores/agentStore';
import { useRationStore } from '@/stores/rationStore';
import { agentApi, ApiError, type ChatContext, type ChatMessage as ApiChatMessage } from '@/lib/api';

const AVAILABLE_MODELS = [
  { id: 'qwen3.5:4b', label: 'Qwen 3.5 4B', desc: 'Fast, 3.4 GB' },
  { id: 'qwen3.5:9b', label: 'Qwen 3.5 9B', desc: 'Accurate, 6.6 GB' },
] as const;

function sanitizeToolMarkup(text: string) {
  return text
    .replace(/<tool>[\s\S]*?<\/tool>/gi, '')
    .replace(/<params>[\s\S]*?<\/params>/gi, '')
    .trim();
}

function formatAnimalType(species: string, productionType: string) {
  const speciesLabel = species === 'swine'
    ? 'Swine'
    : species === 'poultry'
      ? 'Poultry'
      : 'Cattle';

  return `${speciesLabel} / ${productionType}`;
}

function formatProductionLevel(props: ReturnType<typeof useRationStore.getState>['animalProperties']) {
  if (props.milkYieldKg) {
    return `${props.milkYieldKg} kg milk/day`;
  }
  if (props.dailyGainG) {
    return `${props.dailyGainG} g gain/day`;
  }
  if (props.eggProductionPerYear) {
    return `${props.eggProductionPerYear} eggs/year`;
  }

  return `${props.liveWeightKg} kg live weight`;
}

function buildChatContext(args: {
  animalCount: number;
  currentProjectName: string | null;
  animalProperties: ReturnType<typeof useRationStore.getState>['animalProperties'];
  localItems: ReturnType<typeof useRationStore.getState>['localItems'];
  nutrients: ReturnType<typeof useRationStore.getState>['nutrients'];
}): ChatContext {
  const { animalCount, currentProjectName, animalProperties, localItems, nutrients } = args;
  const rationLines = [
    currentProjectName ? `Project: ${currentProjectName}` : null,
    `Animals: ${animalCount}`,
    localItems.length === 0
      ? 'No feeds selected yet. Build a starter ration from the local feed library.'
      : null,
    ...localItems.map((item, index) => (
      `${index + 1}. ${item.feed.name_ru} - ${item.amount_kg.toFixed(2)} kg/day${item.is_locked ? ' [locked]' : ''}`
    )),
  ].filter(Boolean);

  const nutrientLines = nutrients && localItems.length > 0 ? [
    `Dry matter: ${nutrients.total_dm_kg.toFixed(2)} kg/day`,
    `Energy: ${nutrients.energy_eke.toFixed(2)} EKE; ${nutrients.energy_oe_cattle.toFixed(2)} MJ OE cattle; ${nutrients.energy_oe_pig.toFixed(2)} MJ OE pig; ${nutrients.energy_oe_poultry.toFixed(2)} MJ OE poultry`,
    `Protein: ${nutrients.crude_protein.toFixed(0)} g CP; lysine ${nutrients.lysine.toFixed(1)} g; methionine + cystine ${nutrients.methionine_cystine.toFixed(1)} g`,
    `Fiber/minerals: crude fiber ${nutrients.crude_fiber.toFixed(1)} g; Ca ${nutrients.calcium.toFixed(1)} g; P ${nutrients.phosphorus.toFixed(1)} g; Ca:P ${nutrients.ca_p_ratio.toFixed(2)}`,
  ] : ['Nutrient status: not calculated yet.'];

  return {
    animal_type: formatAnimalType(animalProperties.species, animalProperties.productionType),
    production_level: formatProductionLevel(animalProperties),
    current_ration: rationLines.join('\n'),
    nutrient_status: nutrientLines.join('\n'),
  };
}

export function AgentChat() {
  const { t, i18n } = useTranslation();
  const assistantTitle = i18n.language === 'ru' ? 'AI-ассистент' : 'AI Assistant';
  const [input, setInput] = useState('');
  const [showModelMenu, setShowModelMenu] = useState(false);
  const [attachCurrentRation, setAttachCurrentRation] = useState(false);
  const messagesEndRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLInputElement>(null);
  const abortControllerRef = useRef<AbortController | null>(null);

  const {
    messages,
    status,
    statusLoading,
    isStreaming,
    settings,
    addMessage,
    updateLastMessage,
    clearMessages,
    setStreaming,
    reloadAgent,
    switchModel,
  } = useAgentStore();
  const {
    localItems,
    animalProperties,
    animalCount,
    nutrients,
    currentProjectName,
  } = useRationStore();

  const canAttachRation = true;
  const chatContext = attachCurrentRation && canAttachRation
    ? buildChatContext({ animalCount, currentProjectName, animalProperties, localItems, nutrients })
    : undefined;

  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [messages]);

  useEffect(() => () => abortControllerRef.current?.abort(), []);

  useEffect(() => {
    if (!showModelMenu) return undefined;
    const handleClick = () => setShowModelMenu(false);
    document.addEventListener('click', handleClick);
    return () => document.removeEventListener('click', handleClick);
  }, [showModelMenu]);

  const currentModel = useMemo(
    () => AVAILABLE_MODELS.find((model) => model.id === settings.model) ?? AVAILABLE_MODELS[0],
    [settings.model]
  );

  const sendMessage = async (userMessage: string) => {
    if (!userMessage.trim() || isStreaming) return;

    addMessage('user', userMessage);
    setStreaming(true);
    addMessage('assistant', '');

    const apiMessages: ApiChatMessage[] = messages
      .filter((message) => message.content)
      .map((message) => ({ role: message.role, content: message.content }));

    apiMessages.push({ role: 'user', content: userMessage });
    abortControllerRef.current = new AbortController();

    try {
      const response = await agentApi.chat({
        messages: apiMessages,
        stream: false,
        context: chatContext,
      });

      const cleanMessage = sanitizeToolMarkup(response.message);
      updateLastMessage(cleanMessage || response.message);
    } catch (error) {
      if (error instanceof Error && error.name === 'AbortError') {
        updateLastMessage(t('common.cancel'));
      } else if (error instanceof ApiError) {
        updateLastMessage(error.message);
      } else if (error instanceof Error && error.message) {
        updateLastMessage(error.message);
      } else {
        updateLastMessage(`${t('common.error')}. ${t('agent.ensureOllamaRunning')}`);
      }
    } finally {
      setStreaming(false);
      abortControllerRef.current = null;
    }
  };

  const handleSubmit = async (event: React.FormEvent) => {
    event.preventDefault();
    if (!input.trim() || isStreaming) return;

    const userMessage = input.trim();
    setInput('');
    await sendMessage(userMessage);
  };

  return (
    <div className="flex flex-col h-full bg-[--bg-base]">
      <div className="px-3 py-2 border-b border-[--border] flex items-center justify-between gap-3">
        <div className="flex items-center gap-2 min-w-0">
          <div className="w-7 h-7 rounded-full bg-[--bg-surface] border border-[--border] flex items-center justify-center shrink-0">
            <Icon icon={Bot} size={14} className="text-[--accent]" />
          </div>
          <div className="min-w-0">
            <div className="text-xs font-medium text-[--text-primary] truncate">{assistantTitle}</div>
          </div>
          <StatusIndicator status={status} loading={statusLoading} />
        </div>

        <div className="flex items-center gap-1 shrink-0">
          <div className="relative">
            <button
              onClick={(event) => {
                event.stopPropagation();
                setShowModelMenu((open) => !open);
              }}
              className="flex items-center gap-1.5 px-2.5 py-1.5 text-[10px] rounded-md border border-[--border] bg-[--bg-surface] hover:bg-[--bg-hover] text-[--text-secondary] transition-colors disabled:opacity-50"
              disabled={statusLoading}
              title={currentModel.desc}
            >
              <span className="max-w-[94px] truncate">{currentModel.label}</span>
              <Icon icon={ChevronDown} size={10} className={cn(statusLoading && 'animate-spin')} />
            </button>
            {showModelMenu ? (
              <div
                className="absolute right-0 top-full mt-1 z-50 min-w-[188px] bg-[--bg-surface] border border-[--border] rounded-lg shadow-lg overflow-hidden"
                onClick={(event) => event.stopPropagation()}
              >
                {AVAILABLE_MODELS.map((model) => {
                  const selected = model.id === currentModel.id;
                  return (
                    <button
                      key={model.id}
                      onClick={async () => {
                        await switchModel(model.id);
                        setShowModelMenu(false);
                      }}
                      disabled={statusLoading || selected}
                      className={cn(
                        'w-full text-left px-3 py-2 text-xs hover:bg-[--bg-hover] transition-colors flex items-center justify-between',
                        selected && 'bg-[--bg-active] text-[--accent]',
                        (statusLoading || selected) && 'opacity-60 cursor-not-allowed'
                      )}
                    >
                      <div>
                        <div className="font-medium">{model.label}</div>
                        <div className="text-[10px] text-[--text-disabled]">{model.desc}</div>
                      </div>
                      {selected ? <div className="w-1.5 h-1.5 rounded-full bg-[--accent]" /> : null}
                    </button>
                  );
                })}
              </div>
            ) : null}
          </div>

          <Button
            variant="ghost"
            size="sm"
            className="h-7 w-7 p-0"
            onClick={() => reloadAgent({ model: settings.model, backend: 'ollama', webEnabled: settings.webSearch, contextSize: settings.contextSize })}
            title={t('agent.reloadAgent')}
            disabled={statusLoading}
          >
            <Icon icon={RefreshCw} size={14} className={cn(statusLoading && 'animate-spin')} />
          </Button>
          <Button variant="ghost" size="sm" className="h-7 w-7 p-0" onClick={clearMessages} title={t('agent.clearConversation')}>
            <Icon icon={Trash2} size={14} />
          </Button>
        </div>
      </div>

      <div className="flex-1 overflow-y-auto p-3 space-y-3">
        {messages.length === 0 ? (
          <WelcomeMessage title={assistantTitle} onSuggestionClick={sendMessage} />
        ) : (
          messages.map((message) => <ChatMessage key={message.id} message={message} />)
        )}
        {isStreaming && messages.length > 0 && messages[messages.length - 1].content === '' ? (
          <div className="flex items-center gap-2 text-xs text-[--text-secondary]">
            <Icon icon={Loader2} size={14} className="animate-spin" />
            {t('agent.thinking')}
          </div>
        ) : null}
        <div ref={messagesEndRef} />
      </div>

      <form onSubmit={handleSubmit} className="p-3 border-t border-[--border]">
        {attachCurrentRation && canAttachRation ? (
          <div className="mb-2 flex items-center justify-between gap-2 rounded-[--radius-md] border border-[--border] bg-[--bg-surface] px-2.5 py-2">
            <div className="min-w-0 flex items-center gap-2">
              <div className="flex h-6 w-6 items-center justify-center rounded-full bg-[--bg-active] text-[--accent]">
                <Icon icon={Paperclip} size={12} />
              </div>
              <div className="min-w-0">
                <div className="text-[10px] font-medium text-[--text-primary]">
                  {localItems.length > 0 ? t('agent.rationAttached') : t('agent.blankRationAttached')}
                </div>
                <div className="truncate text-[10px] text-[--text-secondary]">
                  {(currentProjectName || t('workspace.ration'))}: {localItems.length} feeds
                </div>
              </div>
            </div>
            <Button
              type="button"
              variant="ghost"
              size="sm"
              className="h-7 px-2 text-[10px]"
              onClick={() => void sendMessage(
                localItems.length > 0 ? t('agent.analyzeAttachedRation') : t('agent.buildStarterRation'),
              )}
              disabled={isStreaming || !status.modelLoaded}
            >
              {localItems.length > 0 ? t('agent.analyzeAttachedRation') : t('agent.buildStarterRation')}
            </Button>
          </div>
        ) : null}

        <div className="flex gap-2">
          <Button
            type="button"
            variant={attachCurrentRation ? 'outline' : 'ghost'}
            size="sm"
            className="h-9 px-2 shrink-0"
            onClick={() => setAttachCurrentRation((value) => !value)}
            title={attachCurrentRation ? t('agent.detachCurrentRation') : t('agent.attachCurrentRation')}
          >
            <Icon icon={Paperclip} size={14} />
          </Button>
          <Input
            ref={inputRef}
            value={input}
            onChange={(event) => setInput(event.target.value)}
            placeholder={t('agent.askAboutNutrition')}
            className="flex-1 text-xs"
            disabled={isStreaming || !status.modelLoaded}
          />
          {isStreaming ? (
            <Button type="button" size="sm" variant="ghost" onClick={() => abortControllerRef.current?.abort()}>
              {t('common.cancel')}
            </Button>
          ) : (
            <Button type="submit" size="sm" disabled={!input.trim() || isStreaming || !status.modelLoaded}>
              <Icon icon={Send} size={14} />
            </Button>
          )}
        </div>
      </form>
    </div>
  );
}

interface ChatMessageProps {
  message: {
    id: string;
    role: 'user' | 'assistant';
    content: string;
    timestamp: Date;
  };
}

function ChatMessage({ message }: ChatMessageProps) {
  const isUser = message.role === 'user';

  return (
    <div className={cn('flex gap-2 min-w-0', isUser && 'flex-row-reverse')}>
      <div
        className={cn(
          'w-6 h-6 rounded-full flex items-center justify-center flex-shrink-0',
          isUser ? 'bg-[--accent]' : 'bg-[--bg-surface] border border-[--border]'
        )}
      >
        <Icon icon={isUser ? User : Bot} size={12} className={isUser ? 'text-white' : 'text-[--text-secondary]'} />
      </div>
      <div
        className={cn(
          'max-w-[calc(100%-2rem)] min-w-0 overflow-hidden px-3 py-2 rounded-[--radius-md] text-xs',
          isUser ? 'bg-[--accent] text-white' : 'bg-[--bg-surface] text-[--text-primary] border border-[--border]'
        )}
      >
        {message.content ? (
          isUser ? (
            <div className="max-w-full whitespace-pre-wrap break-all [overflow-wrap:anywhere] leading-relaxed">{message.content}</div>
          ) : (
            <MarkdownContent markdown={message.content} variant="chat" className="max-w-full" />
          )
        ) : (
          <span className="text-[--text-disabled]">...</span>
        )}
      </div>
    </div>
  );
}

interface WelcomeMessageProps {
  title: string;
  onSuggestionClick: (text: string) => void;
}

function WelcomeMessage({ title, onSuggestionClick }: WelcomeMessageProps) {
  const { t } = useTranslation();
  const { status } = useAgentStore();

  return (
    <div className="text-center py-6">
      <div className="w-12 h-12 rounded-full bg-[--bg-surface] border border-[--border] flex items-center justify-center mx-auto mb-3">
        <Icon icon={Bot} size={24} className="text-[--accent]" />
      </div>
      <h3 className="text-sm font-medium text-[--text-primary] mb-1">{title}</h3>
      <p className="text-xs text-[--text-secondary] mb-4 max-w-xs mx-auto">{t('agent.welcomeDescription')}</p>
      {status.modelLoaded ? (
        <div className="space-y-2">
          <SuggestionChip text={t('agent.suggestion1')} onClick={onSuggestionClick} />
          <SuggestionChip text={t('agent.suggestion2')} onClick={onSuggestionClick} />
          <SuggestionChip text={t('agent.suggestion3')} onClick={onSuggestionClick} />
        </div>
      ) : (
        <div className="text-xs text-[--status-warn] bg-[--status-warn-bg] px-3 py-2 rounded mx-auto max-w-xs">
          {t('agent.modelNotLoaded')}. {t('agent.ensureOllamaRunning')}
        </div>
      )}
    </div>
  );
}

interface SuggestionChipProps {
  text: string;
  onClick: (text: string) => void;
}

function SuggestionChip({ text, onClick }: SuggestionChipProps) {
  const { status, isStreaming } = useAgentStore();

  return (
    <button
      onClick={() => {
        if (!status.modelLoaded || isStreaming) return;
        onClick(text);
      }}
      disabled={!status.modelLoaded || isStreaming}
      className="inline-block px-3 py-1.5 text-xs text-[--text-secondary] bg-[--bg-surface] border border-[--border] rounded-full hover:border-[--accent] hover:text-[--accent] transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
    >
      {text}
    </button>
  );
}

interface StatusIndicatorProps {
  status: {
    modelLoaded: boolean;
    modelName: string;
    backend: string;
    webEnabled: boolean;
  };
  loading?: boolean;
}

function StatusIndicator({ status, loading }: StatusIndicatorProps) {
  const { t } = useTranslation();

  if (loading) {
    return (
      <div className="inline-flex items-center gap-1 px-2 py-1 rounded-full bg-[--bg-surface] text-[10px] text-[--text-secondary] border border-[--border]">
        <Icon icon={Loader2} size={11} className="animate-spin" />
        <span>{t('agent.connecting')}</span>
      </div>
    );
  }

  if (!status.modelLoaded) {
    return (
      <div className="inline-flex items-center gap-1 px-2 py-1 rounded-full bg-[--status-warn-bg] text-[10px] text-[--status-warn]">
        <Icon icon={AlertCircle} size={11} />
        <span>{t('agent.modelNotLoaded')}</span>
      </div>
    );
  }

  return (
    <div className="inline-flex items-center gap-1.5 px-2 py-1 rounded-full bg-green-500/10 text-[10px] text-green-700 border border-green-500/20">
      <Icon icon={CheckCircle2} size={11} />
      <span>{t('common.ok')}</span>
      {status.webEnabled ? <span title="Web enabled"><Icon icon={Globe} size={11} /></span> : null}
    </div>
  );
}

