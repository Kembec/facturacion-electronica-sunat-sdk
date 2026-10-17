#import <Foundation/Foundation.h>

NS_ASSUME_NONNULL_BEGIN

/// The domain of every error of this package.
FOUNDATION_EXPORT NSErrorDomain const KSSErrorDomain;
/// userInfo key: the command that failed (`build`, `send`...), absent for errors raised before any call.
FOUNDATION_EXPORT NSString *const KSSErrorCommandKey;

/// Why a command gave no result. A SOAP fault, a refused query or a missing answer to a query are results, not errors.
typedef NS_ERROR_ENUM(KSSErrorDomain, KSSErrorCode) {
    /// The command ran and failed: the SDK rejected the document (the message says which field and why), the
    /// certificate could not be read, the endpoint is not https://, a send got no answer (it may have arrived: send the
    /// same XML again, never another number). The CLI's {"error"} with exit code 1.
    KSSErrorCommandFailed = 1,
    /// An argument was refused before any call: nil, a document that is not one JSON value, a timeout out of range.
    KSSErrorInvalidArgument = 2,
    /// The native library is not the one this package was built for (another ABI or another version).
    KSSErrorLoadFailed = 3,
    /// The library answered something that is not the contract's shape for the command.
    KSSErrorUnexpectedResponse = 4,
};

NS_ASSUME_NONNULL_END
