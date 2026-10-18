#import <KembecSunatSDKObjC/KSSClient.h>

#import <KembecSunatSDKObjC/KSSErrors.h>
#import "KSSResults+Private.h"

NSString *const KSSPackageVersion = @"0.1.0-beta.7";

/// Sets *error to KSSErrorInvalidArgument when `value` is nil and says so; ObjC lets nil through a nonnull parameter.
#define KSS_REQUIRE(value)                                                                                   \
    if ((value) == nil) {                                                                                    \
        if (error != NULL) *error = KSSMakeError(KSSErrorInvalidArgument, nil, @#value " must not be nil"); \
        return nil;                                                                                          \
    }

@implementation KSSClient {
    int64_t _timeoutSeconds;
}

+ (NSTimeInterval)defaultHTTPTimeout {
    return 90;
}

- (instancetype)init {
    return [self initWithHTTPTimeout:KSSClient.defaultHTTPTimeout error:NULL];
}

- (instancetype)initWithHTTPTimeout:(NSTimeInterval)httpTimeout error:(NSError **)error {
    if (!(httpTimeout >= 1 && httpTimeout <= 300 && httpTimeout == floor(httpTimeout))) {
        if (error != NULL) {
            NSString *message = [NSString stringWithFormat:@"httpTimeout must be whole seconds from 1 to 300, not %g", httpTimeout];
            *error = KSSMakeError(KSSErrorInvalidArgument, nil, message);
        }
        return nil;
    }
    if ((self = [super init])) {
        _httpTimeout = httpTimeout;
        _timeoutSeconds = (int64_t)httpTimeout;
    }
    return self;
}

+ (NSString *)nativeVersionWithError:(NSError **)error {
    return KSSLoadedVersion(error);
}

/// Runs a command whose result is a class with rawJSON, and sets it.
- (nullable id)run:(NSString *)command
             flags:(NSArray<NSArray *> *)flags
             input:(void (^)(KSSRequest *))input
             parse:(KSSParse)parse
             error:(NSError **)error {
    NSString *raw = nil;
    id result = KSSRun(command, flags, input, parse, &raw, error);
    [result setValue:raw forKey:@"rawJSON"];
    return result;
}

- (KSSBuildResult *)buildDocumentJSON:(NSString *)documentJSON error:(NSError **)error {
    KSS_REQUIRE(documentJSON);
    NSError *invalid = nil;
    NSData *data = [documentJSON dataUsingEncoding:NSUTF8StringEncoding];
    if ([NSJSONSerialization JSONObjectWithData:data options:NSJSONReadingFragmentsAllowed error:&invalid] == nil) {
        if (error != NULL) {
            NSString *why = invalid.userInfo[NSDebugDescriptionErrorKey] ?: invalid.localizedDescription;
            *error = KSSMakeError(KSSErrorInvalidArgument, nil, [NSString stringWithFormat:@"documentJSON is not one JSON value: %@", why]);
        }
        return nil;
    }
    return [self run:@"build" flags:@[] input:^(KSSRequest *request) { [request raw:documentJSON]; }
               parse:^id(KSSFields *fields) { return [[KSSBuildResult alloc] initWithFields:fields]; } error:error];
}

- (KSSSignResult *)signXML:(NSString *)unsignedXML key:(KSSSigningKey *)key signatureID:(NSString *)signatureID error:(NSError **)error {
    KSS_REQUIRE(unsignedXML);
    KSS_REQUIRE(key);
    NSArray *flags = @[@[@"cert", key.certificatePath], @[@"key", key.privateKeyPath]];
    NSMutableArray *input = [NSMutableArray arrayWithObject:@[@"xml", unsignedXML]];
    if (signatureID != nil) [input addObject:@[@"signature_id", signatureID]];
    return [self run:@"sign" flags:flags input:^(KSSRequest *request) { [request object:input]; }
               parse:^id(KSSFields *fields) { return [[KSSSignResult alloc] initWithFields:fields]; } error:error];
}

- (KSSZipResult *)zipXML:(NSString *)xml name:(NSString *)name error:(NSError **)error {
    KSS_REQUIRE(xml);
    KSS_REQUIRE(name);
    return [self run:@"zip" flags:@[] input:^(KSSRequest *request) { [request object:@[@[@"name", name], @[@"xml", xml]]]; }
               parse:^id(KSSFields *fields) { return [[KSSZipResult alloc] initWithFields:fields]; } error:error];
}

- (KSSCdr *)readCDR:(NSData *)cdr error:(NSError **)error {
    KSS_REQUIRE(cdr);
    NSString *encoded = [cdr base64EncodedStringWithOptions:0];
    return [self run:@"cdr" flags:@[] input:^(KSSRequest *request) { [request object:@[@[@"cdr_base64", encoded]]]; }
               parse:^id(KSSFields *fields) { return [[KSSCdr alloc] initWithFields:fields]; } error:error];
}

- (NSArray<NSArray *> *)soap:(KSSSOLCredentials *)credentials endpoint:(NSURL *)endpoint {
    return @[@[@"endpoint", endpoint.absoluteString ?: @""], @[@"user", credentials.user], @[@"password", credentials.password],
             @[@"timeout", @(_timeoutSeconds)]];
}

- (NSArray<NSArray *> *)gre:(KSSAPICredentials *)api sol:(KSSSOLCredentials *)sol tokenEndpoint:(NSURL *)tokenEndpoint endpoint:(NSURL *)endpoint {
    return @[@[@"client-id", api.clientID], @[@"client-secret", api.clientSecret], @[@"user", sol.user], @[@"password", sol.password],
             @[@"token-endpoint", tokenEndpoint.absoluteString ?: @""], @[@"endpoint", endpoint.absoluteString ?: @""], @[@"timeout", @(_timeoutSeconds)]];
}

- (KSSSendResult *)sendXML:(NSString *)signedXML name:(NSString *)name credentials:(KSSSOLCredentials *)credentials
                  endpoint:(NSURL *)endpoint error:(NSError **)error {
    KSS_REQUIRE(signedXML);
    KSS_REQUIRE(name);
    KSS_REQUIRE(credentials);
    KSS_REQUIRE(endpoint);
    return [self run:@"send" flags:[self soap:credentials endpoint:endpoint]
               input:^(KSSRequest *request) { [request object:@[@[@"name", name], @[@"xml", signedXML]]]; }
               parse:^id(KSSFields *fields) { return [KSSSendResult resultWithFields:fields]; } error:error];
}

- (KSSSummaryResult *)sendSummaryXML:(NSString *)signedXML name:(NSString *)name credentials:(KSSSOLCredentials *)credentials
                            endpoint:(NSURL *)endpoint error:(NSError **)error {
    KSS_REQUIRE(signedXML);
    KSS_REQUIRE(name);
    KSS_REQUIRE(credentials);
    KSS_REQUIRE(endpoint);
    return [self run:@"summary" flags:[self soap:credentials endpoint:endpoint]
               input:^(KSSRequest *request) { [request object:@[@[@"name", name], @[@"xml", signedXML]]]; }
               parse:^id(KSSFields *fields) { return [KSSSummaryResult resultWithFields:fields]; } error:error];
}

- (KSSTicketResult *)ticketStatus:(NSString *)ticket credentials:(KSSSOLCredentials *)credentials endpoint:(NSURL *)endpoint
                            error:(NSError **)error {
    KSS_REQUIRE(ticket);
    KSS_REQUIRE(credentials);
    KSS_REQUIRE(endpoint);
    return [self run:@"ticket" flags:[self soap:credentials endpoint:endpoint]
               input:^(KSSRequest *request) { [request object:@[@[@"ticket", ticket]]]; }
               parse:^id(KSSFields *fields) { return [KSSTicketResult resultWithFields:fields]; } error:error];
}

- (NSMutableArray<NSArray *> *)voucherKey:(NSString *)ruc documentType:(NSString *)documentType series:(NSString *)series number:(int64_t)number {
    return [NSMutableArray arrayWithArray:@[@[@"ruc", ruc], @[@"document_type", documentType], @[@"series", series], @[@"number", @(number)]]];
}

- (KSSStatusResult *)statusForRUC:(NSString *)ruc documentType:(NSString *)documentType series:(NSString *)series number:(int64_t)number
                      credentials:(KSSSOLCredentials *)credentials endpoint:(NSURL *)endpoint error:(NSError **)error {
    KSS_REQUIRE(ruc);
    KSS_REQUIRE(documentType);
    KSS_REQUIRE(series);
    KSS_REQUIRE(credentials);
    KSS_REQUIRE(endpoint);
    NSArray *input = [self voucherKey:ruc documentType:documentType series:series number:number];
    return [self run:@"status" flags:[self soap:credentials endpoint:endpoint] input:^(KSSRequest *request) { [request object:input]; }
               parse:^id(KSSFields *fields) { return [KSSStatusResult resultWithFields:fields]; } error:error];
}

- (KSSValidityResult *)validityForRUC:(NSString *)ruc documentType:(NSString *)documentType series:(NSString *)series
                               number:(int64_t)number issueDate:(NSDateComponents *)issueDate total:(NSDecimalNumber *)total
                          credentials:(KSSAPICredentials *)credentials requesterRUC:(NSString *)requesterRUC
                        tokenEndpoint:(NSURL *)tokenEndpoint endpoint:(NSURL *)endpoint error:(NSError **)error {
    KSS_REQUIRE(ruc);
    KSS_REQUIRE(documentType);
    KSS_REQUIRE(series);
    KSS_REQUIRE(issueDate);
    KSS_REQUIRE(total);
    KSS_REQUIRE(credentials);
    NSInteger year = issueDate.year, month = issueDate.month, day = issueDate.day;
    if (year < 1 || year > 9999 || month < 1 || month > 12 || day < 1 || day > 31) {
        if (error != NULL) *error = KSSMakeError(KSSErrorInvalidArgument, nil, @"issueDate needs a year, a month and a day");
        return nil;
    }
    if ([total isEqualToNumber:[NSDecimalNumber notANumber]]) {
        if (error != NULL) *error = KSSMakeError(KSSErrorInvalidArgument, nil, @"total is not a number");
        return nil;
    }
    NSMutableArray *flags = [NSMutableArray arrayWithArray:@[@[@"client-id", credentials.clientID],
                                                             @[@"client-secret", credentials.clientSecret],
                                                             @[@"timeout", @(_timeoutSeconds)]]];
    if (tokenEndpoint != nil) [flags addObject:@[@"token-endpoint", tokenEndpoint.absoluteString ?: @""]];
    if (endpoint != nil) [flags addObject:@[@"endpoint", endpoint.absoluteString ?: @""]];
    NSMutableArray *input = [self voucherKey:ruc documentType:documentType series:series number:number];
    [input addObject:@[@"issue_date", [NSString stringWithFormat:@"%04ld-%02ld-%02ld", (long)year, (long)month, (long)day]]];
    // The decimal's own digits, never a binary double: 118.10 stays 118.1, not 118.09999999999999.
    [input addObject:@[@"total", [total descriptionWithLocale:[NSLocale localeWithLocaleIdentifier:@"en_US_POSIX"]]]];
    if (requesterRUC != nil) [input addObject:@[@"requester_ruc", requesterRUC]];
    return [self run:@"validity" flags:flags input:^(KSSRequest *request) { [request object:input]; }
               parse:^id(KSSFields *fields) { return [KSSValidityResult resultWithFields:fields]; } error:error];
}

- (KSSGRESendResult *)greSendXML:(NSString *)signedXML name:(NSString *)name apiCredentials:(KSSAPICredentials *)apiCredentials
                  solCredentials:(KSSSOLCredentials *)solCredentials tokenEndpoint:(NSURL *)tokenEndpoint endpoint:(NSURL *)endpoint
                           error:(NSError **)error {
    KSS_REQUIRE(signedXML);
    KSS_REQUIRE(name);
    KSS_REQUIRE(apiCredentials);
    KSS_REQUIRE(solCredentials);
    KSS_REQUIRE(tokenEndpoint);
    KSS_REQUIRE(endpoint);
    return [self run:@"gre-send" flags:[self gre:apiCredentials sol:solCredentials tokenEndpoint:tokenEndpoint endpoint:endpoint]
               input:^(KSSRequest *request) { [request object:@[@[@"name", name], @[@"xml", signedXML]]]; }
               parse:^id(KSSFields *fields) { return [KSSGRESendResult resultWithFields:fields]; } error:error];
}

- (KSSGRETicketResult *)greTicket:(NSString *)ticket apiCredentials:(KSSAPICredentials *)apiCredentials
                   solCredentials:(KSSSOLCredentials *)solCredentials tokenEndpoint:(NSURL *)tokenEndpoint endpoint:(NSURL *)endpoint
                            error:(NSError **)error {
    KSS_REQUIRE(ticket);
    KSS_REQUIRE(apiCredentials);
    KSS_REQUIRE(solCredentials);
    KSS_REQUIRE(tokenEndpoint);
    KSS_REQUIRE(endpoint);
    return [self run:@"gre-ticket" flags:[self gre:apiCredentials sol:solCredentials tokenEndpoint:tokenEndpoint endpoint:endpoint]
               input:^(KSSRequest *request) { [request object:@[@[@"ticket", ticket]]]; }
               parse:^id(KSSFields *fields) { return [KSSGRETicketResult resultWithFields:fields]; } error:error];
}

@end
