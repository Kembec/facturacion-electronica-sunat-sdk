#import "KSSObjCBattery.h"

#import <KembecSunatSDKObjC/KembecSunatSDKObjC.h>

#import "diagnosis.h"

#include "battery_support.h"

static const NSInteger KSSExpectedFixtures = 38;

#pragma mark - Report and checker

@interface KSSObjCBatteryReport ()
- (instancetype)initWithChecks:(NSInteger)checks failures:(NSArray *)failures vectors:(NSInteger)vectors summary:(NSString *)summary;
@end

@implementation KSSObjCBatteryReport

- (instancetype)initWithChecks:(NSInteger)checks failures:(NSArray *)failures vectors:(NSInteger)vectors summary:(NSString *)summary {
    if ((self = [super init])) {
        _checks = checks;
        _failures = [failures copy];
        _vectors = vectors;
        _summary = [summary copy];
    }
    return self;
}

@end

/// Counts checks and collects failures; safe from several threads.
@interface KSSChecker : NSObject
@property (nonatomic, readonly) NSInteger checks;
@property (nonatomic, readonly) NSMutableArray<NSString *> *failures;
@end

@implementation KSSChecker

- (instancetype)init {
    if ((self = [super init])) _failures = [NSMutableArray array];
    return self;
}

- (void)check:(BOOL)ok what:(NSString *)what {
    @synchronized(self) {
        _checks++;
        if (!ok) [_failures addObject:what];
    }
}

/// `error` must be of KSSErrorDomain with `code`, and its message must contain `text` when given.
- (void)expectError:(nullable NSError *)error code:(KSSErrorCode)code containing:(nullable NSString *)text what:(NSString *)what {
    BOOL ok = [error.domain isEqualToString:KSSErrorDomain] && error.code == code &&
              (text == nil || [error.localizedDescription containsString:text]);
    [self check:ok what:[NSString stringWithFormat:@"%@: %@", what, error ?: @"no error"]];
}

@end

#pragma mark - Repository and C14N

static NSString *KSSText(NSURL *repository, NSString *path) {
    return [NSString stringWithContentsOfURL:[repository URLByAppendingPathComponent:path] encoding:NSUTF8StringEncoding error:NULL];
}

static NSString *KSSVector(NSURL *repository, NSString *name, NSString *kind) {
    return KSSText(repository, [NSString stringWithFormat:@"tests/vectors/%@.%@.xml", name, kind]);
}

/// Inclusive C14N 1.0 without comments, from libxml2; nil if the text is not XML.
static NSString *KSSCanonical(NSString *xml) {
    if (xml == nil) return nil;
    const char *utf8 = xml.UTF8String;
    char *out = battery_c14n(utf8, strlen(utf8));
    if (out == NULL) return nil;
    NSString *text = [NSString stringWithUTF8String:out];
    battery_free(out);
    return text;
}

static BOOL KSSSameCanonical(NSString *left, NSString *right) {
    NSString *a = KSSCanonical(left), *b = KSSCanonical(right);
    return a != nil && [a isEqualToString:b];
}

/// ruc-type-series-number, or ruc-RC|RA-YYYYMMDD-number, from the fixture's own fields, read with Foundation's parser.
static NSString *KSSExpectedFileName(NSString *document) {
    NSDictionary *object = [NSJSONSerialization JSONObjectWithData:[document dataUsingEncoding:NSUTF8StringEncoding] options:0 error:NULL];
    NSDictionary *issuer = object[@"issuer"];
    if (![object isKindOfClass:[NSDictionary class]] || ![issuer isKindOfClass:[NSDictionary class]]) return @"";
    NSString *(^field)(id) = ^NSString *(id value) { return value == nil ? @"" : [NSString stringWithFormat:@"%@", value]; };
    NSString *ruc = field(issuer[@"ruc"]), *number = field(object[@"number"]), *kind = field(object[@"kind"]);
    if ([kind isEqualToString:@"summary"] || [kind isEqualToString:@"voided"]) {
        BOOL summary = [kind isEqualToString:@"summary"];
        NSString *date = field(object[summary ? @"generation_date" : @"communication_date"]);
        return [NSString stringWithFormat:@"%@-%@-%@-%@", ruc, summary ? @"RC" : @"RA",
                                          [date stringByReplacingOccurrencesOfString:@"-" withString:@""], number];
    }
    return [NSString stringWithFormat:@"%@-%@-%@-%@", ruc, field(object[@"document_type"]), field(object[@"series"]), number];
}

static NSString *KSSPlatform(void) {
#if TARGET_OS_SIMULATOR
    NSString *system = @"iOS simulator";
#elif TARGET_OS_IOS
    NSString *system = @"iOS";
#elif TARGET_OS_OSX
    NSString *system = @"macOS";
#else
    NSString *system = @"other";
#endif
#if defined(__arm64__)
    NSString *arch = @"arm64";
#elif defined(__x86_64__)
    NSString *arch = @"x86_64";
#else
    NSString *arch = @"other";
#endif
    return [NSString stringWithFormat:@"%@ %@ %@", system, NSProcessInfo.processInfo.operatingSystemVersionString, arch];
}

#pragma mark - Battery

@implementation KSSObjCBattery {
    KSSChecker *_checker;
    NSURL *_repository;
    KSSClient *_client;
    KSSSigningKey *_key;
}

+ (NSInteger)expectedFixtures {
    return KSSExpectedFixtures;
}

+ (KSSObjCBatteryReport *)runWithRepository:(NSURL *)repository {
    KSSObjCBattery *battery = [[KSSObjCBattery alloc] init];
    battery->_checker = [[KSSChecker alloc] init];
    battery->_repository = repository.URLByStandardizingPath;
    return [battery run];
}

- (KSSObjCBatteryReport *)run {
    NSURL *certs = [_repository URLByAppendingPathComponent:@"tests/vectors/test-cert"];
    _key = [[KSSSigningKey alloc] initWithCertificatePath:[certs URLByAppendingPathComponent:@"test-cert.pem"].path
                                           privateKeyPath:[certs URLByAppendingPathComponent:@"test-key.pem"].path];
    NSError *error = nil;
    _client = [[KSSClient alloc] initWithHTTPTimeout:30 error:&error];
    if (_client == nil) {
        [_checker check:NO what:[NSString stringWithFormat:@"a client with a 30 s timeout: %@", error]];
        return [self reportWithVectors:0];
    }
    NSString *native = [KSSClient nativeVersionWithError:&error];
    [_checker check:[native isEqualToString:KSSPackageVersion]
               what:[NSString stringWithFormat:@"native %@, package %@ (%@)", native, KSSPackageVersion, error]];

    NSArray<NSString *> *fixtures = [self fixtures];
    [_checker check:(NSInteger)fixtures.count == KSSExpectedFixtures
               what:[NSString stringWithFormat:@"fixtures: %lu, expected %ld", (unsigned long)fixtures.count, (long)KSSExpectedFixtures]];
    for (NSString *fixture in fixtures) [self vector:fixture];
    [self zipAndCDR];
    [self errors];
    [self silent];
    KSSCheckDiagnosis(_repository, ^(BOOL ok, NSString *what) { [self->_checker check:ok what:what]; });
    [self threads:[fixtures subarrayWithRange:NSMakeRange(0, MIN((NSUInteger)8, fixtures.count))]];
    return [self reportWithVectors:(NSInteger)fixtures.count];
}

- (KSSObjCBatteryReport *)reportWithVectors:(NSInteger)vectors {
    NSArray *failures;
    NSInteger checks;
    @synchronized(_checker) {
        failures = [_checker.failures copy];
        checks = _checker.checks;
    }
    NSString *summary;
    if (failures.count == 0) {
        NSString *native = [KSSClient nativeVersionWithError:NULL] ?: @"not loaded";
        summary = [NSString stringWithFormat:@"ok Objective-C: %ld checks, %ld vectors, %@, native %@", (long)checks, (long)vectors,
                                             KSSPlatform(), native];
    } else {
        summary = [NSString stringWithFormat:@"Objective-C: %lu of %ld checks failed, %@", (unsigned long)failures.count, (long)checks,
                                             KSSPlatform()];
    }
    return [[KSSObjCBatteryReport alloc] initWithChecks:checks failures:failures vectors:vectors summary:summary];
}

/// The fixture names, sorted, without .json.
- (NSArray<NSString *> *)fixtures {
    NSString *directory = [_repository URLByAppendingPathComponent:@"fixtures"].path;
    NSMutableArray *names = [NSMutableArray array];
    for (NSString *name in [NSFileManager.defaultManager contentsOfDirectoryAtPath:directory error:NULL]) {
        if ([name hasSuffix:@".json"]) [names addObject:[name stringByDeletingPathExtension]];
    }
    return [names sortedArrayUsingSelector:@selector(compare:)];
}

- (NSString *)signedFixture:(NSString *)name {
    NSString *document = KSSText(_repository, [NSString stringWithFormat:@"fixtures/%@.json", name]);
    KSSBuildResult *built = document == nil ? nil : [_client buildDocumentJSON:document error:NULL];
    return built == nil ? nil : [_client signXML:built.xml key:_key signatureID:nil error:NULL].xml;
}

- (void)vector:(NSString *)name {
    NSError *error = nil;
    NSString *document = KSSText(_repository, [NSString stringWithFormat:@"fixtures/%@.json", name]);
    KSSBuildResult *built = document == nil ? nil : [_client buildDocumentJSON:document error:&error];
    if (built == nil) {
        [_checker check:NO what:[NSString stringWithFormat:@"%@: build: %@", name, error]];
        return;
    }
    [_checker check:KSSSameCanonical(KSSVector(_repository, name, @"unsigned"), built.xml)
               what:[NSString stringWithFormat:@"%@: unsigned XML differs from its vector under C14N", name]];
    [_checker check:[built.fileName isEqualToString:KSSExpectedFileName(document)]
               what:[NSString stringWithFormat:@"%@: file name %@", name, built.fileName]];
    [_checker check:built.rawJSON.length > 0 what:[NSString stringWithFormat:@"%@: rawJSON of build is empty", name]];

    KSSSignResult *signedResult = [_client signXML:built.xml key:_key signatureID:nil error:&error];
    if (signedResult == nil) {
        [_checker check:NO what:[NSString stringWithFormat:@"%@: sign: %@", name, error]];
        return;
    }
    NSString *expected = KSSVector(_repository, name, @"signed");
    NSString *digest = [NSString stringWithFormat:@"<ds:DigestValue>%@</ds:DigestValue>", signedResult.digestValue];
    NSString *signature = [NSString stringWithFormat:@"<ds:SignatureValue>%@</ds:SignatureValue>", signedResult.signatureValue];
    [_checker check:KSSSameCanonical(expected, signedResult.xml)
               what:[NSString stringWithFormat:@"%@: signed XML differs from its vector under C14N", name]];
    [_checker check:[expected containsString:digest] what:[NSString stringWithFormat:@"%@: DigestValue %@", name, signedResult.digestValue]];
    [_checker check:[expected containsString:signature] what:[NSString stringWithFormat:@"%@: SignatureValue differs", name]];
    [_checker check:[signedResult.xml containsString:digest] what:[NSString stringWithFormat:@"%@: DigestValue not in the signed XML", name]];
}

- (void)zipAndCDR {
    NSError *error = nil;
    NSString *signedXML = [self signedFixture:@"01-invoice-taxed-cash"];
    KSSZipResult *zip = signedXML == nil ? nil : [_client zipXML:signedXML name:@"20123456789-01-F001-1" error:&error];
    [_checker check:[zip.fileName isEqualToString:@"20123456789-01-F001-1.zip"] what:[NSString stringWithFormat:@"zip: %@ %@", zip.fileName, error]];
    [_checker check:zip.content.length > 4 && memcmp(zip.content.bytes, "PK", 2) == 0 what:@"zip: not a zip"];

    NSURL *recorded = [_repository URLByAppendingPathComponent:@"crates/sunat-transport/tests/data/R-20150147718-01-FT01-982.zip"];
    NSData *bytes = [NSData dataWithContentsOfURL:recorded];
    KSSCdr *cdr = bytes == nil ? nil : [_client readCDR:bytes error:&error];
    [_checker check:[cdr.referenceID isEqualToString:@"FT01-982"] what:[NSString stringWithFormat:@"cdr: reference %@ %@", cdr.referenceID, error]];
    [_checker check:[cdr.responseCode isEqualToString:@"2047"] what:[NSString stringWithFormat:@"cdr: code %@", cdr.responseCode]];
    [_checker check:cdr.responseClass == KSSResponseClassRejected && !cdr.isValidDocument
               what:[NSString stringWithFormat:@"cdr: class %ld", (long)cdr.responseClass]];
    [_checker check:cdr.rawJSON.length > 0 && [cdr.rawJSON containsString:@"FT01-982"] what:@"cdr: rawJSON"];
}

- (void)errors {
    NSError *error = nil;
    [_client buildDocumentJSON:@"{}" error:&error];
    [_checker expectError:error code:KSSErrorCommandFailed containing:nil what:@"build of {} is the SDK's error"];
    [_checker check:[error.userInfo[KSSErrorCommandKey] isEqualToString:@"build"] what:@"build of {} names its command"];

    error = nil;
    [_client buildDocumentJSON:@"{not json" error:&error];
    [_checker expectError:error code:KSSErrorInvalidArgument containing:@"documentJSON" what:@"invalid JSON names the parameter"];
    [_checker check:error.userInfo[KSSErrorCommandKey] == nil what:@"invalid JSON has no command"];

    error = nil;
    NSString *nothing = nil;
    [_client zipXML:nothing name:@"x" error:&error];
    [_checker expectError:error code:KSSErrorInvalidArgument containing:@"xml" what:@"nil xml"];

    error = nil;
    KSSClient *invalid = [[KSSClient alloc] initWithHTTPTimeout:1.5 error:&error];
    [_checker check:invalid == nil what:@"timeout of 1.5 s gave a client"];
    [_checker expectError:error code:KSSErrorInvalidArgument containing:@"httpTimeout" what:@"timeout of 1.5 s"];

    error = nil;
    NSDateComponents *noDay = [[NSDateComponents alloc] init];
    noDay.year = 2026;
    noDay.month = 10;
    KSSAPICredentials *api = [[KSSAPICredentials alloc] initWithClientID:@"id" clientSecret:@"api-secret"];
    [_client validityForRUC:@"20123456789" documentType:@"01" series:@"F001" number:1 issueDate:noDay
                      total:[NSDecimalNumber decimalNumberWithString:@"118"] credentials:api requesterRUC:nil tokenEndpoint:nil
                   endpoint:nil error:&error];
    [_checker expectError:error code:KSSErrorInvalidArgument containing:@"issueDate" what:@"issue date without a day"];

    KSSSOLCredentials *sol = [[KSSSOLCredentials alloc] initWithUser:@"20123456789MODDATOS" password:@"moddatos-secret"];
    [_checker check:![sol.description containsString:@"moddatos-secret"] && ![sol.debugDescription containsString:@"moddatos-secret"] &&
                    ![api.description containsString:@"api-secret"]
               what:@"credentials show their secret"];
    error = nil;
    NSString *signedXML = [self signedFixture:@"01-invoice-taxed-cash"] ?: @"";
    [_client sendXML:signedXML name:@"20123456789-01-F001-1" credentials:sol endpoint:[NSURL URLWithString:@"http://127.0.0.1/billService"]
               error:&error];
    [_checker expectError:error code:KSSErrorCommandFailed containing:@"endpoint must use https://" what:@"send over http://"];
    [_checker check:[error.userInfo[KSSErrorCommandKey] isEqualToString:@"send"] && ![error.localizedDescription containsString:@"moddatos-secret"]
               what:[NSString stringWithFormat:@"send over http://: %@", error]];
}

/// The network commands against a listener that never answers: each query waits out its 1 s and has no verdict.
- (void)silent {
    int port = battery_silent_listener_start();
    if (port <= 0) {
        [_checker check:NO what:@"no_verdict: no local listener"];
        return;
    }
    NSString *silent = [NSString stringWithFormat:@"https://127.0.0.1:%d", port];
    NSURL *(^at)(NSString *) = ^NSURL *(NSString *path) { return [NSURL URLWithString:[silent stringByAppendingString:path]]; };
    KSSClient *client = [[KSSClient alloc] initWithHTTPTimeout:1 error:NULL];
    KSSSOLCredentials *sol = [[KSSSOLCredentials alloc] initWithUser:@"20123456789MODDATOS" password:@"moddatos"];
    KSSAPICredentials *api = [[KSSAPICredentials alloc] initWithClientID:@"client-id" clientSecret:@"client-secret"];
    NSError *error = nil;

    KSSStatusResult *status = [client statusForRUC:@"20123456789" documentType:@"01" series:@"F001" number:1 credentials:sol
                                          endpoint:at(@"/billConsultService") error:&error];
    [_checker check:status.outcome == KSSOutcomeNoVerdict && status.noVerdict.reason == KSSNoVerdictReasonTimeout &&
                    status.noVerdict.httpStatus == nil && status.voucherStatus == nil && status.fault == nil
               what:[NSString stringWithFormat:@"status without an answer: %@ %@", status.rawJSON, error]];

    NSDateComponents *date = [[NSDateComponents alloc] init];
    date.year = 2026;
    date.month = 10;
    date.day = 1;
    KSSValidityResult *validity = [client validityForRUC:@"20123456789" documentType:@"01" series:@"F001" number:1 issueDate:date
                                                   total:[NSDecimalNumber decimalNumberWithString:@"118.00"] credentials:api
                                            requesterRUC:nil tokenEndpoint:at(@"/token") endpoint:at(@"/query") error:&error];
    [_checker check:validity.outcome == KSSOutcomeNoVerdict && validity.noVerdict.stage == KSSExchangeStageToken
               what:[NSString stringWithFormat:@"validity without a token: %@ %@", validity.rawJSON, error]];

    KSSGRETicketResult *gre = [client greTicket:@"1700000000000" apiCredentials:api solCredentials:sol tokenEndpoint:at(@"/token")
                                       endpoint:at(@"/v1") error:&error];
    [_checker check:gre.outcome == KSSOutcomeNoVerdict && gre.noVerdict.stage == KSSExchangeStageToken &&
                    [gre.ticket isEqualToString:@"1700000000000"]
               what:[NSString stringWithFormat:@"gre-ticket without a token: %@ %@", gre.rawJSON, error]];
    battery_silent_listener_stop();
}

/// Eight fixtures built and signed five times each, all at once: every signature is still its vector's.
- (void)threads:(NSArray<NSString *> *)chosen {
    NSMutableArray *digests = [NSMutableArray array];
    for (NSUInteger i = 0; i < chosen.count; i++) [digests addObject:[NSNull null]];
    dispatch_apply(chosen.count, dispatch_get_global_queue(QOS_CLASS_USER_INITIATED, 0), ^(size_t index) {
        NSString *document = KSSText(self->_repository, [NSString stringWithFormat:@"fixtures/%@.json", chosen[index]]);
        NSString *digest = nil;
        for (int round = 0; round < 5; round++) {
            KSSBuildResult *built = [self->_client buildDocumentJSON:document error:NULL];
            digest = built == nil ? nil : [self->_client signXML:built.xml key:self->_key signatureID:nil error:NULL].digestValue;
        }
        @synchronized(digests) {
            digests[index] = digest ?: [NSNull null];
        }
    });
    for (NSUInteger i = 0; i < chosen.count; i++) {
        id digest = digests[i];
        NSString *expected = KSSVector(_repository, chosen[i], @"signed");
        BOOL ok = [digest isKindOfClass:[NSString class]] &&
                  [expected containsString:[NSString stringWithFormat:@"<ds:DigestValue>%@</ds:DigestValue>", digest]];
        [_checker check:ok what:[NSString stringWithFormat:@"threads: %@", chosen[i]]];
    }
}

@end
