#import <Foundation/Foundation.h>

#import <KembecSunatSDKObjC/KSSCredentials.h>
#import <KembecSunatSDKObjC/KSSNetworkResults.h>
#import <KembecSunatSDKObjC/KSSResults.h>

NS_ASSUME_NONNULL_BEGIN

/// The version of this package, equal to the native library's.
FOUNDATION_EXPORT NSString *const KSSPackageVersion;

/// The commands of the SDK's contract (docs/cli.md) in process, through its C ABI (docs/c-abi.md): the same validation,
/// XML, signature, network calls and outcomes as the CLI and every other language.
///
/// Without state after init, safe to share between threads. Each method blocks until its command ends, the network ones
/// for up to httpTimeout: call them off the main thread. A method returns nil and sets *error (KSSErrorDomain) when the
/// command gives no result; a SOAP fault, a refused query or no verdict are results.
@interface KSSClient : NSObject

/// 90 seconds, the CLI's default.
@property (class, nonatomic, readonly) NSTimeInterval defaultHTTPTimeout;
/// How long a network command waits on SUNAT: the token and the call of the REST commands share it.
@property (nonatomic, readonly) NSTimeInterval httpTimeout;

/// A client with the default timeout.
- (instancetype)init;
/// A client that waits on SUNAT up to httpTimeout: whole seconds from 1 to 300, or nil with KSSErrorInvalidArgument.
- (nullable instancetype)initWithHTTPTimeout:(NSTimeInterval)httpTimeout error:(NSError **)error NS_DESIGNATED_INITIALIZER;

/// The version of the native library linked, which is the SDK's; nil with KSSErrorLoadFailed if it is not the one this
/// package was built for.
+ (nullable NSString *)nativeVersionWithError:(NSError **)error;

/// Renders the unsigned UBL XML of one voucher, daily summary, void communication or despatch advice from its JSON, in the
/// shape of docs/input-model.md. KSSErrorInvalidArgument if the text is not one JSON value; KSSErrorCommandFailed if the
/// SDK rejected the document (the message says which field and why).
- (nullable KSSBuildResult *)buildDocumentJSON:(NSString *)documentJSON error:(NSError **)error;

/// Signs XML from buildDocumentJSON: with XML-DSig (enveloped, RSA-SHA1, as SUNAT requires). signatureID is the
/// signature's Id; signatureKG when nil.
- (nullable KSSSignResult *)signXML:(NSString *)unsignedXML
                                key:(KSSSigningKey *)key
                        signatureID:(nullable NSString *)signatureID
                              error:(NSError **)error;

/// Zips the XML under <name>.xml, as SUNAT expects it. name is the file name without extension (20100066603-01-F001-1).
- (nullable KSSZipResult *)zipXML:(NSString *)xml name:(NSString *)name error:(NSError **)error;

/// Reads a CDR, zip or XML, and classifies its response code.
- (nullable KSSCdr *)readCDR:(NSData *)cdr error:(NSError **)error;

/// Zips the signed XML and sends it with sendBill to endpoint (the billService URL, https only). A fault is a result; a
/// send without an answer is KSSErrorCommandFailed, since it may have arrived: send the same XML again, never another
/// number.
- (nullable KSSSendResult *)sendXML:(NSString *)signedXML
                               name:(NSString *)name
                        credentials:(KSSSOLCredentials *)credentials
                           endpoint:(NSURL *)endpoint
                              error:(NSError **)error;

/// Zips a signed daily summary (RC) or void communication (RA) and sends it with sendSummary. name is
/// RUC-RC|RA-YYYYMMDD-N. Store the ticket before anything else. A send without an answer fails, as sendXML:.
- (nullable KSSSummaryResult *)sendSummaryXML:(NSString *)signedXML
                                         name:(NSString *)name
                                  credentials:(KSSSOLCredentials *)credentials
                                     endpoint:(NSURL *)endpoint
                                        error:(NSError **)error;

/// Asks getStatus for a summary's or a void's ticket. No answer is KSSOutcomeNoVerdict: ask again, never send again.
- (nullable KSSTicketResult *)ticketStatus:(NSString *)ticket
                               credentials:(KSSSOLCredentials *)credentials
                                  endpoint:(NSURL *)endpoint
                                     error:(NSError **)error;

/// Asks getStatusCdr for one voucher (documentType 01, 03, 07 or 08), with its CDR when SUNAT attaches it. endpoint is
/// the billConsultService URL, https only. No answer is KSSOutcomeNoVerdict.
- (nullable KSSStatusResult *)statusForRUC:(NSString *)ruc
                              documentType:(NSString *)documentType
                                    series:(NSString *)series
                                    number:(int64_t)number
                               credentials:(KSSSOLCredentials *)credentials
                                  endpoint:(NSURL *)endpoint
                                     error:(NSError **)error;

/// Asks SUNAT's Consulta Integrada whether a voucher (01, 03, 07, 08) exists and in which state: a token, then the query,
/// both inside httpTimeout. There is no test environment: this is production.
/// issueDate needs a year, a month and a day (the rest is ignored); total is the voucher's PayableAmount, up to 2
/// decimals; credentials, the Consulta Integrada pair of «Credenciales de API SUNAT»; requesterRUC, the RUC that asks
/// (the issuer's when nil); tokenEndpoint and endpoint replace the production bases (https only) when not nil.
- (nullable KSSValidityResult *)validityForRUC:(NSString *)ruc
                                  documentType:(NSString *)documentType
                                        series:(NSString *)series
                                        number:(int64_t)number
                                     issueDate:(NSDateComponents *)issueDate
                                         total:(NSDecimalNumber *)total
                                   credentials:(KSSAPICredentials *)credentials
                                  requesterRUC:(nullable NSString *)requesterRUC
                                 tokenEndpoint:(nullable NSURL *)tokenEndpoint
                                      endpoint:(nullable NSURL *)endpoint
                                         error:(NSError **)error;

/// Sends a signed despatch advice (09) by SUNAT's GRE REST API: a token by password, then the zip with its hash. name is
/// RUC-09-SERIE-NUMERO; apiCredentials, the GRE pair of «Credenciales de API SUNAT»; solCredentials, the RUC followed by
/// the SOL user, and its password. SUNAT has no test environment for GRE and the endpoints have no default:
/// tokenEndpoint https://api-seguridad.sunat.gob.pe/v1/clientessol, endpoint https://api-cpe.sunat.gob.pe/v1. A send
/// without an answer is KSSErrorCommandFailed: send the same file again, never the operation with another number.
- (nullable KSSGRESendResult *)greSendXML:(NSString *)signedXML
                                     name:(NSString *)name
                           apiCredentials:(KSSAPICredentials *)apiCredentials
                           solCredentials:(KSSSOLCredentials *)solCredentials
                            tokenEndpoint:(NSURL *)tokenEndpoint
                                 endpoint:(NSURL *)endpoint
                                    error:(NSError **)error;

/// Asks for the outcome of a sent guide by its ticket (a token, then the query). 98 is in process: ask again with the
/// same ticket.
- (nullable KSSGRETicketResult *)greTicket:(NSString *)ticket
                            apiCredentials:(KSSAPICredentials *)apiCredentials
                            solCredentials:(KSSSOLCredentials *)solCredentials
                             tokenEndpoint:(NSURL *)tokenEndpoint
                                  endpoint:(NSURL *)endpoint
                                     error:(NSError **)error;

@end

NS_ASSUME_NONNULL_END
