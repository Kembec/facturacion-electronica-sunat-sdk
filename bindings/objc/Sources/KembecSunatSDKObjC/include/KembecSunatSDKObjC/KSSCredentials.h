#import <Foundation/Foundation.h>

NS_ASSUME_NONNULL_BEGIN

/// The certificate and its private key, both PEM files; the key unencrypted RSA. The SDK reads them on each sign.
@interface KSSSigningKey : NSObject
@property (nonatomic, copy, readonly) NSString *certificatePath;
@property (nonatomic, copy, readonly) NSString *privateKeyPath;
- (instancetype)initWithCertificatePath:(NSString *)certificatePath privateKeyPath:(NSString *)privateKeyPath NS_DESIGNATED_INITIALIZER;
- (instancetype)init NS_UNAVAILABLE;
@end

/// The SOL user (the RUC followed by the user, 20123456789MODDATOS) and its password, which never shows in -description.
@interface KSSSOLCredentials : NSObject
@property (nonatomic, copy, readonly) NSString *user;
@property (nonatomic, copy, readonly) NSString *password;
- (instancetype)initWithUser:(NSString *)user password:(NSString *)password NS_DESIGNATED_INITIALIZER;
- (instancetype)init NS_UNAVAILABLE;
@end

/// A pair of «Credenciales de API SUNAT» created in SOL: the Consulta Integrada pair for validity, the GRE pair for
/// greSend and greTicket. The secret never shows in -description.
@interface KSSAPICredentials : NSObject
@property (nonatomic, copy, readonly) NSString *clientID;
@property (nonatomic, copy, readonly) NSString *clientSecret;
- (instancetype)initWithClientID:(NSString *)clientID clientSecret:(NSString *)clientSecret NS_DESIGNATED_INITIALIZER;
- (instancetype)init NS_UNAVAILABLE;
@end

NS_ASSUME_NONNULL_END
