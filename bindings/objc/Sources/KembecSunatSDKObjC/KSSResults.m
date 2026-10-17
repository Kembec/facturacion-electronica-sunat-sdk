#import "KSSResults+Private.h"

NSArray<NSString *> *KSSResponseClassValues(void) {
    return @[@"accepted", @"observed", @"exception", @"already_registered", @"number_voided_or_rejected", @"rejected",
             @"void_already_reported"];
}

NSArray<NSString *> *KSSTicketStateValues(void) {
    return @[@"processed", @"in_process", @"processed_with_errors"];
}

NSArray<NSString *> *KSSNoVerdictReasonValues(void) {
    return @[@"timeout", @"http_status"];
}

NSArray<NSString *> *KSSExchangeStageValues(void) {
    return @[@"token", @"query", @"send", @"ticket"];
}

@implementation KSSBuildResult
- (instancetype)initWithFields:(KSSFields *)fields {
    if ((self = [super init])) {
        _fileName = [fields string:@"file_name"];
        _xml = [fields string:@"xml"];
        _rawJSON = @"";
    }
    return self;
}

@end

@implementation KSSSignResult
- (instancetype)initWithFields:(KSSFields *)fields {
    if ((self = [super init])) {
        _xml = [fields string:@"xml"];
        _digestValue = [fields string:@"digest_value"];
        _signatureValue = [fields string:@"signature_value"];
        _rawJSON = @"";
    }
    return self;
}

@end

@implementation KSSZipResult
- (instancetype)initWithFields:(KSSFields *)fields {
    if ((self = [super init])) {
        _fileName = [fields string:@"file_name"];
        _content = [fields base64:@"zip_base64"];
        _rawJSON = @"";
    }
    return self;
}

@end

@implementation KSSCdr
- (instancetype)initWithFields:(KSSFields *)fields {
    if ((self = [super init])) {
        _referenceID = [fields string:@"reference_id"];
        _responseCode = [fields string:@"response_code"];
        _responseDescription = [fields string:@"description"];
        _notes = [fields strings:@"notes"];
        _responseClass = [fields enumeration:@"class" values:KSSResponseClassValues()];
        _rawJSON = [fields JSONText];
    }
    return self;
}

- (BOOL)isAccepted {
    return self.responseClass == KSSResponseClassAccepted;
}

- (BOOL)isValidDocument {
    return self.responseClass == KSSResponseClassAccepted || self.responseClass == KSSResponseClassObserved;
}

@end

@implementation KSSFault

- (instancetype)initWithFields:(KSSFields *)fields {
    if ((self = [super init])) {
        _code = [fields string:@"code"];
        _message = [fields string:@"message"];
        _responseClass = [fields enumeration:@"class" values:KSSResponseClassValues()];
    }
    return self;
}

@end

@implementation KSSNoVerdict

- (instancetype)initWithFields:(KSSFields *)fields {
    if ((self = [super init])) {
        _reason = [fields enumeration:@"reason" values:KSSNoVerdictReasonValues()];
        _httpStatus = [fields optionalInteger:@"http_status"];
        _message = [fields string:@"message"];
        _stage = [fields enumeration:@"stage" values:KSSExchangeStageValues()];
    }
    return self;
}

@end

@implementation KSSDiagnosisDetail

- (instancetype)initWithFields:(KSSFields *)fields {
    if ((self = [super init])) {
        _node = [fields string:@"node"];
        _value = [fields string:@"value"];
    }
    return self;
}

@end

@implementation KSSDiagnosisField

- (instancetype)initWithFields:(KSSFields *)fields {
    if ((self = [super init])) {
        _inputPath = [fields optionalString:@"input_path"];
        _xpath = [fields optionalString:@"xpath"];
        _certainty = [fields enumeration:@"certainty" values:@[@"fuente", @"inferido"]];
    }
    return self;
}

@end

@implementation KSSDiagnosis

- (instancetype)initWithFields:(KSSFields *)fields {
    if ((self = [super init])) {
        _code = [fields string:@"code"];
        KSSFields *detail = [fields optionalObject:@"detail"];
        _detail = detail == nil ? nil : [[KSSDiagnosisDetail alloc] initWithFields:detail];
        NSMutableArray *list = [NSMutableArray array];
        for (KSSFields *field in [fields objects:@"fields"]) [list addObject:[[KSSDiagnosisField alloc] initWithFields:field]];
        _fields = [list copy];
    }
    return self;
}

@end

NSArray<KSSDiagnosis *> *KSSReadDiagnosis(KSSFields *fields) {
    NSMutableArray *list = [NSMutableArray array];
    for (KSSFields *item in [fields objects:@"diagnosis"]) [list addObject:[[KSSDiagnosis alloc] initWithFields:item]];
    return [list copy];
}
