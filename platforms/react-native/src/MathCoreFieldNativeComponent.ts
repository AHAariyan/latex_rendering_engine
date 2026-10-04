import type * as React from 'react';
import { codegenNativeCommands, codegenNativeComponent } from 'react-native';
import type { CodegenTypes, ColorValue, HostComponent, ViewProps } from 'react-native';

type SizeEvent = Readonly<{ width: CodegenTypes.Float; height: CodegenTypes.Float }>;
type ChangeEvent = Readonly<{ latex: string }>;

export interface NativeProps extends ViewProps {
  /** The formula as TeX; applied when it differs from what the field holds. */
  value?: string;
  fontSize?: CodegenTypes.Float;
  color?: ColorValue;
  cursorColor?: ColorValue;
  placeholder?: string;
  editable?: CodegenTypes.WithDefault<boolean, true>;
  onMathChange?: CodegenTypes.DirectEventHandler<ChangeEvent>;
  onMathSize?: CodegenTypes.DirectEventHandler<SizeEvent>;
}

interface NativeCommands {
  /** Runs an editor command: frac, sqrt, nthroot, alpha... */
  runCommand: (viewRef: React.ElementRef<HostComponent<NativeProps>>, name: string) => void;
  /** Types text as the keyboard would. */
  typeText: (viewRef: React.ElementRef<HostComponent<NativeProps>>, text: string) => void;
  focus: (viewRef: React.ElementRef<HostComponent<NativeProps>>) => void;
  blur: (viewRef: React.ElementRef<HostComponent<NativeProps>>) => void;
}

export const Commands = codegenNativeCommands<NativeCommands>({
  supportedCommands: ['runCommand', 'typeText', 'focus', 'blur'],
});

export default codegenNativeComponent<NativeProps>('MathCoreField');
