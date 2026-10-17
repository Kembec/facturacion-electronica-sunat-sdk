#import <Foundation/Foundation.h>

NS_ASSUME_NONNULL_BEGIN

// The results of the commands, read from the contract's JSON (docs/cli.md). Every top-level result keeps the response
// as the library gave it in rawJSON. A value the contract may add later reads as the enum's Unknown.

/// Which outcome of its command a result is.
typedef NS_ENUM(NSInteger, KSSOutcome) {
    KSSOutcomeCDR,
    KSSOutcomeFault,
    KSSOutcomeTicket,
    KSSOutcomeTicketStatus,
    KSSOutcomeStatus,
    KSSOutcomeNoVerdict,
    KSSOutcomeValidity,
    KSSOutcomeQueryError,
    KSSOutcomeAPIError,
};

/// How the SDK classifies a SUNAT code.
typedef NS_ENUM(NSInteger, KSSResponseClass) {
    KSSResponseClassUnknown,
    KSSResponseClassAccepted,
    KSSResponseClassObserved,
    KSSResponseClassException,
    KSSResponseClassAlreadyRegistered,
    KSSResponseClassNumberVoidedOrRejected,
    KSSResponseClassRejected,
    KSSResponseClassVoidAlreadyReported,
};

/// The state of a ticket: 0 processed, 98 in process (ask again with the same ticket, never send again), 99 with errors.
typedef NS_ENUM(NSInteger, KSSTicketState) {
    KSSTicketStateUnknown,
    KSSTicketStateProcessed,
    KSSTicketStateInProcess,
    KSSTicketStateProcessedWithErrors,
};

/// Why a query has no verdict.
typedef NS_ENUM(NSInteger, KSSNoVerdictReason) {
    KSSNoVerdictReasonUnknown,
    KSSNoVerdictReasonTimeout,
    KSSNoVerdictReasonHTTPStatus,
};

/// The exchange of a REST command that a result speaks of.
typedef NS_ENUM(NSInteger, KSSExchangeStage) {
    KSSExchangeStageUnknown,
    KSSExchangeStageToken,
    KSSExchangeStageQuery,
    KSSExchangeStageSend,
    KSSExchangeStageTicket,
};

/// The unsigned UBL XML of build, and the file name SUNAT expects (without extension).
@interface KSSBuildResult : NSObject
@property (nonatomic, copy, readonly) NSString *fileName;
@property (nonatomic, copy, readonly) NSString *xml;
@property (nonatomic, copy, readonly) NSString *rawJSON;
- (instancetype)init NS_UNAVAILABLE;
@end

/// The signed XML of sign, with the two values a signature is compared by.
@interface KSSSignResult : NSObject
@property (nonatomic, copy, readonly) NSString *xml;
@property (nonatomic, copy, readonly) NSString *digestValue;
@property (nonatomic, copy, readonly) NSString *signatureValue;
@property (nonatomic, copy, readonly) NSString *rawJSON;
- (instancetype)init NS_UNAVAILABLE;
@end

/// The zip of zip, with the XML inside under <name>.xml.
@interface KSSZipResult : NSObject
/// <name>.zip
@property (nonatomic, copy, readonly) NSString *fileName;
@property (nonatomic, copy, readonly) NSData *content;
@property (nonatomic, copy, readonly) NSString *rawJSON;
- (instancetype)init NS_UNAVAILABLE;
@end

/// A CDR (constancia de recepción): the code SUNAT answered and how the SDK classifies it.
@interface KSSCdr : NSObject
/// The voucher it answers: F001-1.
@property (nonatomic, copy, readonly) NSString *referenceID;
/// 0, 2047...
@property (nonatomic, copy, readonly) NSString *responseCode;
@property (nonatomic, copy, readonly) NSString *responseDescription;
/// Each "code - text"; with code 0, every note is an observation.
@property (nonatomic, copy, readonly) NSArray<NSString *> *notes;
@property (nonatomic, readonly) KSSResponseClass responseClass;
/// Code 0 without observations.
@property (nonatomic, readonly, getter=isAccepted) BOOL accepted;
/// Accepted with or without observations: the voucher is valid.
@property (nonatomic, readonly, getter=isValidDocument) BOOL validDocument;
/// The response of readCDR:; for a CDR inside another result, that member's JSON.
@property (nonatomic, copy, readonly) NSString *rawJSON;
- (instancetype)init NS_UNAVAILABLE;
@end

/// How sure the table is of a field. A word the contract may add later reads as Unknown.
typedef NS_ENUM(NSInteger, KSSDiagnosisCertainty) {
    KSSDiagnosisCertaintyUnknown,
    /// fuente: the node is the rule's own and one field writes it.
    KSSDiagnosisCertaintySource,
    /// inferido: the rule names more than the node, or the field is a reading of it.
    KSSDiagnosisCertaintyInferred,
};

/// What SUNAT's message names, as it writes it: (nodo: "cbc:ID/schemeID" valor: "1").
@interface KSSDiagnosisDetail : NSObject
/// The last element and, if any, its attribute: cbc:ID/schemeID.
@property (nonatomic, copy, readonly) NSString *node;
/// The value SUNAT read in that node.
@property (nonatomic, copy, readonly) NSString *value;
- (instancetype)init NS_UNAVAILABLE;
@end

/// One candidate field for a code, from SUNAT's validation rules of 26.08.2026.
@interface KSSDiagnosisField : NSObject
/// A field of the input model (customer.document_type, lines[].unit_price), sdk when the SDK writes the node itself, or
/// nil when the input model has no such field.
@property (nonatomic, copy, readonly, nullable) NSString *inputPath;
/// The node the rule checks; nil for rules on the file, the signature or the service.
@property (nonatomic, copy, readonly, nullable) NSString *xpath;
@property (nonatomic, readonly) KSSDiagnosisCertainty certainty;
- (instancetype)init NS_UNAVAILABLE;
@end

/// A code SUNAT answered, with the fields that may have caused it.
@interface KSSDiagnosis : NSObject
/// 2800, 4287...
@property (nonatomic, copy, readonly) NSString *code;
/// What the message names, when it names a node.
@property (nonatomic, readonly, nullable) KSSDiagnosisDetail *detail;
/// With a detail, the fields that write that node, unless none does; otherwise all of the code's. Empty when the code is
/// not in the table.
@property (nonatomic, copy, readonly) NSArray<KSSDiagnosisField *> *fields;
- (instancetype)init NS_UNAVAILABLE;
@end

/// A SOAP fault: SUNAT refused the call with a code.
@interface KSSFault : NSObject
/// 1033, 0111...
@property (nonatomic, copy, readonly) NSString *code;
@property (nonatomic, copy, readonly) NSString *message;
@property (nonatomic, readonly) KSSResponseClass responseClass;
- (instancetype)init NS_UNAVAILABLE;
@end

/// A query without a verdict: it says nothing about the document. Ask again, never send again.
@interface KSSNoVerdict : NSObject
@property (nonatomic, readonly) KSSNoVerdictReason reason;
/// The HTTP status SUNAT answered; nil when the wait ran out.
@property (nonatomic, readonly, nullable) NSNumber *httpStatus;
@property (nonatomic, copy, readonly) NSString *message;
/// The exchange without a verdict (validity, greSend, greTicket); Unknown for status and ticket.
@property (nonatomic, readonly) KSSExchangeStage stage;
- (instancetype)init NS_UNAVAILABLE;
@end

NS_ASSUME_NONNULL_END
