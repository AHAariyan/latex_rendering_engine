import type { TurboModule } from 'react-native';
import { TurboModuleRegistry } from 'react-native';

export interface Spec extends TurboModule {
  speech(tex: string, verbosity: number, language: string): string;
  speechTree(tex: string, verbosity: number, language: string): string;
  mathml(tex: string, displayMode: boolean): string;
  asciimathToTex(source: string): string;
  nemeth(tex: string): string;
}

export default TurboModuleRegistry.getEnforcing<Spec>('MathCore');
