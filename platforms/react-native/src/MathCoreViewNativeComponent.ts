import { codegenNativeComponent } from 'react-native';
import type { CodegenTypes, ColorValue, ViewProps } from 'react-native';

type SizeEvent = Readonly<{ width: CodegenTypes.Float; height: CodegenTypes.Float }>;
type TapEvent = Readonly<{ start: CodegenTypes.Int32; end: CodegenTypes.Int32 }>;
type ErrorEvent = Readonly<{ message: string }>;

export interface NativeProps extends ViewProps {
  latex: string;
  fontSize?: CodegenTypes.Float;
  color?: ColorValue;
  displayMode?: CodegenTypes.WithDefault<boolean, true>;
  wrap?: CodegenTypes.WithDefault<boolean, true>;
  asciimath?: boolean;
  /** 0 verbose, 1 brief, 2 superbrief: how much a screen reader hears. */
  speechVerbosity?: CodegenTypes.WithDefault<CodegenTypes.Int32, 1>;
  onMathSize?: CodegenTypes.DirectEventHandler<SizeEvent>;
  onMathTap?: CodegenTypes.DirectEventHandler<TapEvent>;
  onMathError?: CodegenTypes.DirectEventHandler<ErrorEvent>;
}

export default codegenNativeComponent<NativeProps>('MathCoreView');
