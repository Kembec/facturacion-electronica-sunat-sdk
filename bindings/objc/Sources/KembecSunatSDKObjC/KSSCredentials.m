#import <KembecSunatSDKObjC/KSSCredentials.h>

// A nil that slips through a nonnull parameter is kept as empty text: the SDK refuses it with its own message (an empty
// path, a missing user) instead of the request crashing on it.

@implementation KSSSigningKey

- (instancetype)initWithCertificatePath:(NSString *)certificatePath privateKeyPath:(NSString *)privateKeyPath {
    if ((self = [super init])) {
        _certificatePath = [certificatePath copy] ?: @"";
        _privateKeyPath = [privateKeyPath copy] ?: @"";
    }
    return self;
}

@end

@implementation KSSSOLCredentials

- (instancetype)initWithUser:(NSString *)user password:(NSString *)password {
    if ((self = [super init])) {
        _user = [user copy] ?: @"";
        _password = [password copy] ?: @"";
    }
    return self;
}

- (NSString *)description {
    return [NSString stringWithFormat:@"<KSSSOLCredentials user: %@, password: ***>", self.user];
}

- (NSString *)debugDescription {
    return self.description;
}

@end

@implementation KSSAPICredentials

- (instancetype)initWithClientID:(NSString *)clientID clientSecret:(NSString *)clientSecret {
    if ((self = [super init])) {
        _clientID = [clientID copy] ?: @"";
        _clientSecret = [clientSecret copy] ?: @"";
    }
    return self;
}

- (NSString *)description {
    return [NSString stringWithFormat:@"<KSSAPICredentials clientID: %@, clientSecret: ***>", self.clientID];
}

- (NSString *)debugDescription {
    return self.description;
}

@end
