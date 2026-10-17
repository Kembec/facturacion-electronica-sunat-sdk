#import <Foundation/Foundation.h>

NS_ASSUME_NONNULL_BEGIN

// Internal: one call of the C ABI (docs/c-abi.md) and the reading of its response. Not a public header.

/// An NSError of KSSErrorDomain; command is nil for errors raised before any call.
NSError *KSSMakeError(NSInteger code, NSString *_Nullable command, NSString *message);

/// The request as UTF-8 bytes, written by hand so the document reaches the command as the caller wrote it and the secrets
/// live in one buffer that is overwritten after the call.
@interface KSSRequest : NSObject
- (void)raw:(NSString *)text;
- (void)string:(NSString *)text;
/// An object of the given members, in order: each @[name, value], the value an NSString or a whole NSNumber.
- (void)object:(NSArray<NSArray *> *)members;
@end

/// A JSON object of the response, read field by field. A field of the wrong shape is recorded as a problem (shared by
/// every nested reader) and read as empty, so a parse runs to its end and the first problem is the error.
@interface KSSFields : NSObject
@property (nonatomic, readonly) NSDictionary<NSString *, id> *dictionary;
- (NSString *)string:(NSString *)key;
- (nullable NSString *)optionalString:(NSString *)key;
- (NSInteger)integer:(NSString *)key;
- (nullable NSNumber *)optionalInteger:(NSString *)key;
- (BOOL)boolean:(NSString *)key;
/// An array of strings; absent or null is empty.
- (NSArray<NSString *> *)strings:(NSString *)key;
- (NSData *)base64:(NSString *)key;
- (nullable NSData *)optionalBase64:(NSString *)key;
- (KSSFields *)object:(NSString *)key;
- (nullable KSSFields *)optionalObject:(NSString *)key;
/// An array of objects; absent or null is empty.
- (NSArray<KSSFields *> *)objects:(NSString *)key;
/// The position + 1 of the member's value in `values`; 0 (the enum's Unknown) when absent, null or not listed.
- (NSInteger)enumeration:(NSString *)key values:(NSArray<NSString *> *)values;
/// Records that the member `key` has a value the contract does not have (an unknown outcome).
- (void)reject:(NSString *)key because:(NSString *)reason;
/// This object as JSON text.
- (NSString *)JSONText;
@end

typedef id _Nullable (^KSSParse)(KSSFields *fields);

/// The C ABI's version and the SDK's, once per process: nil and *error when the library is not the one this package was
/// built for.
NSString *_Nullable KSSLoadedVersion(NSError **error);

/// Runs `command` and reads its response with `parse`. Status 1 is KSSErrorCommandFailed; a response that is not the
/// contract's shape is KSSErrorUnexpectedResponse. Top-level results get the response in rawJSON from the caller.
id _Nullable KSSRun(NSString *command, NSArray<NSArray *> *flags, void (^input)(KSSRequest *request), KSSParse parse,
                    NSString *_Nullable *_Nullable rawJSON, NSError **error);

NS_ASSUME_NONNULL_END
