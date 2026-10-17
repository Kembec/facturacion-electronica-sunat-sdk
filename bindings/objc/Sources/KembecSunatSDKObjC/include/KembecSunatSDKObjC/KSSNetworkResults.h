#import <Foundation/Foundation.h>

#import <KembecSunatSDKObjC/KSSResults.h>

NS_ASSUME_NONNULL_BEGIN

// The results of the network commands. Each has its outcome and one property per outcome: the one that matches
// `outcome` is set, the others are nil. rawJSON is the response as the library gave it.

/// What sendXML: got: the CDR (KSSOutcomeCDR) or a SOAP fault (KSSOutcomeFault). A send with no answer is not here: it
/// is a KSSErrorCommandFailed error, since it may have arrived (send the same XML again, never another number).
@interface KSSSendResult : NSObject
@property (nonatomic, readonly) KSSOutcome outcome;
@property (nonatomic, readonly, nullable) KSSCdr *cdr;
/// The CDR as it came, to store; with KSSOutcomeCDR when SUNAT attached it.
@property (nonatomic, copy, readonly, nullable) NSData *cdrZip;
@property (nonatomic, readonly, nullable) KSSFault *fault;
/// The input field behind each code SUNAT answered; empty when the response carries none.
@property (nonatomic, copy, readonly) NSArray<KSSDiagnosis *> *diagnosis;
@property (nonatomic, copy, readonly) NSString *rawJSON;
- (instancetype)init NS_UNAVAILABLE;
@end

/// What sendSummaryXML: got: the ticket (KSSOutcomeTicket; store it before anything else) or a SOAP fault.
@interface KSSSummaryResult : NSObject
@property (nonatomic, readonly) KSSOutcome outcome;
@property (nonatomic, copy, readonly, nullable) NSString *ticket;
@property (nonatomic, readonly, nullable) KSSFault *fault;
/// The input field behind each code SUNAT answered; empty when the response carries none.
@property (nonatomic, copy, readonly) NSArray<KSSDiagnosis *> *diagnosis;
@property (nonatomic, copy, readonly) NSString *rawJSON;
- (instancetype)init NS_UNAVAILABLE;
@end

/// The state of a summary's or a void's ticket (getStatus).
@interface KSSTicketStatus : NSObject
/// 0, 98, 99
@property (nonatomic, copy, readonly) NSString *statusCode;
@property (nonatomic, readonly) KSSTicketState state;
@property (nonatomic, readonly, nullable) KSSCdr *cdr;
@property (nonatomic, copy, readonly, nullable) NSData *cdrZip;
- (instancetype)init NS_UNAVAILABLE;
@end

/// What ticketStatus: got: the ticket's state (KSSOutcomeTicketStatus), a SOAP fault, or no verdict (ask again; never
/// send the summary again).
@interface KSSTicketResult : NSObject
@property (nonatomic, readonly) KSSOutcome outcome;
/// The ticket asked for, when the response repeats it (no verdict).
@property (nonatomic, copy, readonly, nullable) NSString *ticket;
@property (nonatomic, readonly, nullable) KSSTicketStatus *ticketStatus;
@property (nonatomic, readonly, nullable) KSSFault *fault;
@property (nonatomic, readonly, nullable) KSSNoVerdict *noVerdict;
/// The input field behind each code SUNAT answered; empty when the response carries none.
@property (nonatomic, copy, readonly) NSArray<KSSDiagnosis *> *diagnosis;
@property (nonatomic, copy, readonly) NSString *rawJSON;
- (instancetype)init NS_UNAVAILABLE;
@end

/// The status of one voucher (getStatusCdr), with its CDR when SUNAT attaches it.
@interface KSSVoucherStatus : NSObject
@property (nonatomic, copy, readonly) NSString *statusCode;
@property (nonatomic, copy, readonly, nullable) NSString *statusMessage;
@property (nonatomic, readonly, nullable) KSSCdr *cdr;
@property (nonatomic, copy, readonly, nullable) NSData *cdrZip;
- (instancetype)init NS_UNAVAILABLE;
@end

/// What statusForRUC: got: the voucher's status (KSSOutcomeStatus), a SOAP fault, or no verdict (ask again).
@interface KSSStatusResult : NSObject
@property (nonatomic, readonly) KSSOutcome outcome;
@property (nonatomic, readonly, nullable) KSSVoucherStatus *voucherStatus;
@property (nonatomic, readonly, nullable) KSSFault *fault;
@property (nonatomic, readonly, nullable) KSSNoVerdict *noVerdict;
/// The input field behind each code SUNAT answered; empty when the response carries none.
@property (nonatomic, copy, readonly) NSArray<KSSDiagnosis *> *diagnosis;
@property (nonatomic, copy, readonly) NSString *rawJSON;
- (instancetype)init NS_UNAVAILABLE;
@end

/// A voucher's state in the Consulta Integrada.
typedef NS_ENUM(NSInteger, KSSValidityState) {
    KSSValidityStateUnknown,
    /// 0. It does not prove the voucher never arrived: reconcile with status or the summary's ticket first.
    KSSValidityStateNotReported,
    /// 1
    KSSValidityStateAccepted,
    /// 2: the number is spent.
    KSSValidityStateVoided,
    /// 3
    KSSValidityStatePrintedAuthorized,
    /// 4
    KSSValidityStatePrintedNotAuthorized,
};

/// What the Consulta Integrada says of a voucher.
@interface KSSValidity : NSObject
@property (nonatomic, copy, readonly, nullable) NSString *stateCode;
@property (nonatomic, readonly) KSSValidityState state;
@property (nonatomic, copy, readonly, nullable) NSString *taxpayerStateCode;
@property (nonatomic, copy, readonly, nullable) NSString *domicileConditionCode;
/// Where the data sent differs from SUNAT's (an accepted voucher with another issue date, for instance).
@property (nonatomic, copy, readonly) NSArray<NSString *> *observations;
@property (nonatomic, copy, readonly, nullable) NSString *message;
- (instancetype)init NS_UNAVAILABLE;
@end

/// SUNAT refused the query (not the voucher): success false with HTTP 200, 400 or 422.
@interface KSSQueryError : NSObject
@property (nonatomic, readonly) NSInteger httpStatus;
/// 99 is a malformed query.
@property (nonatomic, copy, readonly, nullable) NSString *errorCode;
@property (nonatomic, copy, readonly) NSString *message;
- (instancetype)init NS_UNAVAILABLE;
@end

/// What validityForRUC: got: the voucher's state (KSSOutcomeValidity), a refused query (KSSOutcomeQueryError), or no
/// verdict (ask again).
@interface KSSValidityResult : NSObject
@property (nonatomic, readonly) KSSOutcome outcome;
@property (nonatomic, readonly, nullable) KSSValidity *validity;
@property (nonatomic, readonly, nullable) KSSQueryError *queryError;
@property (nonatomic, readonly, nullable) KSSNoVerdict *noVerdict;
@property (nonatomic, copy, readonly) NSString *rawJSON;
- (instancetype)init NS_UNAVAILABLE;
@end

/// One validation error of the GRE API.
@interface KSSGREAPIErrorItem : NSObject
@property (nonatomic, copy, readonly, nullable) NSString *code;
@property (nonatomic, copy, readonly) NSString *itemDescription;
- (instancetype)init NS_UNAVAILABLE;
@end

/// SUNAT answered the GRE API's error body.
@interface KSSGREAPIError : NSObject
@property (nonatomic, readonly) KSSExchangeStage stage;
@property (nonatomic, readonly) NSInteger httpStatus;
/// Annex I's status: unauthorized, unprocessable_entity...
@property (nonatomic, copy, readonly, nullable) NSString *generalError;
@property (nonatomic, copy, readonly, nullable) NSString *code;
@property (nonatomic, copy, readonly) NSString *message;
/// What SUNAT rejected, in its words (422).
@property (nonatomic, copy, readonly) NSArray<KSSGREAPIErrorItem *> *errors;
- (instancetype)init NS_UNAVAILABLE;
@end

/// The ticket of a sent guide: store it before anything else, it is the only handle on the outcome.
@interface KSSGRETicket : NSObject
@property (nonatomic, copy, readonly) NSString *ticket;
@property (nonatomic, copy, readonly, nullable) NSString *receivedAt;
@property (nonatomic, copy, readonly, nullable) NSString *zipName;
/// SHA-256 of the zip sent, lowercase hex.
@property (nonatomic, copy, readonly, nullable) NSString *hashZip;
- (instancetype)init NS_UNAVAILABLE;
@end

/// What greSendXML: got: the ticket (KSSOutcomeTicket), an error of the API (KSSOutcomeAPIError), or no verdict on the
/// token (nothing was sent). A send with no answer is a KSSErrorCommandFailed error: send the same file again.
@interface KSSGRESendResult : NSObject
@property (nonatomic, readonly) KSSOutcome outcome;
@property (nonatomic, readonly, nullable) KSSGRETicket *greTicket;
@property (nonatomic, readonly, nullable) KSSGREAPIError *apiError;
@property (nonatomic, readonly, nullable) KSSNoVerdict *noVerdict;
@property (nonatomic, copy, readonly) NSString *rawJSON;
- (instancetype)init NS_UNAVAILABLE;
@end

/// The error of a guide processed with errors (99): SUNAT's numError and desError.
@interface KSSGRETicketError : NSObject
@property (nonatomic, copy, readonly, nullable) NSString *code;
@property (nonatomic, copy, readonly) NSString *errorDescription;
@property (nonatomic, readonly) KSSResponseClass responseClass;
- (instancetype)init NS_UNAVAILABLE;
@end

/// The state of a sent guide.
@interface KSSGRETicketStatus : NSObject
@property (nonatomic, copy, readonly) NSString *statusCode;
@property (nonatomic, readonly) KSSTicketState state;
@property (nonatomic, readonly) BOOL cdrGenerated;
@property (nonatomic, readonly, nullable) KSSGRETicketError *error;
@property (nonatomic, readonly, nullable) KSSCdr *cdr;
@property (nonatomic, copy, readonly, nullable) NSData *cdrZip;
- (instancetype)init NS_UNAVAILABLE;
@end

/// What greTicket: got: the guide's state (KSSOutcomeTicketStatus), an error of the API, or no verdict (ask again with
/// the same ticket).
@interface KSSGRETicketResult : NSObject
@property (nonatomic, readonly) KSSOutcome outcome;
/// The ticket asked for.
@property (nonatomic, copy, readonly) NSString *ticket;
@property (nonatomic, readonly, nullable) KSSGRETicketStatus *ticketStatus;
@property (nonatomic, readonly, nullable) KSSGREAPIError *apiError;
@property (nonatomic, readonly, nullable) KSSNoVerdict *noVerdict;
@property (nonatomic, copy, readonly) NSString *rawJSON;
- (instancetype)init NS_UNAVAILABLE;
@end

NS_ASSUME_NONNULL_END
