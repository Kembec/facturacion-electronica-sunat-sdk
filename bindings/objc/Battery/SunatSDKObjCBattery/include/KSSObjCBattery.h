#import <Foundation/Foundation.h>

NS_ASSUME_NONNULL_BEGIN

/// What one run of the battery found.
@interface KSSObjCBatteryReport : NSObject
@property (nonatomic, readonly) NSInteger checks;
@property (nonatomic, copy, readonly) NSArray<NSString *> *failures;
@property (nonatomic, readonly) NSInteger vectors;
/// The line a runner prints: "ok Objective-C: …" or "Objective-C: n of m checks failed, …".
@property (nonatomic, copy, readonly) NSString *summary;
- (instancetype)init NS_UNAVAILABLE;
@end

/// The battery of the Objective-C package as a consumer links it: the 38 fixtures against their vectors (inclusive C14N,
/// exact DigestValue and SignatureValue), the native version against the package's, zip and CDR, the errors, no_verdict
/// without an answer and concurrent calls. The same checks as the Swift battery, through the other product.
@interface KSSObjCBattery : NSObject
@property (class, nonatomic, readonly) NSInteger expectedFixtures;
/// repository: the checked-out repository (fixtures, vectors, the test certificate, the recorded CDR).
+ (KSSObjCBatteryReport *)runWithRepository:(NSURL *)repository;
@end

NS_ASSUME_NONNULL_END
