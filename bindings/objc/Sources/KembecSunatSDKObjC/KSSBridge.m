#import "KSSBridge.h"

#import <SunatSDKFFI/sunat_sdk.h>

#import <KembecSunatSDKObjC/KSSClient.h>
#import <KembecSunatSDKObjC/KSSErrors.h>

#if !__has_feature(objc_arc)
#error "KembecSunatSDKObjC is written for ARC (-fobjc-arc)"
#endif

NSError *KSSMakeError(NSInteger code, NSString *command, NSString *message) {
    NSMutableDictionary *info = [NSMutableDictionary dictionaryWithObject:message forKey:NSLocalizedDescriptionKey];
    if (command != nil) info[KSSErrorCommandKey] = command;
    return [NSError errorWithDomain:KSSErrorDomain code:code userInfo:info];
}

static void KSSSetError(NSError **error, NSError *value) {
    if (error != NULL) *error = value;
}

#pragma mark - Request

@implementation KSSRequest {
    NSMutableData *_bytes;
}

- (instancetype)init {
    if ((self = [super init])) _bytes = [NSMutableData dataWithCapacity:4096];
    return self;
}

- (void)raw:(NSString *)text {
    const char *utf8 = text.UTF8String;
    [_bytes appendBytes:utf8 length:strlen(utf8)];
}

- (void)string:(NSString *)text {
    static const char hex[] = "0123456789abcdef";
    NSData *utf8 = [text dataUsingEncoding:NSUTF8StringEncoding];
    const uint8_t *units = utf8.bytes;
    [_bytes appendBytes:"\"" length:1];
    for (NSUInteger i = 0; i < utf8.length; i++) {
        uint8_t unit = units[i];
        if (unit == '"' || unit == '\\') {
            uint8_t escaped[2] = {'\\', unit};
            [_bytes appendBytes:escaped length:2];
        } else if (unit < 0x20) {
            uint8_t escaped[6] = {'\\', 'u', '0', '0', (uint8_t)hex[unit >> 4], (uint8_t)hex[unit & 0xF]};
            [_bytes appendBytes:escaped length:6];
        } else {
            [_bytes appendBytes:&unit length:1];
        }
    }
    [_bytes appendBytes:"\"" length:1];
}

- (void)object:(NSArray<NSArray *> *)members {
    [self raw:@"{"];
    [members enumerateObjectsUsingBlock:^(NSArray *member, NSUInteger index, BOOL *stop) {
        if (index > 0) [self raw:@","];
        [self string:member[0]];
        [self raw:@":"];
        id value = member[1];
        if ([value isKindOfClass:[NSNumber class]]) {
            [self raw:[NSString stringWithFormat:@"%lld", [(NSNumber *)value longLongValue]]];
        } else {
            [self string:value];
        }
    }];
    [self raw:@"}"];
}

/// Calls the library with the request, then overwrites the request's bytes.
- (int32_t)callWithResponse:(char **)response {
    [_bytes appendBytes:"\0" length:1];
    int32_t status = sunat_sdk_call(_bytes.bytes, response);
    memset(_bytes.mutableBytes, 0, _bytes.length);
    return status;
}

@end

#pragma mark - Fields

@implementation KSSFields {
    NSString *_path;
    NSMutableArray<NSString *> *_problems;
}

- (instancetype)initWithDictionary:(NSDictionary *)dictionary path:(NSString *)path problems:(NSMutableArray *)problems {
    if ((self = [super init])) {
        _dictionary = dictionary;
        _path = path;
        _problems = problems;
    }
    return self;
}

- (NSString *)pathOf:(NSString *)key {
    return _path.length > 0 ? [NSString stringWithFormat:@"%@.%@", _path, key] : key;
}

- (void)reject:(NSString *)key because:(NSString *)reason {
    [_problems addObject:[NSString stringWithFormat:@"`%@`: %@", [self pathOf:key], reason]];
}

/// The member's value; nil when absent or null. A value of another class is a problem and reads as nil.
- (nullable id)value:(NSString *)key ofClass:(Class)type required:(BOOL)required {
    id value = _dictionary[key];
    if (value == nil || value == [NSNull null]) {
        if (required) [_problems addObject:[NSString stringWithFormat:@"`%@` is missing", [self pathOf:key]]];
        return nil;
    }
    if (![value isKindOfClass:type]) {
        [self reject:key because:[NSString stringWithFormat:@"expected %@", NSStringFromClass(type)]];
        return nil;
    }
    return value;
}

- (nullable NSNumber *)wholeNumber:(NSString *)key required:(BOOL)required {
    NSNumber *number = [self value:key ofClass:[NSNumber class] required:required];
    if (number == nil) return nil;
    if (CFGetTypeID((__bridge CFTypeRef)number) == CFBooleanGetTypeID() || number.doubleValue != floor(number.doubleValue)) {
        [self reject:key because:@"expected a whole number"];
        return nil;
    }
    return number;
}

- (NSString *)string:(NSString *)key {
    return [self value:key ofClass:[NSString class] required:YES] ?: @"";
}

- (NSString *)optionalString:(NSString *)key {
    return [self value:key ofClass:[NSString class] required:NO];
}

- (NSInteger)integer:(NSString *)key {
    return [self wholeNumber:key required:YES].integerValue;
}

- (NSNumber *)optionalInteger:(NSString *)key {
    return [self wholeNumber:key required:NO];
}

- (BOOL)boolean:(NSString *)key {
    NSNumber *value = [self value:key ofClass:[NSNumber class] required:YES];
    if (value != nil && CFGetTypeID((__bridge CFTypeRef)value) != CFBooleanGetTypeID()) {
        [self reject:key because:@"expected true or false"];
        return NO;
    }
    return value.boolValue;
}

- (NSArray<NSString *> *)strings:(NSString *)key {
    NSArray *array = [self value:key ofClass:[NSArray class] required:NO];
    for (id item in array) {
        if (![item isKindOfClass:[NSString class]]) {
            [self reject:key because:@"expected an array of strings"];
            return @[];
        }
    }
    return array ?: @[];
}

- (nullable NSData *)decode:(nullable NSString *)text key:(NSString *)key {
    if (text == nil) return nil;
    NSData *data = [[NSData alloc] initWithBase64EncodedString:text options:0];
    if (data == nil) [self reject:key because:@"not base64"];
    return data;
}

- (NSData *)base64:(NSString *)key {
    return [self decode:[self value:key ofClass:[NSString class] required:YES] key:key] ?: [NSData data];
}

- (NSData *)optionalBase64:(NSString *)key {
    return [self decode:[self optionalString:key] key:key];
}

- (KSSFields *)object:(NSString *)key {
    NSDictionary *object = [self value:key ofClass:[NSDictionary class] required:YES];
    return [[KSSFields alloc] initWithDictionary:object ?: @{} path:[self pathOf:key] problems:_problems];
}

- (KSSFields *)optionalObject:(NSString *)key {
    NSDictionary *object = [self value:key ofClass:[NSDictionary class] required:NO];
    if (object == nil) return nil;
    return [[KSSFields alloc] initWithDictionary:object path:[self pathOf:key] problems:_problems];
}

- (NSArray<KSSFields *> *)objects:(NSString *)key {
    NSArray *array = [self value:key ofClass:[NSArray class] required:NO];
    NSMutableArray *objects = [NSMutableArray arrayWithCapacity:array.count];
    for (NSUInteger i = 0; i < array.count; i++) {
        NSString *path = [NSString stringWithFormat:@"%@.%lu", [self pathOf:key], (unsigned long)i];
        if (![array[i] isKindOfClass:[NSDictionary class]]) {
            [_problems addObject:[NSString stringWithFormat:@"`%@`: expected an object", path]];
            continue;
        }
        [objects addObject:[[KSSFields alloc] initWithDictionary:array[i] path:path problems:_problems]];
    }
    return objects;
}

- (NSInteger)enumeration:(NSString *)key values:(NSArray<NSString *> *)values {
    NSString *value = [self optionalString:key];
    NSUInteger index = value == nil ? NSNotFound : [values indexOfObject:value];
    return index == NSNotFound ? 0 : (NSInteger)index + 1;
}

- (NSString *)JSONText {
    NSData *data = [NSJSONSerialization dataWithJSONObject:_dictionary options:NSJSONWritingSortedKeys error:NULL];
    return (data == nil ? nil : [[NSString alloc] initWithData:data encoding:NSUTF8StringEncoding]) ?: @"{}";
}

@end

#pragma mark - Call

NSString *KSSLoadedVersion(NSError **error) {
    static NSString *version;
    static NSError *failure;
    static dispatch_once_t once;
    dispatch_once(&once, ^{
        uint32_t abi = sunat_sdk_abi_version();
        if (abi != SUNAT_SDK_ABI_VERSION) {
            failure = KSSMakeError(KSSErrorLoadFailed, nil,
                                   [NSString stringWithFormat:@"the native library has ABI %u; this package needs %u", abi,
                                                              (unsigned)SUNAT_SDK_ABI_VERSION]);
            return;
        }
        const char *native = sunat_sdk_version();
        NSString *text = native == NULL ? nil : [NSString stringWithUTF8String:native];
        if (text == nil) {
            failure = KSSMakeError(KSSErrorLoadFailed, nil, @"the native library gives no version");
        } else if (![text isEqualToString:KSSPackageVersion]) {
            failure = KSSMakeError(KSSErrorLoadFailed, nil,
                                   [NSString stringWithFormat:@"the native library is %@; this package is %@", text, KSSPackageVersion]);
        } else {
            version = text;
        }
    });
    if (version == nil) KSSSetError(error, failure);
    return version;
}

id KSSRun(NSString *command, NSArray<NSArray *> *flags, void (^input)(KSSRequest *), KSSParse parse, NSString **rawJSON,
          NSError **error) {
    if (KSSLoadedVersion(error) == nil) return nil;
    KSSRequest *request = [[KSSRequest alloc] init];
    [request raw:@"{\"command\":"];
    [request string:command];
    [request raw:@",\"flags\":"];
    [request object:flags];
    [request raw:@",\"input\":"];
    input(request);
    [request raw:@"}"];
    char *response = NULL;
    int32_t status = [request callWithResponse:&response];
    NSString *text = response == NULL ? nil : [NSString stringWithUTF8String:response];
    sunat_sdk_free(response);
    if (text == nil) {
        KSSSetError(error, KSSMakeError(KSSErrorUnexpectedResponse, command,
                                        [NSString stringWithFormat:@"no response, or one that is not UTF-8 (status %d)", status]));
        return nil;
    }
    id object = [NSJSONSerialization JSONObjectWithData:[text dataUsingEncoding:NSUTF8StringEncoding] options:0 error:NULL];
    NSDictionary *dictionary = [object isKindOfClass:[NSDictionary class]] ? object : nil;
    if (status == SUNAT_SDK_ERROR) {
        id message = dictionary[@"error"];
        if (![message isKindOfClass:[NSString class]]) {
            KSSSetError(error, KSSMakeError(KSSErrorUnexpectedResponse, command, @"status 1 without {\"error\"}"));
            return nil;
        }
        KSSSetError(error, KSSMakeError(KSSErrorCommandFailed, command, message));
        return nil;
    }
    if (status != SUNAT_SDK_OK || dictionary == nil) {
        NSString *why = status != SUNAT_SDK_OK ? [NSString stringWithFormat:@"status %d", status] : @"not a JSON object";
        KSSSetError(error, KSSMakeError(KSSErrorUnexpectedResponse, command, why));
        return nil;
    }
    NSMutableArray *problems = [NSMutableArray array];
    KSSFields *fields = [[KSSFields alloc] initWithDictionary:dictionary path:@"" problems:problems];
    id result = parse(fields);
    if (problems.count > 0 || result == nil) {
        KSSSetError(error, KSSMakeError(KSSErrorUnexpectedResponse, command, problems.firstObject ?: @"unreadable response"));
        return nil;
    }
    if (rawJSON != NULL) *rawJSON = text;
    return result;
}
