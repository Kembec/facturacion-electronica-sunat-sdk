#import "KSSBridge.h"
#import <KembecSunatSDKObjC/KSSNetworkResults.h>
#import <KembecSunatSDKObjC/KSSResults.h>

NS_ASSUME_NONNULL_BEGIN

// Internal: each result read from its JSON object (KSSFields), and rawJSON set by the client on the top-level ones.

@interface KSSBuildResult ()
@property (nonatomic, copy, readwrite) NSString *rawJSON;
- (instancetype)initWithFields:(KSSFields *)fields;
@end
@interface KSSSignResult ()
@property (nonatomic, copy, readwrite) NSString *rawJSON;
- (instancetype)initWithFields:(KSSFields *)fields;
@end
@interface KSSZipResult ()
@property (nonatomic, copy, readwrite) NSString *rawJSON;
- (instancetype)initWithFields:(KSSFields *)fields;
@end
@interface KSSCdr ()
@property (nonatomic, copy, readwrite) NSString *rawJSON;
/// A nested CDR keeps its member's JSON in rawJSON.
- (instancetype)initWithFields:(KSSFields *)fields;
@end
@interface KSSFault ()
- (instancetype)initWithFields:(KSSFields *)fields;
@end
@interface KSSNoVerdict ()
- (instancetype)initWithFields:(KSSFields *)fields;
@end

@interface KSSSendResult ()
@property (nonatomic, copy, readwrite) NSString *rawJSON;
+ (nullable instancetype)resultWithFields:(KSSFields *)fields;
@end
@interface KSSSummaryResult ()
@property (nonatomic, copy, readwrite) NSString *rawJSON;
+ (nullable instancetype)resultWithFields:(KSSFields *)fields;
@end
@interface KSSTicketResult ()
@property (nonatomic, copy, readwrite) NSString *rawJSON;
+ (nullable instancetype)resultWithFields:(KSSFields *)fields;
@end
@interface KSSStatusResult ()
@property (nonatomic, copy, readwrite) NSString *rawJSON;
+ (nullable instancetype)resultWithFields:(KSSFields *)fields;
@end
@interface KSSValidityResult ()
@property (nonatomic, copy, readwrite) NSString *rawJSON;
+ (nullable instancetype)resultWithFields:(KSSFields *)fields;
@end
@interface KSSGRESendResult ()
@property (nonatomic, copy, readwrite) NSString *rawJSON;
+ (nullable instancetype)resultWithFields:(KSSFields *)fields;
@end
@interface KSSGRETicketResult ()
@property (nonatomic, copy, readwrite) NSString *rawJSON;
+ (nullable instancetype)resultWithFields:(KSSFields *)fields;
@end

/// The wire values of the contract's enumerations, in the order of their NS_ENUM after Unknown.
FOUNDATION_EXPORT NSArray<NSString *> *KSSResponseClassValues(void);
FOUNDATION_EXPORT NSArray<NSString *> *KSSTicketStateValues(void);
FOUNDATION_EXPORT NSArray<NSString *> *KSSNoVerdictReasonValues(void);
FOUNDATION_EXPORT NSArray<NSString *> *KSSExchangeStageValues(void);

NS_ASSUME_NONNULL_END
