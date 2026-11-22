import {useEffect, useLayoutEffect, useRef, useState} from 'react';
import type {ReactNode} from 'react';
import {parseTerminalSession} from '@beyond10x/docs-system/product-data';
import type {TerminalEntry} from '@beyond10x/docs-system/product-data';
import styles from './styles.module.css';

/**
 * A recorded `b10x-terminal/1` session played like a terminal: each command is typed, its output
 * follows line by line, the view scrolls, and the session starts again after a pause. The server
 * renders the whole session as text; with reduced motion it stays that way.
 */
export interface AgentTerminalProps {
  session: unknown;
  caption?: ReactNode;
  /** The anchor of the figure on its page. */
  id?: string;
  prompt?: string;
}

/** `entry === entries.length` is the empty prompt after the last command. */
interface Shown {
  entry: number;
  chars: number;
  lines: number;
}

type Mode = 'whole' | 'waiting' | 'playing' | 'paused';

const START = {entry: 0, chars: 0, lines: 0};
const BEFORE_TYPING_MS = 700;
const TYPE_MS = 34;
const BEFORE_OUTPUT_MS = 380;
const LINE_MS = 55;
const BEFORE_PROMPT_MS = 160;
const AT_END_MS = 5000;

function outputLines(entry: TerminalEntry): string[] {
  return entry.output ? entry.output.split('\n') : [];
}

/** How long to wait in `shown`, and what comes after it; `undefined` at the end. */
function step(entries: readonly TerminalEntry[], shown: Shown): {delay: number; next: Shown} | undefined {
  if (shown.entry >= entries.length) return undefined;
  const entry = entries[shown.entry];
  const lines = outputLines(entry).length;
  if (shown.chars < entry.command.length) {
    const delay = shown.chars === 0 ? BEFORE_TYPING_MS : TYPE_MS + ((shown.chars * 7) % 5) * 9;
    return {delay, next: {...shown, chars: shown.chars + 1}};
  }
  if (shown.lines < lines) return {delay: shown.lines === 0 ? BEFORE_OUTPUT_MS : LINE_MS, next: {...shown, lines: shown.lines + 1}};
  return {delay: lines === 0 ? BEFORE_OUTPUT_MS : BEFORE_PROMPT_MS, next: {entry: shown.entry + 1, chars: 0, lines: 0}};
}

export default function AgentTerminal({session, caption, id, prompt = '$'}: AgentTerminalProps): ReactNode {
  const parsed = parseTerminalSession(session);
  const entries = parsed.entries;
  const title = parsed.title ?? 'Terminal';
  const [mode, setMode] = useState<Mode>('whole');
  const [shown, setShown] = useState<Shown | undefined>(undefined);
  const figure = useRef<HTMLElement>(null);
  const body = useRef<HTMLPreElement>(null);

  useEffect(() => {
    const reduced = window.matchMedia?.('(prefers-reduced-motion: reduce)');
    const target = figure.current;
    let observer: IntersectionObserver | undefined;
    const arm = (): void => {
      observer?.disconnect();
      if (reduced?.matches) {
        setMode('whole');
        setShown(undefined);
        return;
      }
      setMode('waiting');
      setShown(START);
      if (!target || !('IntersectionObserver' in window)) {
        setMode('playing');
        return;
      }
      observer = new IntersectionObserver((records) => {
        if (records.some((record) => record.isIntersecting)) {
          observer?.disconnect();
          setMode((current) => (current === 'waiting' ? 'playing' : current));
        }
      }, {threshold: 0.35});
      observer.observe(target);
    };
    arm();
    reduced?.addEventListener?.('change', arm);
    return () => {
      observer?.disconnect();
      reduced?.removeEventListener?.('change', arm);
    };
  }, []);

  useEffect(() => {
    if (mode !== 'playing' || !shown) return undefined;
    const next = step(entries, shown);
    const timer = window.setTimeout(() => setShown(next ? next.next : START), next ? next.delay : AT_END_MS);
    return () => window.clearTimeout(timer);
  }, [mode, shown, entries]);

  useLayoutEffect(() => {
    if (mode === 'playing' && body.current) body.current.scrollTop = body.current.scrollHeight;
  }, [mode, shown]);

  const animated = mode !== 'whole';
  // While it types, a screen reader reads the whole session from a hidden copy instead.
  const hidden = mode === 'waiting' || mode === 'playing';
  const toggle = (): void => {
    if (mode === 'playing') setMode('paused');
    else setMode('playing');
  };
  const replay = (): void => {
    setShown(START);
    setMode('playing');
  };

  const last = shown ? shown.entry : entries.length;
  const visible = entries.slice(0, Math.min(last + 1, entries.length));
  const cursor = (index: number): ReactNode => (shown && index === shown.entry && shown.lines === 0 ? <span className={styles.cursor} aria-hidden="true" /> : null);

  return <figure ref={figure} id={id} className={`b10x-terminal ${styles.terminal}`} aria-label={title}>
    <div className="b10x-terminal__bar">
      <span className="b10x-terminal__lights" aria-hidden="true"><i /><i /><i /></span>
      <span className="b10x-terminal__title">{title}</span>
      {animated && <span className={styles.controls}>
        <button type="button" className="b10x-terminal__copy" onClick={toggle} aria-label={mode === 'playing' ? 'Pause the recorded session' : 'Continue the recorded session'}>{mode === 'playing' ? 'Pause' : 'Continue'}</button>
        <button type="button" className="b10x-terminal__copy" onClick={replay} aria-label="Replay the recorded session from the start">Replay</button>
      </span>}
    </div>
    <pre ref={body} className={`b10x-terminal__body ${styles.body}`} tabIndex={hidden ? -1 : 0} aria-hidden={hidden ? true : undefined} data-mode={mode}><code>{visible.map((entry, index) => {
      const partial = shown !== undefined && index === shown.entry;
      const command = partial ? entry.command.slice(0, shown.chars) : entry.command;
      const lines = outputLines(entry);
      const output = partial ? lines.slice(0, shown.lines) : lines;
      return <span key={index}>
        <span className="b10x-terminal__line"><span className="b10x-terminal__prompt" aria-hidden="true">{prompt} </span><span className="b10x-terminal__command">{command}</span>{cursor(index)}{'\n'}</span>
        {output.length > 0 && <span className="b10x-terminal__output">{output.map((line, number) => <span key={number}>{line}{'\n'}</span>)}</span>}
      </span>;
    })}{last >= entries.length && <span className="b10x-terminal__line"><span className="b10x-terminal__prompt" aria-hidden="true">{prompt} </span>{shown && <span className={styles.cursor} aria-hidden="true" />}</span>}</code></pre>
    {hidden && <pre className={styles.srOnly}><code>{entries.map((entry) => [`${prompt} ${entry.command}`, ...outputLines(entry)].join('\n')).join('\n')}</code></pre>}
    {caption && <figcaption className="b10x-terminal__foot"><span className="b10x-terminal__dot" aria-hidden="true" /><span className="b10x-terminal__provenance">{caption}</span></figcaption>}
  </figure>;
}
