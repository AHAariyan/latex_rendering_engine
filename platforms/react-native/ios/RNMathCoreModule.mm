#import "RNMathCoreModule.h"

#if __has_include(<react_native_mathcore/react_native_mathcore-Swift.h>)
#import <react_native_mathcore/react_native_mathcore-Swift.h>
#else
#import "react_native_mathcore-Swift.h"
#endif

@implementation RNMathCoreModule

RCT_EXPORT_MODULE(MathCore)

static NSString *orThrow(NSString *result, NSError *error)
{
  if (!result) {
    @throw [NSException exceptionWithName:@"MathParseException" reason:error.localizedDescription userInfo:nil];
  }
  return result;
}

- (NSString *)speech:(NSString *)tex verbosity:(double)verbosity
{
  NSError *error = nil;
  return orThrow([RNMathCoreBridge speech:tex verbosity:verbosity error:&error], error);
}

- (NSString *)speechTree:(NSString *)tex verbosity:(double)verbosity
{
  NSError *error = nil;
  return orThrow([RNMathCoreBridge speechTreeJson:tex verbosity:verbosity error:&error], error);
}

- (NSString *)mathml:(NSString *)tex displayMode:(BOOL)displayMode
{
  NSError *error = nil;
  return orThrow([RNMathCoreBridge mathml:tex displayMode:displayMode error:&error], error);
}

- (NSString *)asciimathToTex:(NSString *)source
{
  NSError *error = nil;
  return orThrow([RNMathCoreBridge asciimathToTex:source error:&error], error);
}

- (std::shared_ptr<facebook::react::TurboModule>)getTurboModule:(const facebook::react::ObjCTurboModule::InitParams &)params
{
  return std::make_shared<facebook::react::NativeMathCoreSpecJSI>(params);
}

@end
