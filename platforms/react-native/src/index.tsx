import { useCallback, useState } from 'react';
import type { ColorValue, StyleProp, ViewStyle } from 'react-native';
import MathCoreView from './MathCoreViewNativeComponent';
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

/** A spoken sentence for a formula: "x squared plus y squared equals z squared". */
export function speech(tex: string, verbosity: SpeechVerbosity = SpeechVerbosity.Brief): string {
  return NativeMathCore.speech(tex, verbosity);
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
export function speechTree(tex: string, verbosity: SpeechVerbosity = SpeechVerbosity.Brief): SpeechNode {
  return JSON.parse(NativeMathCore.speechTree(tex, verbosity));
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
