#import "KSSResults+Private.h"

// Each result reads `outcome` and fills the property of that outcome; an outcome the contract does not have is a problem,
// so the call fails with KSSErrorUnexpectedResponse rather than giving a result with every property nil.

static NSArray<NSString *> *KSSValidityStateValues(void) {
    return @[@"not_reported", @"accepted", @"voided", @"printed_authorized", @"printed_not_authorized"];
}

static void KSSUnknownOutcome(KSSFields *fields, NSString *outcome) {
    [fields reject:@"outcome" because:[NSString stringWithFormat:@"unknown outcome `%@`", outcome]];
}

static KSSCdr *_Nullable KSSOptionalCdr(KSSFields *fields) {
    KSSFields *cdr = [fields optionalObject:@"cdr"];
    return cdr == nil ? nil : [[KSSCdr alloc] initWithFields:cdr];
}

@interface KSSTicketStatus ()
- (instancetype)initWithFields:(KSSFields *)fields;
@end
@interface KSSVoucherStatus ()
- (instancetype)initWithFields:(KSSFields *)fields;
@end
@interface KSSValidity ()
- (instancetype)initWithFields:(KSSFields *)fields;
@end
@interface KSSQueryError ()
- (instancetype)initWithFields:(KSSFields *)fields;
@end
@interface KSSGREAPIErrorItem ()
- (instancetype)initWithFields:(KSSFields *)fields;
@end
@interface KSSGREAPIError ()
- (instancetype)initWithFields:(KSSFields *)fields;
@end
@interface KSSGRETicket ()
- (instancetype)initWithFields:(KSSFields *)fields;
@end
@interface KSSGRETicketError ()
- (instancetype)initWithFields:(KSSFields *)fields;
@end
@interface KSSGRETicketStatus ()
- (instancetype)initWithFields:(KSSFields *)fields;
@end

#pragma mark - SOAP

@implementation KSSSendResult

- (instancetype)initWithOutcome:(KSSOutcome)outcome fields:(KSSFields *)fields {
    if ((self = [super init])) {
        _outcome = outcome;
        _rawJSON = @"";
        if (outcome == KSSOutcomeCDR) {
            _cdr = [[KSSCdr alloc] initWithFields:[fields object:@"cdr"]];
            _cdrZip = [fields optionalBase64:@"cdr_zip_base64"];
        } else {
            _fault = [[KSSFault alloc] initWithFields:fields];
        }
    }
    return self;
}

+ (instancetype)resultWithFields:(KSSFields *)fields {
    NSString *outcome = [fields string:@"outcome"];
    if ([outcome isEqualToString:@"cdr"]) return [[self alloc] initWithOutcome:KSSOutcomeCDR fields:fields];
    if ([outcome isEqualToString:@"fault"]) return [[self alloc] initWithOutcome:KSSOutcomeFault fields:fields];
    KSSUnknownOutcome(fields, outcome);
    return nil;
}

@end

@implementation KSSSummaryResult

- (instancetype)initWithOutcome:(KSSOutcome)outcome fields:(KSSFields *)fields {
    if ((self = [super init])) {
        _outcome = outcome;
        _rawJSON = @"";
        if (outcome == KSSOutcomeTicket) {
            _ticket = [fields string:@"ticket"];
        } else {
            _fault = [[KSSFault alloc] initWithFields:fields];
        }
    }
    return self;
}

+ (instancetype)resultWithFields:(KSSFields *)fields {
    NSString *outcome = [fields string:@"outcome"];
    if ([outcome isEqualToString:@"ticket"]) return [[self alloc] initWithOutcome:KSSOutcomeTicket fields:fields];
    if ([outcome isEqualToString:@"fault"]) return [[self alloc] initWithOutcome:KSSOutcomeFault fields:fields];
    KSSUnknownOutcome(fields, outcome);
    return nil;
}

@end

@implementation KSSTicketStatus

- (instancetype)initWithFields:(KSSFields *)fields {
    if ((self = [super init])) {
        _statusCode = [fields string:@"status_code"];
        _state = [fields enumeration:@"state" values:KSSTicketStateValues()];
        _cdr = KSSOptionalCdr(fields);
        _cdrZip = [fields optionalBase64:@"cdr_zip_base64"];
    }
    return self;
}

@end

@implementation KSSTicketResult

- (instancetype)initWithOutcome:(KSSOutcome)outcome fields:(KSSFields *)fields {
    if ((self = [super init])) {
        _outcome = outcome;
        _rawJSON = @"";
        _ticket = [fields optionalString:@"ticket"];
        if (outcome == KSSOutcomeTicketStatus) {
            _ticketStatus = [[KSSTicketStatus alloc] initWithFields:fields];
        } else if (outcome == KSSOutcomeFault) {
            _fault = [[KSSFault alloc] initWithFields:fields];
        } else {
            _noVerdict = [[KSSNoVerdict alloc] initWithFields:fields];
        }
    }
    return self;
}

+ (instancetype)resultWithFields:(KSSFields *)fields {
    NSString *outcome = [fields string:@"outcome"];
    if ([outcome isEqualToString:@"ticket_status"]) return [[self alloc] initWithOutcome:KSSOutcomeTicketStatus fields:fields];
    if ([outcome isEqualToString:@"fault"]) return [[self alloc] initWithOutcome:KSSOutcomeFault fields:fields];
    if ([outcome isEqualToString:@"no_verdict"]) return [[self alloc] initWithOutcome:KSSOutcomeNoVerdict fields:fields];
    KSSUnknownOutcome(fields, outcome);
    return nil;
}

@end

@implementation KSSVoucherStatus

- (instancetype)initWithFields:(KSSFields *)fields {
    if ((self = [super init])) {
        _statusCode = [fields string:@"status_code"];
        _statusMessage = [fields optionalString:@"status_message"];
        _cdr = KSSOptionalCdr(fields);
        _cdrZip = [fields optionalBase64:@"cdr_zip_base64"];
    }
    return self;
}

@end

@implementation KSSStatusResult

- (instancetype)initWithOutcome:(KSSOutcome)outcome fields:(KSSFields *)fields {
    if ((self = [super init])) {
        _outcome = outcome;
        _rawJSON = @"";
        if (outcome == KSSOutcomeStatus) {
            _voucherStatus = [[KSSVoucherStatus alloc] initWithFields:fields];
        } else if (outcome == KSSOutcomeFault) {
            _fault = [[KSSFault alloc] initWithFields:fields];
        } else {
            _noVerdict = [[KSSNoVerdict alloc] initWithFields:fields];
        }
    }
    return self;
}

+ (instancetype)resultWithFields:(KSSFields *)fields {
    NSString *outcome = [fields string:@"outcome"];
    if ([outcome isEqualToString:@"status"]) return [[self alloc] initWithOutcome:KSSOutcomeStatus fields:fields];
    if ([outcome isEqualToString:@"fault"]) return [[self alloc] initWithOutcome:KSSOutcomeFault fields:fields];
    if ([outcome isEqualToString:@"no_verdict"]) return [[self alloc] initWithOutcome:KSSOutcomeNoVerdict fields:fields];
    KSSUnknownOutcome(fields, outcome);
    return nil;
}

@end

#pragma mark - Consulta Integrada

@implementation KSSValidity

- (instancetype)initWithFields:(KSSFields *)fields {
    if ((self = [super init])) {
        _stateCode = [fields optionalString:@"state_code"];
        _state = [fields enumeration:@"state" values:KSSValidityStateValues()];
        _taxpayerStateCode = [fields optionalString:@"taxpayer_state_code"];
        _domicileConditionCode = [fields optionalString:@"domicile_condition_code"];
        _observations = [fields strings:@"observations"];
        _message = [fields optionalString:@"message"];
    }
    return self;
}

@end

@implementation KSSQueryError

- (instancetype)initWithFields:(KSSFields *)fields {
    if ((self = [super init])) {
        _httpStatus = [fields integer:@"http_status"];
        _errorCode = [fields optionalString:@"error_code"];
        _message = [fields string:@"message"];
    }
    return self;
}

@end

@implementation KSSValidityResult

- (instancetype)initWithOutcome:(KSSOutcome)outcome fields:(KSSFields *)fields {
    if ((self = [super init])) {
        _outcome = outcome;
        _rawJSON = @"";
        if (outcome == KSSOutcomeValidity) {
            _validity = [[KSSValidity alloc] initWithFields:fields];
        } else if (outcome == KSSOutcomeQueryError) {
            _queryError = [[KSSQueryError alloc] initWithFields:fields];
        } else {
            _noVerdict = [[KSSNoVerdict alloc] initWithFields:fields];
        }
    }
    return self;
}

+ (instancetype)resultWithFields:(KSSFields *)fields {
    NSString *outcome = [fields string:@"outcome"];
    if ([outcome isEqualToString:@"validity"]) return [[self alloc] initWithOutcome:KSSOutcomeValidity fields:fields];
    if ([outcome isEqualToString:@"query_error"]) return [[self alloc] initWithOutcome:KSSOutcomeQueryError fields:fields];
    if ([outcome isEqualToString:@"no_verdict"]) return [[self alloc] initWithOutcome:KSSOutcomeNoVerdict fields:fields];
    KSSUnknownOutcome(fields, outcome);
    return nil;
}

@end

#pragma mark - GRE

@implementation KSSGREAPIErrorItem

- (instancetype)initWithFields:(KSSFields *)fields {
    if ((self = [super init])) {
        _code = [fields optionalString:@"code"];
        _itemDescription = [fields string:@"description"];
    }
    return self;
}

@end

@implementation KSSGREAPIError

- (instancetype)initWithFields:(KSSFields *)fields {
    if ((self = [super init])) {
        _stage = [fields enumeration:@"stage" values:KSSExchangeStageValues()];
        _httpStatus = [fields integer:@"http_status"];
        _generalError = [fields optionalString:@"general_error"];
        _code = [fields optionalString:@"code"];
        _message = [fields string:@"message"];
        NSMutableArray *errors = [NSMutableArray array];
        for (KSSFields *item in [fields objects:@"errors"]) [errors addObject:[[KSSGREAPIErrorItem alloc] initWithFields:item]];
        _errors = errors;
    }
    return self;
}

@end

@implementation KSSGRETicket

- (instancetype)initWithFields:(KSSFields *)fields {
    if ((self = [super init])) {
        _ticket = [fields string:@"ticket"];
        _receivedAt = [fields optionalString:@"received_at"];
        _zipName = [fields optionalString:@"zip_name"];
        _hashZip = [fields optionalString:@"hash_zip"];
    }
    return self;
}

@end

@implementation KSSGRESendResult

- (instancetype)initWithOutcome:(KSSOutcome)outcome fields:(KSSFields *)fields {
    if ((self = [super init])) {
        _outcome = outcome;
        _rawJSON = @"";
        if (outcome == KSSOutcomeTicket) {
            _greTicket = [[KSSGRETicket alloc] initWithFields:fields];
        } else if (outcome == KSSOutcomeAPIError) {
            _apiError = [[KSSGREAPIError alloc] initWithFields:fields];
        } else {
            _noVerdict = [[KSSNoVerdict alloc] initWithFields:fields];
        }
    }
    return self;
}

+ (instancetype)resultWithFields:(KSSFields *)fields {
    NSString *outcome = [fields string:@"outcome"];
    if ([outcome isEqualToString:@"ticket"]) return [[self alloc] initWithOutcome:KSSOutcomeTicket fields:fields];
    if ([outcome isEqualToString:@"api_error"]) return [[self alloc] initWithOutcome:KSSOutcomeAPIError fields:fields];
    if ([outcome isEqualToString:@"no_verdict"]) return [[self alloc] initWithOutcome:KSSOutcomeNoVerdict fields:fields];
    KSSUnknownOutcome(fields, outcome);
    return nil;
}

@end

@implementation KSSGRETicketError

- (instancetype)initWithFields:(KSSFields *)fields {
    if ((self = [super init])) {
        _code = [fields optionalString:@"code"];
        _errorDescription = [fields string:@"description"];
        _responseClass = [fields enumeration:@"class" values:KSSResponseClassValues()];
    }
    return self;
}

@end

@implementation KSSGRETicketStatus

- (instancetype)initWithFields:(KSSFields *)fields {
    if ((self = [super init])) {
        _statusCode = [fields string:@"status_code"];
        _state = [fields enumeration:@"state" values:KSSTicketStateValues()];
        _cdrGenerated = [fields boolean:@"cdr_generated"];
        KSSFields *error = [fields optionalObject:@"error"];
        _error = error == nil ? nil : [[KSSGRETicketError alloc] initWithFields:error];
        _cdr = KSSOptionalCdr(fields);
        _cdrZip = [fields optionalBase64:@"cdr_zip_base64"];
    }
    return self;
}

@end

@implementation KSSGRETicketResult

- (instancetype)initWithOutcome:(KSSOutcome)outcome fields:(KSSFields *)fields {
    if ((self = [super init])) {
        _outcome = outcome;
        _rawJSON = @"";
        _ticket = [fields string:@"ticket"];
        if (outcome == KSSOutcomeTicketStatus) {
            _ticketStatus = [[KSSGRETicketStatus alloc] initWithFields:fields];
        } else if (outcome == KSSOutcomeAPIError) {
            _apiError = [[KSSGREAPIError alloc] initWithFields:fields];
        } else {
            _noVerdict = [[KSSNoVerdict alloc] initWithFields:fields];
        }
    }
    return self;
}

+ (instancetype)resultWithFields:(KSSFields *)fields {
    NSString *outcome = [fields string:@"outcome"];
    if ([outcome isEqualToString:@"ticket_status"]) return [[self alloc] initWithOutcome:KSSOutcomeTicketStatus fields:fields];
    if ([outcome isEqualToString:@"api_error"]) return [[self alloc] initWithOutcome:KSSOutcomeAPIError fields:fields];
    if ([outcome isEqualToString:@"no_verdict"]) return [[self alloc] initWithOutcome:KSSOutcomeNoVerdict fields:fields];
    KSSUnknownOutcome(fields, outcome);
    return nil;
}

@end
