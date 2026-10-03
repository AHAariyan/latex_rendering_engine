#import "RNMathCoreView.h"

#import <React/RCTConversions.h>
#import <react/renderer/components/RNMathcoreSpec/ComponentDescriptors.h>
#import <react/renderer/components/RNMathcoreSpec/EventEmitters.h>
#import <react/renderer/components/RNMathcoreSpec/Props.h>
#import <react/renderer/components/RNMathcoreSpec/RCTComponentViewHelpers.h>

#if __has_include(<react_native_mathcore/react_native_mathcore-Swift.h>)
#import <react_native_mathcore/react_native_mathcore-Swift.h>
#else
#import "react_native_mathcore-Swift.h"
#endif

using namespace facebook::react;

@interface RNMathCoreView () <RCTMathCoreViewViewProtocol>
@end

@implementation RNMathCoreView {
  RNMathCoreHostView *_host;
}

+ (ComponentDescriptorProvider)componentDescriptorProvider
{
  return concreteComponentDescriptorProvider<MathCoreViewComponentDescriptor>();
}

- (instancetype)initWithFrame:(CGRect)frame
{
  if (self = [super initWithFrame:frame]) {
    static const auto defaultProps = std::make_shared<const MathCoreViewProps>();
    _props = defaultProps;
    _host = [[RNMathCoreHostView alloc] initWithFrame:frame];
    __weak RNMathCoreView *weakSelf = self;
    _host.onSize = ^(CGFloat width, CGFloat height) {
      RNMathCoreView *strongSelf = weakSelf;
      if (strongSelf && strongSelf->_eventEmitter) {
        std::static_pointer_cast<const MathCoreViewEventEmitter>(strongSelf->_eventEmitter)
            ->onMathSize({.width = static_cast<float>(width), .height = static_cast<float>(height)});
      }
    };
    _host.onError = ^(NSString *message) {
      RNMathCoreView *strongSelf = weakSelf;
      if (strongSelf && strongSelf->_eventEmitter) {
        std::static_pointer_cast<const MathCoreViewEventEmitter>(strongSelf->_eventEmitter)
            ->onMathError({.message = std::string(message.UTF8String)});
      }
    };
    self.contentView = _host;
  }
  return self;
}

- (void)prepareForRecycle
{
  [super prepareForRecycle];
  [_host prepareForReuse];
}

// Fabric attaches the event emitter after the first props, so the size
// measured then went nowhere: send it again now.
- (void)updateEventEmitter:(EventEmitter::Shared const &)eventEmitter
{
  [super updateEventEmitter:eventEmitter];
  CGSize size = _host.reported;
  if (size.width >= 0 && _eventEmitter) {
    std::static_pointer_cast<const MathCoreViewEventEmitter>(_eventEmitter)
        ->onMathSize({.width = static_cast<float>(size.width), .height = static_cast<float>(size.height)});
  }
}

- (void)updateProps:(Props::Shared const &)props oldProps:(Props::Shared const &)oldProps
{
  const auto &p = *std::static_pointer_cast<MathCoreViewProps const>(props);
  _host.latex = [NSString stringWithUTF8String:p.latex.c_str()];
  _host.fontSize = p.fontSize > 0 ? p.fontSize : 17;
  _host.color = p.color ? RCTUIColorFromSharedColor(p.color) : UIColor.labelColor;
  _host.displayMode = p.displayMode;
  _host.wrap = p.wrap;
  _host.asciimath = p.asciimath;
  _host.speechVerbosity = p.speechVerbosity;
  __weak RNMathCoreView *weakSelf = self;
  _host.onTap = ^(NSInteger start, NSInteger end) {
    RNMathCoreView *strongSelf = weakSelf;
    if (strongSelf && strongSelf->_eventEmitter) {
      std::static_pointer_cast<const MathCoreViewEventEmitter>(strongSelf->_eventEmitter)
          ->onMathTap({.start = static_cast<int>(start), .end = static_cast<int>(end)});
    }
  };
  [_host commit];
  [super updateProps:props oldProps:oldProps];
}

@end

Class<RCTComponentViewProtocol> MathCoreViewCls(void)
{
  return RNMathCoreView.class;
}
