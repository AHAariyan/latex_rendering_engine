import { forwardRef, useCallback, useImperativeHandle, useRef, useState } from 'react';
import type { ColorValue, StyleProp, ViewStyle } from 'react-native';
import MathCoreView from './MathCoreViewNativeComponent';
import MathCoreField, { Commands as FieldCommands } from './MathCoreFieldNativeComponent';
import NativeMathCore from './NativeMathCore';

/** How much scaffolding a screen reader hears. */
export enum SpeechVerbosity {
  /** Every structure is opened and closed: "the fraction 1 over 2, end fraction". */
  Verbose = 0,
  /** Scaffolding only where the reading would be ambiguous. The default. */
  Brief = 1,
  /** Content words only. */
  Superbrief = 2,
}

/** The part of the source under a tap: UTF-8 byte offsets into `latex`. */
export interface MathRegion {
  start: number;
  end: number;
}

export interface MathTextProps {
  /** TeX source, or AsciiMath when `asciimath` is set. */
  latex: string;
  /** Em size in density-independent pixels. Default 17. */
  fontSize?: number;
  color?: ColorValue;
  /** Display style (`$$...$$`) rather than inline. Default true. */
  displayMode?: boolean;
  /** Break a formula too wide for its container into lines. Default true. */
  wrap?: boolean;
  /** Read `latex` as AsciiMath (`sum_(i=1)^n i^2`). */
  asciimath?: boolean;
  speechVerbosity?: SpeechVerbosity;
  style?: StyleProp<ViewStyle>;
  onTap?: (region: MathRegion, source: string) => void;
  onError?: (message: string) => void;
}

/**
 * Typesets a formula natively and sizes itself to it. VoiceOver and TalkBack
 * read the formula, then let the user step through its parts.
 */
export function MathText({
  latex,
  fontSize = 17,
  color,
  displayMode = true,
  wrap = true,
  asciimath = false,
  speechVerbosity = SpeechVerbosity.Brief,
  style,
  onTap,
  onError,
}: MathTextProps) {
  const [size, setSize] = useState<{ width: number; height: number } | null>(null);
  const onMathSize = useCallback(
    (e: { nativeEvent: { width: number; height: number } }) => {
      const { width, height } = e.nativeEvent;
      setSize((s) => (s && Math.abs(s.width - width) < 0.5 && Math.abs(s.height - height) < 0.5 ? s : { width, height }));
    },
    [],
  );
  const onMathTap = useCallback(
    (e: { nativeEvent: MathRegion }) => onTap?.(e.nativeEvent, sliceUtf8(latex, e.nativeEvent.start, e.nativeEvent.end)),
    [latex, onTap],
  );
  const onMathError = useCallback((e: { nativeEvent: { message: string } }) => onError?.(e.nativeEvent.message), [onError]);
  const sized: ViewStyle = wrap
    ? { alignSelf: 'stretch', height: size?.height ?? fontSize * 1.4 }
    : { width: size?.width ?? 0, height: size?.height ?? fontSize * 1.4 };
  return (
    <MathCoreView
      latex={latex}
      fontSize={fontSize}
      color={color}
      displayMode={displayMode}
      wrap={wrap}
      asciimath={asciimath}
      speechVerbosity={speechVerbosity}
      style={[sized, style]}
      onMathSize={onMathSize}
      onMathTap={onTap ? onMathTap : undefined}
      onMathError={onMathError}
    />
  );
}

export interface MathFieldProps {
  /** The formula as TeX. */
  value?: string;
  /** Called with the new TeX after every change. */
  onChangeText?: (latex: string) => void;
  /** Em size in density-independent pixels. Default 20. */
  fontSize?: number;
  color?: ColorValue;
  cursorColor?: ColorValue;
  placeholder?: string;
  editable?: boolean;
  style?: StyleProp<ViewStyle>;
}

/** What a `ref` to a MathField can do, for toolbar buttons. */
export interface MathFieldHandle {
  /** Runs an editor command: `frac`, `sqrt`, `nthroot`, `alpha`... */
  command(name: string): void;
  /** Types text as the keyboard would: `/` makes a fraction, `^` a superscript. */
  type(text: string): void;
  focus(): void;
  blur(): void;
}

/**
 * An editable formula: a text field for mathematics. Type as you would
 * write (`/` for a fraction, `^` for a superscript, `sqrt`, `pi`...); arrows
 * walk into and out of structures. VoiceOver and TalkBack hear the formula
 * and, after each key, where the cursor is.
 */
export const MathField = forwardRef<MathFieldHandle, MathFieldProps>(function MathField(
  { value, onChangeText, fontSize = 20, color, cursorColor, placeholder = '', editable = true, style },
  ref,
) {
  const native = useRef<React.ElementRef<typeof MathCoreField>>(null);
  const [size, setSize] = useState<{ width: number; height: number } | null>(null);
  useImperativeHandle(
    ref,
    () => ({
      command: (name) => native.current && FieldCommands.runCommand(native.current, name),
      type: (text) => native.current && FieldCommands.typeText(native.current, text),
      focus: () => native.current && FieldCommands.focus(native.current),
      blur: () => native.current && FieldCommands.blur(native.current),
    }),
    [],
  );
  const onMathSize = useCallback((e: { nativeEvent: { width: number; height: number } }) => {
    const { width, height } = e.nativeEvent;
    setSize((s) => (s && Math.abs(s.width - width) < 0.5 && Math.abs(s.height - height) < 0.5 ? s : { width, height }));
  }, []);
  const onMathChange = useCallback((e: { nativeEvent: { latex: string } }) => onChangeText?.(e.nativeEvent.latex), [onChangeText]);
  return (
    <MathCoreField
      ref={native}
      value={value}
      fontSize={fontSize}
      color={color}
      cursorColor={cursorColor}
      placeholder={placeholder}
      editable={editable}
      style={[{ minWidth: size?.width ?? fontSize * 3, height: size?.height ?? fontSize * 2 }, style]}
      onMathSize={onMathSize}
      onMathChange={onMathChange}
    />
  );
});

/**
 * A spoken sentence for a formula: "x squared plus y squared equals z squared".
 * `language` is a BCP 47 tag, one of 35 languages (en, es, bn, ar, zh-Hans, ja...); the device's by default.
 */
export function speech(tex: string, verbosity: SpeechVerbosity = SpeechVerbosity.Brief, language = ''): string {
  return NativeMathCore.speech(tex, verbosity, language);
}

/** One part of a formula for a screen reader to step through. */
export interface SpeechNode {
  role: string;
  label: string;
  text: string;
  /** UTF-8 byte range of the source. */
  start: number;
  end: number;
  children: SpeechNode[];
}

/** The formula as a tree a screen reader can walk part by part. */
export function speechTree(tex: string, verbosity: SpeechVerbosity = SpeechVerbosity.Brief, language = ''): SpeechNode {
  return JSON.parse(NativeMathCore.speechTree(tex, verbosity, language));
}

/** Presentation MathML for a formula. */
export function mathml(tex: string, displayMode = true): string {
  return NativeMathCore.mathml(tex, displayMode);
}

/** The formula in Nemeth braille (Unicode braille cells). */
export function nemeth(tex: string): string {
  return NativeMathCore.nemeth(tex);
}

/** AsciiMath (`sum_(i=1)^n i^2`) translated to TeX. */
export function asciimathToTex(source: string): string {
  return NativeMathCore.asciimathToTex(source);
}

/** The text between two UTF-8 byte offsets, without TextDecoder (Hermes lacks it). */
function sliceUtf8(s: string, start: number, end: number): string {
  let byte = 0;
  let out = '';
  for (const ch of s) {
    const cp = ch.codePointAt(0)!;
    if (byte >= start && byte < end) out += ch;
    byte += cp < 0x80 ? 1 : cp < 0x800 ? 2 : cp < 0x10000 ? 3 : 4;
    if (byte >= end) break;
  }
  return out;
}
