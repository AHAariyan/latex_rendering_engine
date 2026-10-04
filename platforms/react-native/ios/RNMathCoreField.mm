#import "RNMathCoreField.h"

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

@interface RNMathCoreField () <RCTMathCoreFieldViewProtocol>
@end

@implementation RNMathCoreField {
  RNMathCoreFieldHost *_host;
}

+ (ComponentDescriptorProvider)componentDescriptorProvider
{
  return concreteComponentDescriptorProvider<MathCoreFieldComponentDescriptor>();
}

- (instancetype)initWithFrame:(CGRect)frame
{
  if (self = [super initWithFrame:frame]) {
    static const auto defaultProps = std::make_shared<const MathCoreFieldProps>();
    _props = defaultProps;
    _host = [[RNMathCoreFieldHost alloc] initWithFrame:frame];
    __weak RNMathCoreField *weakSelf = self;
    _host.onSize = ^(CGFloat width, CGFloat height) {
      RNMathCoreField *strongSelf = weakSelf;
      if (strongSelf && strongSelf->_eventEmitter) {
        std::static_pointer_cast<const MathCoreFieldEventEmitter>(strongSelf->_eventEmitter)
            ->onMathSize({.width = static_cast<float>(width), .height = static_cast<float>(height)});
      }
    };
    _host.onChange = ^(NSString *latex) {
      RNMathCoreField *strongSelf = weakSelf;
      if (strongSelf && strongSelf->_eventEmitter) {
        std::static_pointer_cast<const MathCoreFieldEventEmitter>(strongSelf->_eventEmitter)
            ->onMathChange({.latex = std::string(latex.UTF8String)});
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

// Fabric attaches the event emitter after the first props: send the size again.
- (void)updateEventEmitter:(EventEmitter::Shared const &)eventEmitter
{
  [super updateEventEmitter:eventEmitter];
  CGSize size = _host.reported;
  if (size.width >= 0 && _eventEmitter) {
    std::static_pointer_cast<const MathCoreFieldEventEmitter>(_eventEmitter)
        ->onMathSize({.width = static_cast<float>(size.width), .height = static_cast<float>(size.height)});
  }
}

- (void)updateProps:(Props::Shared const &)props oldProps:(Props::Shared const &)oldProps
{
  const auto &p = *std::static_pointer_cast<MathCoreFieldProps const>(props);
  _host.value = [NSString stringWithUTF8String:p.value.c_str()];
  _host.fontSize = p.fontSize > 0 ? p.fontSize : 20;
  _host.color = p.color ? RCTUIColorFromSharedColor(p.color) : UIColor.labelColor;
  _host.cursorColor = p.cursorColor ? RCTUIColorFromSharedColor(p.cursorColor) : nil;
  _host.placeholder = [NSString stringWithUTF8String:p.placeholder.c_str()];
  _host.editable = p.editable;
  [_host commit];
  [super updateProps:props oldProps:oldProps];
}

- (void)handleCommand:(const NSString *)commandName args:(const NSArray *)args
{
  RCTMathCoreFieldHandleCommand(self, commandName, args);
}

- (void)runCommand:(NSString *)name
{
  [_host runCommand:name];
}

- (void)typeText:(NSString *)text
{
  [_host typeText:text];
}

- (void)focus
{
  [_host focusField];
}

- (void)blur
{
  [_host blurField];
}

@end

Class<RCTComponentViewProtocol> MathCoreFieldCls(void)
{
  return RNMathCoreField.class;
}
