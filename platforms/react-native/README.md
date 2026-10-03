# react-native-mathcore

Native TeX math for React Native on iOS and Android: the mathcore engine's
own views behind a Fabric component. No WebView, no fonts to bundle.

```sh
npm install react-native-mathcore
cd ios && bundle exec pod install
```

```tsx
import { MathText, speech, asciimathToTex } from 'react-native-mathcore';

<MathText latex="x = \frac{-b \pm \sqrt{b^2-4ac}}{2a}" fontSize={22} />
<MathText latex="\ce{2H2 + O2 -> 2H2O}" />
<MathText latex="sum_(i=1)^n i^2" asciimath />
<MathText latex={tex} onTap={(region, source) => console.log(source)} />
```

- Sizes itself to the formula. With `wrap` (the default) it takes the
  container's width and breaks a formula too wide for it into lines.
- VoiceOver and TalkBack read the formula as a sentence, then let the user
  step through its parts; `speechVerbosity` sets how much they hear.
- `onTap` gives the source under the finger; `onError` the parse error.
- `speech`, `speechTree`, `mathml` and `asciimathToTex` are synchronous
  calls into the engine.

Requires React Native 0.80+ with the New Architecture (the default), iOS
15.1+, Android API 24+.

Built by `cargo xtask sdk react-native` in the mathcore repository, which
vendors the engine (a dynamic `MathCoreFFI.xcframework` for iOS, `.so`
libraries for Android) and the native view sources into the package.
